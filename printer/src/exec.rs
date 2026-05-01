use crate::agent::TokenUsage;
use crate::cli::{ExecArgs, HistoryArgs, ReviewArgs, RunArgs};
use crate::codegraph_watch;
use crate::drivers::{ActiveSandbox, DriverSet};
use crate::hooks::{Event, HookContext, HookSet};
use crate::{review, run};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const CHECKPOINT_REL: &str = ".printer/exec.json";
const HISTORY_REL: &str = ".printer/history.json";
/// Default upper bound on review cycles (one initial review + N-1 follow-ups).
/// Each non-PASS verdict triggers a fix pass and another review, so this caps
/// how many round-trips we'll attempt before giving up.
pub const DEFAULT_MAX_REVIEW_PASSES: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Implementation loop in progress (or crashed mid-loop).
    Running,
    /// Run completed cleanly; review has not started yet.
    ReviewPending,
    /// Review report produced; nothing left to do.
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub spec: PathBuf,
    pub phase: Phase,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// One archived exec — a Checkpoint plus the time it was retired from
/// `.printer/exec.json` (i.e. a follow-up spec replaced it).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    #[serde(flatten)]
    pub checkpoint: Checkpoint,
    pub archived_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct History {
    pub entries: Vec<HistoryEntry>,
}

impl History {
    fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading history {}", path.display()))?;
        let history: History = serde_json::from_str(&raw)
            .with_context(|| format!("parsing history {}", path.display()))?;
        Ok(history)
    }

    fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let body = serde_json::to_string_pretty(self).context("serializing history")?;
        std::fs::write(path, body)
            .with_context(|| format!("writing history {}", path.display()))?;
        Ok(())
    }

    fn append(path: &Path, entry: HistoryEntry) -> Result<()> {
        let mut h = Self::load(path)?;
        h.entries.push(entry);
        h.save(path)
    }
}

pub fn history_path(cwd: &Path) -> PathBuf {
    cwd.join(HISTORY_REL)
}

pub fn load_history(cwd: &Path) -> Result<History> {
    History::load(&history_path(cwd))
}

pub fn print_history(args: HistoryArgs) -> Result<()> {
    let cwd: PathBuf = match args.cwd.as_deref() {
        Some(p) => p
            .canonicalize()
            .with_context(|| format!("--cwd not found: {}", p.display()))?,
        None => std::env::current_dir()?,
    };

    let history = load_history(&cwd)?;
    let current_path = cwd.join(CHECKPOINT_REL);
    let current = if current_path.exists() {
        Some(Checkpoint::load(&current_path)?)
    } else {
        None
    };

    if args.json {
        #[derive(Serialize)]
        struct Out<'a> {
            current: Option<&'a Checkpoint>,
            history: &'a History,
        }
        let out = Out {
            current: current.as_ref(),
            history: &history,
        };
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    if history.entries.is_empty() && current.is_none() {
        println!("No exec history at {}.", history_path(&cwd).display());
        return Ok(());
    }

    if !history.entries.is_empty() {
        println!("Archived execs ({}):", history.entries.len());
        for (i, e) in history.entries.iter().enumerate() {
            println!(
                "  [{}] {}  phase={:?}  started={}  finished={}  archived={}",
                i + 1,
                e.checkpoint.spec.display(),
                e.checkpoint.phase,
                e.checkpoint.started_at.to_rfc3339(),
                e.checkpoint.updated_at.to_rfc3339(),
                e.archived_at.to_rfc3339(),
            );
        }
    }

    if let Some(cp) = current {
        println!(
            "\nCurrent checkpoint ({}): {}  phase={:?}  started={}  updated={}",
            current_path.display(),
            cp.spec.display(),
            cp.phase,
            cp.started_at.to_rfc3339(),
            cp.updated_at.to_rfc3339(),
        );
    }

    Ok(())
}

impl Checkpoint {
    fn new(spec: PathBuf, phase: Phase) -> Self {
        let now = Utc::now();
        Self {
            spec,
            phase,
            started_at: now,
            updated_at: now,
        }
    }

    fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading checkpoint {}", path.display()))?;
        let cp: Checkpoint = serde_json::from_str(&raw)
            .with_context(|| format!("parsing checkpoint {}", path.display()))?;
        Ok(cp)
    }

    fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let body = serde_json::to_string_pretty(self)
            .context("serializing checkpoint")?;
        std::fs::write(path, body)
            .with_context(|| format!("writing checkpoint {}", path.display()))?;
        Ok(())
    }
}

