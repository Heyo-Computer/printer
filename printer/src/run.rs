use crate::agent::AgentInvocation;
use crate::cli::RunArgs;
use crate::hooks::{AgentContribution, Event, HookContext, HookSet};
use crate::prompts::{
    bootstrap_prompt, nudge_prompt_with, rotation_prompt, unstall_prompt, SENTINEL_BLOCKED,
    SENTINEL_DONE,
};
use crate::session::Session;
use crate::skills;
use crate::tasks::model::{Status, Task};
use crate::tasks::{spec, store};
use anyhow::{Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

pub async fn run(args: RunArgs) -> Result<()> {
    let hooks = HookSet::load_installed().unwrap_or_default();
    let spec_abs = args
        .spec
        .canonicalize()
        .with_context(|| format!("spec file not found: {}", args.spec.display()))?;
    if !spec_abs.is_file() {
        anyhow::bail!("spec must be a file: {}", spec_abs.display());
    }

    let cwd: PathBuf = match args.cwd.as_deref() {
        Some(p) => p
            .canonicalize()
            .with_context(|| format!("--cwd not found: {}", p.display()))?,
        None => std::env::current_dir()?,
    };
    let tasks_dir = cwd.join(".printer").join("tasks");
    std::fs::create_dir_all(&tasks_dir).with_context(|| {
        format!(
            "creating task store at {}",
            tasks_dir.display()
        )
    })?;

    hooks.run_cli(
        Event::BeforeRun,
        &HookContext::new(Event::BeforeRun, cwd.clone())
            .with_spec(spec_abs.clone()),
    )?;
    let agent_contrib = hooks.agent_for(Event::BeforeRun);
    let resolved_skills = resolve_agent_skills(&agent_contrib)?;
    if !agent_contrib.is_empty() {
        eprintln!(
            "[printer] before_run hooks: {} prompt chunk(s), {} skill(s)",
            agent_contrib.prompt_chunks.len(),
            resolved_skills.len()
        );
    }
    let injected_block = agent_contrib.render_prompt_block();

    let inner = run_inner(
        &args,
        &hooks,
        &spec_abs,
        &cwd,
        &tasks_dir,
        injected_block.as_deref(),
        &resolved_skills,
    )
    .await;

    let after_ctx = HookContext::new(Event::AfterRun, cwd.clone())
        .with_spec(spec_abs.clone())
        .with_exit_status(inner.is_ok());
    let _ = hooks.run_cli(Event::AfterRun, &after_ctx);

    inner
}

fn resolve_agent_skills(contrib: &AgentContribution) -> Result<Vec<skills::Skill>> {
    if contrib.skills.is_empty() {
        return Ok(Vec::new());
    }
    skills::resolve(&contrib.skills, None)
}

async fn run_inner(
    args: &RunArgs,
    _hooks: &HookSet,
    spec_abs: &std::path::Path,
    cwd: &std::path::Path,
    tasks_dir: &std::path::Path,
    injected_block: Option<&str>,
    injected_skills: &[skills::Skill],
) -> Result<()> {
    let printer_bin = std::env::current_exe()
        .context("resolving printer binary path for the agent prompt")?;
    let printer_bin_str = printer_bin.to_string_lossy().into_owned();

    let agent = AgentInvocation {
        kind: args.agent,
        model: args.model.as_deref(),
        cwd: Some(cwd),
        permission_mode: &args.permission_mode,
    };
    let mut session = Session::new(agent).with_verbose(args.verbose);

    // Initial sync: parse spec and reconcile with the task store.
    let report = sync_spec(spec_abs, tasks_dir)?;
    eprintln!(
        "[printer] spec sync: {} new, {} existing, {} closed-from-spec",
        report.created, report.existing, report.closed
    );

    // If the spec has no checklist items at all, ask the agent to write one
    // into the spec. Then re-sync. If still empty, bail.
    if !any_tasks(tasks_dir)? {
        eprintln!("[printer] spec has no checklist items; asking agent to bootstrap one into the spec");
        let outcome = session
            .turn(&bootstrap_prompt(&spec_abs.to_string_lossy()))
            .await?;
        print_result_tail(&outcome.result_text);
        if let Some(reason) = blocked_reason(&outcome.result_text) {
            anyhow::bail!("agent reported blocked during bootstrap: {reason}");
        }
        let report = sync_spec(spec_abs, tasks_dir)?;
        eprintln!(
            "[printer] post-bootstrap sync: {} new, {} existing",
            report.created, report.existing
        );
        if !any_tasks(tasks_dir)? {
            anyhow::bail!("agent did not write any checklist items into `{}`", spec_abs.display());
        }
    }

    // Execute loop.
    let mut tasks = store::list_all(tasks_dir)?;
    let mut prev_state_hash = state_hash(&tasks);
    let mut stalls: u32 = 0;

    if all_done(&tasks) {
        eprintln!("[printer] all tasks already done; nothing to do.");
        return Ok(());
    }

    for _ in 0..args.max_turns {
        // Compaction check.
        if session.cumulative_input_tokens >= args.compact_at {
            eprintln!(
                "[printer] cumulative input tokens {} >= {}; rotating session",
                session.cumulative_input_tokens, args.compact_at
            );
            session.rotate();
            let outcome = session
                .turn(&rotation_prompt(&printer_bin_str, &spec_abs.to_string_lossy()))
                .await?;
            print_result_tail(&outcome.result_text);
            if let Some(reason) = blocked_reason(&outcome.result_text) {
                anyhow::bail!("agent reported blocked: {reason}");
            }
            tasks = store::list_all(tasks_dir)?;
            prev_state_hash = state_hash(&tasks);
            if all_done(&tasks) {
                eprintln!("[printer] all tasks done.");
                return Ok(());
            }
            continue;
        }

        let prompt = if stalls > 0 {
            unstall_prompt(&printer_bin_str)
        } else {
            nudge_prompt_with(&printer_bin_str, injected_block, injected_skills)
        };
        let outcome = session.turn(&prompt).await?;
        print_result_tail(&outcome.result_text);

        if let Some(reason) = blocked_reason(&outcome.result_text) {
            anyhow::bail!("agent reported blocked: {reason}");
        }

        tasks = store::list_all(tasks_dir)?;

        if outcome.result_text.contains(SENTINEL_DONE) {
            if all_done(&tasks) {
                eprintln!("[printer] all tasks done.");
                return Ok(());
            } else {
                eprintln!(
                    "[printer] agent emitted {SENTINEL_DONE} but the task store still has unfinished work; nudging once more"
                );
            }
        }
        if all_done(&tasks) {
            eprintln!("[printer] all tasks done.");
            return Ok(());
        }

        // Stall detection: did any task transition this turn?
        let new_hash = state_hash(&tasks);
        if new_hash == prev_state_hash {
            stalls += 1;
            if stalls >= 3 {
                anyhow::bail!("agent stalled for 3 consecutive turns; aborting");
            }
        } else {
            stalls = 0;
            prev_state_hash = new_hash;
        }
    }

    anyhow::bail!("--max-turns {} exhausted", args.max_turns);
}

fn sync_spec(spec_abs: &std::path::Path, tasks_dir: &std::path::Path) -> Result<spec::SyncReport> {
    let text = std::fs::read_to_string(spec_abs)
        .with_context(|| format!("reading spec {}", spec_abs.display()))?;
    let items = spec::parse_spec(&text);
    spec::sync_to_store(&items, spec_abs, tasks_dir)
}

fn any_tasks(tasks_dir: &std::path::Path) -> Result<bool> {
    Ok(!store::list_all(tasks_dir)?.is_empty())
}

fn all_done(tasks: &[Task]) -> bool {
    !tasks.is_empty() && tasks.iter().all(|t| t.meta.status == Status::Done)
}

fn state_hash(tasks: &[Task]) -> u64 {
    let mut h = DefaultHasher::new();
    // Include id + status + updated_at so any state transition changes the hash.
    for t in tasks {
        t.meta.id.hash(&mut h);
        t.meta.status.hash(&mut h);
        t.meta.updated_at.hash(&mut h);
    }
    h.finish()
}

fn print_result_tail(result_text: &str) {
    let trimmed = result_text.trim();
    if trimmed.is_empty() {
        return;
    }
    println!("{}", trimmed);
}

fn blocked_reason(result_text: &str) -> Option<String> {
    let idx = result_text.find(SENTINEL_BLOCKED)?;
    let after = &result_text[idx + SENTINEL_BLOCKED.len()..];
    let end = after.find('\n').unwrap_or(after.len());
    let line = &after[..end];
    let reason = line.trim_end_matches('>').trim();
    Some(reason.to_string())
}
