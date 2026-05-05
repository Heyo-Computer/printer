use crate::agent::{AgentInvocation, TokenUsage};
use crate::cli::ReviewArgs;
use crate::drivers::{ActiveSandbox, DriverSet};
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

/// Extract the body of a top-level markdown section (e.g. `## Suggested follow-ups`)
/// from a review report. Header match is case-insensitive on the heading text.
/// Returns None if the section is missing, or its body trims to empty / "none".
pub fn extract_section(report: &str, heading: &str) -> Option<String> {
    let needle = heading.trim().to_ascii_lowercase();
    let mut lines = report.lines().enumerate();
    let mut start: Option<usize> = None;
    for (i, line) in lines.by_ref() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("## ") {
            if rest.trim().to_ascii_lowercase() == needle.strip_prefix("## ").unwrap_or(&needle) {
                start = Some(i + 1);
                break;
            }
        }
    }
    let start = start?;
    let collected: Vec<&str> = report
        .lines()
        .skip(start)
        .take_while(|l| !l.trim_start().starts_with("## "))
        .collect();
    let body = collected.join("\n").trim().to_string();
    if body.is_empty() || body.eq_ignore_ascii_case("none") {
        None
    } else {
        Some(body)
    }
}

/// Persist the parsed `## Suggested follow-ups` section of a review report to
/// `.printer/followups/<spec-stem>.md` under `cwd`. Overwrites existing file
/// so the most recent review wins. Returns the path written.
pub fn write_followups(
    cwd: &Path,
    spec_abs: &Path,
    verdict: Verdict,
    report: &str,
) -> Result<PathBuf> {
    let stem = spec_abs
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "spec".to_string());
    let dir = cwd.join(".printer").join("followups");
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating {}", dir.display()))?;
    let out = dir.join(format!("{stem}.md"));
    let body = extract_section(report, "## Suggested follow-ups").unwrap_or_else(|| "none".into());
    let now = chrono::Utc::now().to_rfc3339();
    let contents = format!(
        "# Follow-ups for {}\n\nGenerated: {}\nVerdict: {}\n\n## Suggested follow-ups\n\n{}\n",
        spec_abs.display(),
        now,
        verdict,
        body,
    );
    std::fs::write(&out, contents)
        .with_context(|| format!("writing followups to {}", out.display()))?;
    Ok(out)
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
    let sandbox = acquire_sandbox(&args)?;
    if let Some(sb) = sandbox.as_ref() {
        sb.sync_in()?;
    }
    let outcome = review_with_sandbox(args, sandbox.as_ref()).await;
    if let Some(sb) = sandbox.as_ref() {
        sb.sync_out();
    }
    outcome
}

/// Sandbox-aware variant. `printer exec` shares one [`ActiveSandbox`] across
/// run + review by calling this directly, so the VM lifecycle is created once
/// per exec rather than per phase.
pub async fn review_with_sandbox(
    args: ReviewArgs,
    sandbox: Option<&ActiveSandbox>,
) -> Result<ReviewOutcome> {
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

    if let Some(sb) = sandbox {
        eprintln!(
            "[printer] dispatching review agent inside sandbox driver `{}` (handle: {})",
            sb.plugin(),
            sb.handle()
        );
    }
    let wrapper = sandbox.map(|s| s.enter_template());
    let acp = crate::agents::resolve_acp_launch(
        &args.agent,
        args.acp_bin.as_deref(),
        &args.acp_args,
    )?;
    let agent = AgentInvocation {
        kind: args.agent.clone(),
        model: args.model.as_deref(),
        cwd: cwd_ref,
        permission_mode: &args.permission_mode,
        command_wrapper: wrapper.as_deref(),
        verbose: args.verbose,
        acp_bin: acp.bin.as_deref(),
        acp_args: acp.args.as_slice(),
        acp_env: &acp.env,
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

    let followups_cwd = cwd
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    match write_followups(&followups_cwd, &spec_abs, verdict, &report) {
        Ok(p) => eprintln!("[printer] follow-ups written to {}", p.display()),
        Err(e) => eprintln!("[printer] failed to persist follow-ups: {e}"),
    }

    Ok(ReviewOutcome {
        usage,
        verdict,
        report,
    })
}

fn acquire_sandbox(args: &ReviewArgs) -> Result<Option<ActiveSandbox>> {
    if args.no_sandbox {
        return Ok(None);
    }
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
    let cwd: PathBuf = match args.cwd.as_deref() {
        Some(p) => p
            .canonicalize()
            .with_context(|| format!("--cwd not found: {}", p.display()))?,
        None => std::env::current_dir()?,
    };
    let spec = args
        .spec
        .canonicalize()
        .with_context(|| format!("spec file not found: {}", args.spec.display()))?;
    Ok(Some(ActiveSandbox::create(
        merged,
        cwd,
        Some(spec),
        Some(cfg.sandbox.base_image.clone()),
    )?))
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
    fn extracts_followups_section() {
        let report = "## Verdict\nPARTIAL\n\n## Suggested follow-ups\n- add tests for X\n- handle empty input\n\n## Out-of-scope\n- big refactor\n";
        let body = extract_section(report, "## Suggested follow-ups").unwrap();
        assert!(body.contains("add tests for X"));
        assert!(body.contains("handle empty input"));
        assert!(!body.contains("big refactor"));
    }

    #[test]
    fn extract_missing_section_returns_none() {
        let report = "## Verdict\nPASS\n\n## Per-item\n- foo MET\n";
        assert!(extract_section(report, "## Suggested follow-ups").is_none());
    }

    #[test]
    fn extract_none_body_returns_none() {
        let report = "## Suggested follow-ups\nnone\n";
        assert!(extract_section(report, "## Suggested follow-ups").is_none());
    }

    #[test]
    fn extract_empty_body_returns_none() {
        let report = "## Suggested follow-ups\n\n## Next\nstuff\n";
        assert!(extract_section(report, "## Suggested follow-ups").is_none());
    }

    #[test]
    fn write_followups_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let spec = dir.path().join("specs").join("003-followups.md");
        std::fs::create_dir_all(spec.parent().unwrap()).unwrap();
        std::fs::write(&spec, "spec body").unwrap();
        let report = "## Verdict\nPARTIAL\n\n## Suggested follow-ups\n- thing one\n- thing two\n";
        let out = write_followups(dir.path(), &spec, Verdict::Partial, report).unwrap();
        assert_eq!(
            out,
            dir.path().join(".printer/followups/003-followups.md")
        );
        let body = std::fs::read_to_string(&out).unwrap();
        assert!(body.contains("Verdict: PARTIAL"));
        assert!(body.contains("- thing one"));
        assert!(body.contains("- thing two"));
    }

    #[test]
    fn write_followups_writes_none_when_section_missing() {
        let dir = tempfile::tempdir().unwrap();
        let spec = dir.path().join("foo.md");
        std::fs::write(&spec, "x").unwrap();
        let out =
            write_followups(dir.path(), &spec, Verdict::Pass, "## Verdict\nPASS\n").unwrap();
        let body = std::fs::read_to_string(&out).unwrap();
        assert!(body.contains("Verdict: PASS"));
        assert!(body.trim_end().ends_with("none"));
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