/// Decision the dispatcher makes after consulting the checkpoint and flags.
/// Pulled out so it can be unit-tested without spawning agents.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    /// No checkpoint or fresh start; create one at `spec` and run+review.
    Fresh { spec: PathBuf },
    /// Resume from a `running` checkpoint: re-enter run, then review.
    ResumeRun { spec: PathBuf },
    /// Skip run, go straight to review.
    ResumeReview { spec: PathBuf },
    /// Already done; nothing to do.
    AlreadyDone { spec: PathBuf },
    /// Prior exec for a different spec finished cleanly; archive it to
    /// history and start a fresh exec for the new spec.
    FreshAfterDone { spec: PathBuf, prior: Checkpoint },
}

/// Pure decision function — no I/O, no agent spawning. The caller has already
/// loaded the checkpoint (if any) and resolved the spec from CLI args.
fn decide(
    cli_spec: Option<&Path>,
    cont: bool,
    existing: Option<&Checkpoint>,
) -> Result<Action> {
    match (cont, existing, cli_spec) {
        (true, None, _) => bail!(
            "--continue requested but no checkpoint at {CHECKPOINT_REL}; run without --continue first"
        ),
        (true, Some(cp), cli) => {
            // If the user passed a spec, it must match the checkpoint.
            if let Some(s) = cli {
                if s != cp.spec {
                    bail!(
                        "--continue spec {} does not match checkpoint spec {}",
                        s.display(),
                        cp.spec.display()
                    );
                }
            }
            Ok(match cp.phase {
                Phase::Running => Action::ResumeRun { spec: cp.spec.clone() },
                Phase::ReviewPending => Action::ResumeReview { spec: cp.spec.clone() },
                Phase::Done => Action::AlreadyDone { spec: cp.spec.clone() },
            })
        }
        (false, None, Some(spec)) => Ok(Action::Fresh { spec: spec.to_path_buf() }),
        (false, Some(cp), Some(spec)) if spec == cp.spec => {
            // Same spec — proceed from the recorded phase. This makes a
            // bare `printer exec spec.md` after a crash do the right thing
            // even without --continue.
            Ok(match cp.phase {
                Phase::Running => Action::ResumeRun { spec: cp.spec.clone() },
                Phase::ReviewPending => Action::ResumeReview { spec: cp.spec.clone() },
                Phase::Done => Action::AlreadyDone { spec: cp.spec.clone() },
            })
        }
        (false, Some(cp), Some(spec)) => {
            // Different spec from the checkpoint. If the prior exec finished
            // cleanly, treat this as a follow-up: archive the old checkpoint
            // and start fresh. Otherwise the prior exec is still in flight,
            // so refuse to clobber it.
            if cp.phase == Phase::Done {
                Ok(Action::FreshAfterDone {
                    spec: spec.to_path_buf(),
                    prior: cp.clone(),
                })
            } else {
                bail!(
                    "checkpoint at {CHECKPOINT_REL} is for spec {} (phase {:?}); \
                     refusing to overwrite with {}. Pass --continue to resume the \
                     prior exec, or remove the checkpoint to start fresh.",
                    cp.spec.display(),
                    cp.phase,
                    spec.display()
                )
            }
        }
        (false, _, None) => bail!("missing spec path (required unless --continue is set)"),
    }
}

