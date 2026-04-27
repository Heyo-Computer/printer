use crate::cli::{ExecArgs, ReviewArgs, RunArgs};
use crate::hooks::{Event, HookContext, HookSet};
use crate::{review, run};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const CHECKPOINT_REL: &str = ".printer/exec.json";

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub spec: PathBuf,
    pub phase: Phase,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
        (false, Some(cp), Some(spec)) => bail!(
            "checkpoint at {CHECKPOINT_REL} is for spec {}; refusing to overwrite with {}. \
             Pass --continue, or remove the checkpoint to start fresh.",
            cp.spec.display(),
            spec.display()
        ),
        (false, _, None) => bail!("missing spec path (required unless --continue is set)"),
    }
}

pub async fn exec(args: ExecArgs) -> Result<()> {
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

    let hooks = HookSet::load_installed().unwrap_or_default();
    let exec_spec: Option<PathBuf> = match &action {
        Action::Fresh { spec } | Action::ResumeRun { spec } | Action::ResumeReview { spec } => {
            Some(spec.clone())
        }
        Action::AlreadyDone { spec } => Some(spec.clone()),
    };
    {
        let mut ctx = HookContext::new(Event::BeforeExec, cwd.clone());
        if let Some(s) = &exec_spec {
            ctx = ctx.with_spec(s.clone());
        }
        hooks.run_cli(Event::BeforeExec, &ctx)?;
    }

    let outcome = run_action(&args, &cwd, &checkpoint_path, action, existing).await;

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
) -> Result<()> {
    let _ = cwd;
    match action {
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
            do_run_then_review(&args, &spec, &checkpoint_path).await?;
        }
        Action::ResumeRun { spec } => {
            // Bump updated_at so the file reflects this resume.
            let mut cp = existing.unwrap();
            cp.updated_at = Utc::now();
            cp.save(&checkpoint_path)?;
            do_run_then_review(&args, &spec, &checkpoint_path).await?;
        }
        Action::ResumeReview { spec } => {
            eprintln!("[printer] resuming at review phase for {}", spec.display());
            do_review(&args, &spec, &checkpoint_path).await?;
        }
    }

    Ok(())
}

async fn do_run_then_review(args: &ExecArgs, spec: &Path, cp_path: &Path) -> Result<()> {
    run::run(build_run_args(args, spec)).await?;
    write_phase(cp_path, spec, Phase::ReviewPending)?;
    do_review(args, spec, cp_path).await
}

async fn do_review(args: &ExecArgs, spec: &Path, cp_path: &Path) -> Result<()> {
    review::review(build_review_args(args, spec)).await?;
    write_phase(cp_path, spec, Phase::Done)?;
    Ok(())
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
    }
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
    fn fresh_with_existing_checkpoint_for_different_spec_errors() {
        let c = cp("/tmp/a.md", Phase::Running);
        let err = decide(Some(Path::new("/tmp/b.md")), false, Some(&c)).unwrap_err();
        assert!(err.to_string().contains("refusing to overwrite"));
    }

    #[test]
    fn fresh_with_existing_checkpoint_same_spec_resumes() {
        let c = cp("/tmp/a.md", Phase::ReviewPending);
        let action = decide(Some(Path::new("/tmp/a.md")), false, Some(&c)).unwrap();
        assert_eq!(action, Action::ResumeReview { spec: PathBuf::from("/tmp/a.md") });
    }
}
