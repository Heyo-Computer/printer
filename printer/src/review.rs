use crate::agent::{AgentInvocation, TokenUsage};
use crate::cli::ReviewArgs;
use crate::hooks::{Event, HookContext, HookSet};
use crate::prompts::review_prompt_with;
use crate::session::Session;
use crate::skills;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub async fn review(args: ReviewArgs) -> Result<TokenUsage> {
    let hooks = HookSet::load_installed().unwrap_or_default();

    let spec_abs = args
        .spec
        .canonicalize()
        .with_context(|| format!("spec file not found: {}", args.spec.display()))?;
    let cwd = match args.cwd.as_deref() {
        Some(p) => Some(p.canonicalize().with_context(|| format!("--cwd not found: {}", p.display()))?),
        None => None,
    };
    let cwd_ref = cwd.as_deref();
    let cwd_for_hook = cwd.clone().unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let base = match args.base {
        Some(b) => b,
        None => detect_base(cwd_ref).unwrap_or_else(|| "HEAD~1".to_string()),
    };
    eprintln!("[printer] reviewing against base ref: {base}");

    hooks.run_cli(
        Event::BeforeReview,
        &HookContext::new(Event::BeforeReview, cwd_for_hook.clone())
            .with_spec(spec_abs.clone())
            .with_base_ref(base.clone()),
    )?;

    let agent_contrib = hooks.agent_for(Event::BeforeReview);
    let injected_block = agent_contrib.render_prompt_block();

    let default_skills_root = cwd_ref
        .map(|d| d.join(".claude").join("skills"))
        .unwrap_or_else(|| std::path::PathBuf::from(".claude/skills"));
    // Merge user-supplied --skill paths with hook-contributed skill paths.
    let mut all_skill_paths = args.skills.clone();
    all_skill_paths.extend(agent_contrib.skills.iter().cloned());
    let resolved_skills = skills::resolve(&all_skill_paths, Some(&default_skills_root))?;
    if !resolved_skills.is_empty() {
        let names: Vec<&str> = resolved_skills.iter().map(|s| s.name.as_str()).collect();
        eprintln!("[printer] skills available to reviewer: {}", names.join(", "));
    }

    let agent = AgentInvocation {
        kind: args.agent,
        model: args.model.as_deref(),
        cwd: cwd_ref,
        permission_mode: &args.permission_mode,
    };
    let mut session = Session::new(agent).with_verbose(args.verbose);

    let spec_arg = spec_abs.to_string_lossy().into_owned();
    let result: Result<String> = async {
        let outcome = session
            .turn(&review_prompt_with(
                &spec_arg,
                &base,
                &resolved_skills,
                injected_block.as_deref(),
            ))
            .await?;
        let report = outcome.result_text.trim().to_string();
        println!("{report}");

        if let Some(out) = args.out.as_deref() {
            std::fs::write(out, format!("{report}\n"))
                .with_context(|| format!("failed to write review report to {}", out.display()))?;
            eprintln!("[printer] review written to {}", out.display());
        }
        Ok(report)
    }
    .await;

    let mut after_ctx = HookContext::new(Event::AfterReview, cwd_for_hook)
        .with_spec(spec_abs.clone())
        .with_base_ref(base)
        .with_exit_status(result.is_ok());
    if let Some(out) = args.out.as_deref() {
        after_ctx = after_ctx.with_report_path(out.to_path_buf());
    }
    let _ = hooks.run_cli(Event::AfterReview, &after_ctx);

    let usage = session.usage_total;
    if result.is_ok() {
        eprintln!("[printer] review token usage: {usage}");
    }
    result.map(|_| usage)
}

fn detect_base(cwd: Option<&Path>) -> Option<String> {
    // Try `main`, then `master`, then HEAD~1.
    for candidate in ["main", "master"] {
        let mut cmd = Command::new("git");
        cmd.args(["rev-parse", "--verify", "--quiet", candidate]);
        if let Some(d) = cwd {
            cmd.current_dir(d);
        }
        if let Ok(out) = cmd.output() {
            if out.status.success() {
                return Some(candidate.to_string());
            }
        }
    }
    None
}