pub async fn exec(args: ExecArgs) -> Result<()> {
    crate::plugins::prompt_if_no_plugins(args.skip_plugin_check)?;
    let cwd: PathBuf = match args.cwd.as_deref() {
        Some(p) => p
            .canonicalize()
            .with_context(|| format!("--cwd not found: {}", p.display()))?,
        None => std::env::current_dir()?,
    };
    let checkpoint_path = cwd.join(CHECKPOINT_REL);

    let cli_spec_abs: Option<PathBuf> = match args.spec.as_deref() {
        Some(p) => Some(
            p.canonicalize()
                .with_context(|| format!("spec file not found: {}", p.display()))?,
        ),
        None => None,
    };

    let existing = if checkpoint_path.exists() {
        Some(Checkpoint::load(&checkpoint_path)?)
    } else {
        None
    };

    let action = decide(cli_spec_abs.as_deref(), args.r#continue, existing.as_ref())?;

    // Spawn the codegraph watch daemon at the exec level so a single daemon
    // covers both run and review. The inner run is configured (via
    // build_run_args) with no_codegraph_watch=true so it won't double-spawn.
    let _watch_guard = if args.no_codegraph_watch {
        None
    } else {
        codegraph_watch::try_spawn(&cwd).unwrap_or_else(|e| {
            eprintln!("[printer] codegraph watch spawn failed: {e}; continuing without daemon");
            None
        })
    };

    let hooks = HookSet::load_installed().unwrap_or_default();
    let exec_spec: Option<PathBuf> = match &action {
        Action::Fresh { spec }
        | Action::ResumeRun { spec }
        | Action::ResumeReview { spec }
        | Action::AlreadyDone { spec }
        | Action::FreshAfterDone { spec, .. } => Some(spec.clone()),
    };

    // Provision the sandbox once at the exec level so a single VM covers both
    // run and review. The inner phases are passed `no_sandbox=true` (via
    // build_run_args / build_review_args) so they won't try to create their
    // own; instead they receive a borrowed reference to this one.
    let sandbox = if args.no_sandbox {
        None
    } else {
        acquire_exec_sandbox(&cwd, exec_spec.clone())?
    };
    if let Some(sb) = sandbox.as_ref() {
        sb.sync_in()?;
    }

    {
        let mut ctx = HookContext::new(Event::BeforeExec, cwd.clone());
        if let Some(s) = &exec_spec {
            ctx = ctx.with_spec(s.clone());
        }
        hooks.run_cli(Event::BeforeExec, &ctx)?;
    }

    let outcome = run_action(&args, &cwd, &checkpoint_path, action, existing, sandbox.as_ref()).await;
    if let Some(sb) = sandbox.as_ref() {
        sb.sync_out();
    }

    {
        let mut ctx = HookContext::new(Event::AfterExec, cwd.clone()).with_exit_status(outcome.is_ok());
        if let Some(s) = &exec_spec {
            ctx = ctx.with_spec(s.clone());
        }
        let _ = hooks.run_cli(Event::AfterExec, &ctx);
    }

    outcome
}

async fn run_action(
    args: &ExecArgs,
    cwd: &Path,
    checkpoint_path: &Path,
    action: Action,
    existing: Option<Checkpoint>,
    sandbox: Option<&ActiveSandbox>,
) -> Result<()> {
    let total = match action {
        Action::AlreadyDone { spec } => {
            eprintln!(
                "[printer] exec already complete for {} (checkpoint phase=done). \
                 Remove {} to start over.",
                spec.display(),
                checkpoint_path.display()
            );
            return Ok(());
        }
        Action::Fresh { spec } => {
            let cp = Checkpoint::new(spec.clone(), Phase::Running);
            cp.save(&checkpoint_path)?;
            do_run_then_review(&args, &spec, &checkpoint_path, sandbox).await?
        }
        Action::FreshAfterDone { spec, prior } => {
            let history_file = cwd.join(HISTORY_REL);
            let entry = HistoryEntry {
                checkpoint: prior.clone(),
                archived_at: Utc::now(),
            };
            History::append(&history_file, entry)?;
            eprintln!(
                "[printer] archived prior exec for {} to {} (phase=done); starting fresh for {}",
                prior.spec.display(),
                history_file.display(),
                spec.display()
            );
            let cp = Checkpoint::new(spec.clone(), Phase::Running);
            cp.save(&checkpoint_path)?;
            do_run_then_review(&args, &spec, &checkpoint_path, sandbox).await?
        }
        Action::ResumeRun { spec } => {
            // Bump updated_at so the file reflects this resume.
            let mut cp = existing.unwrap();
            cp.updated_at = Utc::now();
            cp.save(&checkpoint_path)?;
            do_run_then_review(&args, &spec, &checkpoint_path, sandbox).await?
        }
        Action::ResumeReview { spec } => {
            eprintln!("[printer] resuming at review phase for {}", spec.display());
            do_review(&args, &spec, &checkpoint_path, sandbox).await?
        }
    };

    eprintln!("[printer] exec token usage (run + review): {total}");
    Ok(())
}

async fn do_run_then_review(
    args: &ExecArgs,
    spec: &Path,
    cp_path: &Path,
    sandbox: Option<&ActiveSandbox>,
) -> Result<TokenUsage> {
    let mut total = run::run_with_sandbox(build_run_args(args, spec), None, sandbox).await?;
    write_phase(cp_path, spec, Phase::ReviewPending)?;
    let review_total = do_review(args, spec, cp_path, sandbox).await?;
    total.add(&review_total);
    Ok(total)
}

/// Drive the review phase, with up to `max_review_passes` cycles of
/// review → fix → re-review. Each iteration:
///   1. runs the review agent and parses its verdict
///   2. if PASS, we're done
///   3. otherwise, feeds the report to the coding agent (which queues fix
///      tasks and works them) and loops back to (1)
/// Stops early if the verdict is PASS or the cap is hit.
async fn do_review(
    args: &ExecArgs,
    spec: &Path,
    cp_path: &Path,
    sandbox: Option<&ActiveSandbox>,
) -> Result<TokenUsage> {
    let max_passes = args
        .max_review_passes
        .unwrap_or(DEFAULT_MAX_REVIEW_PASSES)
        .max(1);
    let mut total = TokenUsage::default();

    for pass in 1..=max_passes {
        eprintln!("[printer] review pass {pass}/{max_passes}");
        let outcome = review::review_with_sandbox(build_review_args(args, spec), sandbox).await?;
        total.add(&outcome.usage);

        if outcome.verdict.is_pass() {
            eprintln!("[printer] review verdict PASS on pass {pass}; finishing");
            break;
        }

        if pass == max_passes {
            eprintln!(
                "[printer] review verdict {} on final pass {pass}/{max_passes}; \
                 stopping cycle without a PASS",
                outcome.verdict
            );
            break;
        }

        eprintln!(
            "[printer] review verdict {} on pass {pass}; feeding report back to coding agent",
            outcome.verdict
        );
        let fix_usage = run::run_with_sandbox(
            build_run_args(args, spec),
            Some(outcome.report.as_str()),
            sandbox,
        )
        .await
        .with_context(|| format!("fix pass after review pass {pass} failed"))?;
        total.add(&fix_usage);
    }

    write_phase(cp_path, spec, Phase::Done)?;
    Ok(total)
}

fn write_phase(cp_path: &Path, spec: &Path, phase: Phase) -> Result<()> {
    let mut cp = if cp_path.exists() {
        Checkpoint::load(cp_path)?
    } else {
        Checkpoint::new(spec.to_path_buf(), phase)
    };
    cp.phase = phase;
    cp.spec = spec.to_path_buf();
    cp.updated_at = Utc::now();
    cp.save(cp_path)
}

fn build_run_args(args: &ExecArgs, spec: &Path) -> RunArgs {
    RunArgs {
        spec: spec.to_path_buf(),
        agent: args.agent,
        model: args.model.clone(),
        max_turns: args.max_turns,
        compact_at: args.compact_at,
        cwd: args.cwd.clone(),
        permission_mode: args.permission_mode.clone(),
        verbose: args.verbose,
        // Exec already owns the daemon (or chose not to spawn one); never
        // double-spawn from the inner run.
        no_codegraph_watch: true,
        // Exec already ran the plugin check up front; suppress it inside the
        // nested run so we don't re-prompt the user mid-exec.
        skip_plugin_check: true,
        // Exec acquires one sandbox covering both phases and passes it down
        // explicitly via run_with_sandbox; the inner run must not create its
        // own.
        no_sandbox: true,
    }
}

fn build_review_args(args: &ExecArgs, spec: &Path) -> ReviewArgs {
    ReviewArgs {
        spec: spec.to_path_buf(),
        agent: args.agent,
        model: args.model.clone(),
        base: args.base.clone(),
        cwd: args.cwd.clone(),
        out: args.out.clone(),
        permission_mode: args.permission_mode.clone(),
        skills: args.skills.clone(),
        verbose: args.verbose,
        no_sandbox: true,
    }
}

fn acquire_exec_sandbox(cwd: &Path, spec: Option<PathBuf>) -> Result<Option<ActiveSandbox>> {
    let cfg = match crate::config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[printer] failed to load config ({e}); using defaults");
            crate::config::GlobalConfig::default()
        }
    };
    let drivers = match DriverSet::load_installed() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[printer] failed to load drivers ({e}); continuing without sandbox");
            return Ok(None);
        }
    };
    let Some(active) = drivers.resolve(&cfg.sandbox.driver)? else {
        return Ok(None);
    };
    let merged = active.with_overrides(&cfg.sandbox.commands)?;
    Ok(Some(ActiveSandbox::create(
        merged,
        cwd.to_path_buf(),
        spec,
        Some(cfg.sandbox.base_image.clone()),
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cp(spec: &str, phase: Phase) -> Checkpoint {
        Checkpoint::new(PathBuf::from(spec), phase)
    }

    #[test]
    fn checkpoint_round_trips() {
        let original = cp("/tmp/spec.md", Phase::ReviewPending);
        let s = serde_json::to_string(&original).unwrap();
        let parsed: Checkpoint = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.spec, original.spec);
        assert_eq!(parsed.phase, original.phase);
    }

    #[test]
    fn fresh_with_spec_no_checkpoint() {
        let action = decide(Some(Path::new("/tmp/a.md")), false, None).unwrap();
        assert_eq!(action, Action::Fresh { spec: PathBuf::from("/tmp/a.md") });
    }

    #[test]
    fn fresh_without_spec_errors() {
        let err = decide(None, false, None).unwrap_err();
        assert!(err.to_string().contains("missing spec"));
    }

    #[test]
    fn continue_without_checkpoint_errors() {
        let err = decide(None, true, None).unwrap_err();
        assert!(err.to_string().contains("--continue"));
    }

    #[test]
    fn continue_with_done_yields_already_done() {
        let c = cp("/tmp/a.md", Phase::Done);
        let action = decide(None, true, Some(&c)).unwrap();
        assert_eq!(action, Action::AlreadyDone { spec: PathBuf::from("/tmp/a.md") });
    }

    #[test]
    fn continue_with_running_yields_resume_run() {
        let c = cp("/tmp/a.md", Phase::Running);
        let action = decide(None, true, Some(&c)).unwrap();
        assert_eq!(action, Action::ResumeRun { spec: PathBuf::from("/tmp/a.md") });
    }

    #[test]
    fn continue_with_review_pending_yields_resume_review() {
        let c = cp("/tmp/a.md", Phase::ReviewPending);
        let action = decide(None, true, Some(&c)).unwrap();
        assert_eq!(action, Action::ResumeReview { spec: PathBuf::from("/tmp/a.md") });
    }

    #[test]
    fn continue_spec_mismatch_errors() {
        let c = cp("/tmp/a.md", Phase::Running);
        let err = decide(Some(Path::new("/tmp/b.md")), true, Some(&c)).unwrap_err();
        assert!(err.to_string().contains("does not match"));
    }

    #[test]
    fn fresh_with_running_checkpoint_for_different_spec_errors() {
        let c = cp("/tmp/a.md", Phase::Running);
        let err = decide(Some(Path::new("/tmp/b.md")), false, Some(&c)).unwrap_err();
        assert!(err.to_string().contains("refusing to overwrite"));
    }

    #[test]
    fn fresh_with_review_pending_checkpoint_for_different_spec_errors() {
        let c = cp("/tmp/a.md", Phase::ReviewPending);
        let err = decide(Some(Path::new("/tmp/b.md")), false, Some(&c)).unwrap_err();
        assert!(err.to_string().contains("refusing to overwrite"));
    }

    #[test]
    fn fresh_with_done_checkpoint_for_different_spec_archives() {
        let c = cp("/tmp/a.md", Phase::Done);
        let action = decide(Some(Path::new("/tmp/b.md")), false, Some(&c)).unwrap();
        match action {
            Action::FreshAfterDone { spec, prior } => {
                assert_eq!(spec, PathBuf::from("/tmp/b.md"));
                assert_eq!(prior.spec, PathBuf::from("/tmp/a.md"));
                assert_eq!(prior.phase, Phase::Done);
            }
            other => panic!("expected FreshAfterDone, got {other:?}"),
        }
    }

    #[test]
    fn history_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");
        let entry = HistoryEntry {
            checkpoint: cp("/tmp/a.md", Phase::Done),
            archived_at: Utc::now(),
        };
        History::append(&path, entry.clone()).unwrap();

        let entry2 = HistoryEntry {
            checkpoint: cp("/tmp/b.md", Phase::Done),
            archived_at: Utc::now(),
        };
        History::append(&path, entry2).unwrap();

        let loaded = History::load(&path).unwrap();
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries[0].checkpoint.spec, PathBuf::from("/tmp/a.md"));
        assert_eq!(loaded.entries[1].checkpoint.spec, PathBuf::from("/tmp/b.md"));
    }

    #[test]
    fn fresh_with_existing_checkpoint_same_spec_resumes() {
        let c = cp("/tmp/a.md", Phase::ReviewPending);
        let action = decide(Some(Path::new("/tmp/a.md")), false, Some(&c)).unwrap();
        assert_eq!(action, Action::ResumeReview { spec: PathBuf::from("/tmp/a.md") });
    }
}
