use crate::agent::AgentInvocation;
use crate::cli::ReviewArgs;
use crate::prompts::review_prompt;
use crate::session::Session;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub async fn review(args: ReviewArgs) -> Result<()> {
    let spec_abs = args
        .spec
        .canonicalize()
        .with_context(|| format!("spec file not found: {}", args.spec.display()))?;
    let cwd = match args.cwd.as_deref() {
        Some(p) => Some(p.canonicalize().with_context(|| format!("--cwd not found: {}", p.display()))?),
        None => None,
    };
    let cwd_ref = cwd.as_deref();

    let base = match args.base {
        Some(b) => b,
        None => detect_base(cwd_ref).unwrap_or_else(|| "HEAD~1".to_string()),
    };
    eprintln!("[printer] reviewing against base ref: {base}");

    let agent = AgentInvocation {
        kind: args.agent,
        model: args.model.as_deref(),
        cwd: cwd_ref,
        permission_mode: &args.permission_mode,
    };
    let mut session = Session::new(agent).with_verbose(args.verbose);

    let spec_arg = spec_abs.to_string_lossy().into_owned();
    let outcome = session.turn(&review_prompt(&spec_arg, &base)).await?;
    let report = outcome.result_text.trim();
    println!("{report}");

    if let Some(out) = args.out.as_deref() {
        std::fs::write(out, format!("{report}\n"))
            .with_context(|| format!("failed to write review report to {}", out.display()))?;
        eprintln!("[printer] review written to {}", out.display());
    }
    Ok(())
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
