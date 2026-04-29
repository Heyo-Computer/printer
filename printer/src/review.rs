use crate::agent::{AgentInvocation, TokenUsage};
use crate::cli::ReviewArgs;
use crate::hooks::{Event, HookContext, HookSet};
use crate::prompts::review_prompt_with;
use crate::session::Session;
use crate::skills;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Verdict parsed out of the review report's `## Verdict` section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Partial,
    Fail,
    /// Could not parse a verdict from the report (treat as fail-safe).
    Unknown,
}

impl Verdict {
    pub fn is_pass(&self) -> bool {
        matches!(self, Verdict::Pass)
    }
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Verdict::Pass => "PASS",
            Verdict::Partial => "PARTIAL",
            Verdict::Fail => "FAIL",
            Verdict::Unknown => "UNKNOWN",
        })
    }
}

/// Normalized result of a single review turn.
#[derive(Debug, Clone)]
pub struct ReviewOutcome {
    pub usage: TokenUsage,
    pub verdict: Verdict,
    pub report: String,
}

/// Parse the verdict out of a review report. We look for a `## Verdict`
/// section heading and pick up the first PASS / PARTIAL / FAIL token after it.
/// Falls back to scanning the whole report if the heading is missing.
pub fn parse_verdict(report: &str) -> Verdict {
    let lower = report.to_ascii_lowercase();
    let scan = match lower.find("## verdict") {
        Some(idx) => &lower[idx..],
        None => lower.as_str(),
    };
    // Pick the first explicit verdict token.
    let pass_at = scan.find("pass");
    let partial_at = scan.find("partial");
    let fail_at = scan.find("fail");
    let mut best: Option<(usize, Verdict)> = None;
    for cand in [
        partial_at.map(|i| (i, Verdict::Partial)),
        pass_at.map(|i| (i, Verdict::Pass)),
        fail_at.map(|i| (i, Verdict::Fail)),
    ]
    .into_iter()
    .flatten()
    {
        best = Some(match best {
            None => cand,
            Some(cur) if cand.0 < cur.0 => cand,
            Some(cur) => cur,
        });
    }
    best.map(|(_, v)| v).unwrap_or(Verdict::Unknown)
}

pub async fn review(args: ReviewArgs) -> Result<ReviewOutcome> {
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
    let report = result?;
    let verdict = parse_verdict(&report);
    eprintln!("[printer] review verdict: {verdict}");
    eprintln!("[printer] review token usage: {usage}");
    Ok(ReviewOutcome {
        usage,
        verdict,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pass_verdict() {
        let report = "## Verdict\nPASS\n\n## Per-item findings\n- foo MET\n";
        assert_eq!(parse_verdict(report), Verdict::Pass);
    }

    #[test]
    fn parses_partial_verdict() {
        let report = "## Verdict\nPARTIAL — two items missing.\n";
        assert_eq!(parse_verdict(report), Verdict::Partial);
    }

    #[test]
    fn parses_fail_verdict() {
        let report = "## Verdict\nFAIL\nseveral items missing.\n";
        assert_eq!(parse_verdict(report), Verdict::Fail);
    }

    #[test]
    fn unknown_when_no_verdict_section() {
        let report = "lots of prose with no verdict here\n";
        assert_eq!(parse_verdict(report), Verdict::Unknown);
    }

    #[test]
    fn ignores_pass_in_per_item_findings() {
        // No `## Verdict` section, but the body uses MET / MISSING tokens.
        // We still scan the whole text — accept the first hit, prefer PARTIAL
        // over FAIL/PASS when ambiguous.
        let report = "## Per-item findings\n- foo MET\n- bar MISSING (partial coverage)\n";
        assert_eq!(parse_verdict(report), Verdict::Partial);
    }
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
