use clap::{Parser, Subcommand, ValueEnum};
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "printer", about = "Drive a Claude/opencode session against a markdown spec")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Write a starter spec file in the canonical format.
    Init(InitArgs),
    /// Plan and execute a markdown spec.
    Run(RunArgs),
    /// Generate a detailed plan from a spec without executing any work.
    /// Writes a checkpoint to `.printer/plan.checkpoint` and lets the agent
    /// optionally ask the user clarifying questions before locking the plan in.
    Plan(PlanArgs),
    /// Review the working tree against the original spec.
    Review(ReviewArgs),
    /// Run-then-review in one shot, with crash-safe `--continue`.
    Exec(ExecArgs),
    /// Show the archive of completed execs (`.printer/history.json`).
    History(HistoryArgs),
    /// File-based task tracking (create / list / start / done / ...).
    #[command(subcommand_help_heading = "Task subcommands")]
    Task(crate::tasks::TaskArgs),
    /// Install a plugin into ~/.printer/plugins/.
    AddPlugin(crate::plugins::AddPluginArgs),
    /// List installed plugins.
    Plugins,
    /// Inspect the lifecycle hooks installed plugins have registered.
    #[command(subcommand_help_heading = "Hook subcommands")]
    Hooks(HooksArgs),
    /// Forward to an installed plugin: `printer <plugin> <args>...`.
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[derive(clap::Args, Debug)]
pub struct InitArgs {
    /// Path to write the spec template. Defaults to `spec.md` in the cwd.
    pub path: Option<PathBuf>,

    /// Project title used in the spec's top-level heading.
    #[arg(long, short, default_value = "New Project")]
    pub title: String,

    /// Overwrite an existing file at the target path.
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(clap::Args, Debug)]
pub struct RunArgs {
    /// Path to the markdown spec / todo file.
    pub spec: PathBuf,

    /// Which agent to drive.
    #[arg(long, value_enum, default_value_t = AgentKind::Claude)]
    pub agent: AgentKind,

    /// Override the model (passed through to the agent).
    #[arg(long)]
    pub model: Option<String>,

    /// Hard cap on driver turns (excluding the bootstrap turn).
    #[arg(long, default_value_t = 40)]
    pub max_turns: u32,

    /// Cumulative input tokens at which we rotate to a fresh session.
    #[arg(long, default_value_t = 150_000)]
    pub compact_at: u64,

    /// Working directory for the child agent process.
    #[arg(long)]
    pub cwd: Option<PathBuf>,

    /// Permission mode passed to the child agent. Defaults to bypassPermissions
    /// because there is no human at the keyboard to approve prompts during a
    /// non-interactive driver run. Set to a stricter value if you want
    /// approvals to block the run.
    #[arg(long, default_value = "bypassPermissions")]
    pub permission_mode: String,

    /// Show a live spinner and periodic heartbeats so you can see the agent
    /// is still working during long turns.
    #[arg(long, short, default_value_t = false)]
    pub verbose: bool,

    /// Skip auto-spawning a `codegraph watch` daemon for the run. By default
    /// printer launches one (if `codegraph` is installed) so the index stays
    /// fresh as the agent edits files.
    #[arg(long, default_value_t = false)]
    pub no_codegraph_watch: bool,

    /// Skip the "no plugins installed" interactive check. Use this in CI where
    /// stdin is not a terminal and the run is intentionally plugin-free.
    #[arg(long, default_value_t = false)]
    pub skip_plugin_check: bool,
}

#[derive(clap::Args, Debug)]
pub struct PlanArgs {
    /// Path to the markdown spec / todo file.
    pub spec: PathBuf,

    /// Which agent to drive.
    #[arg(long, value_enum, default_value_t = AgentKind::Claude)]
    pub agent: AgentKind,

    /// Override the model.
    #[arg(long)]
    pub model: Option<String>,

    /// Working directory for the child agent process.
    #[arg(long)]
    pub cwd: Option<PathBuf>,

    /// Permission mode passed to the child agent.
    #[arg(long, default_value = "bypassPermissions")]
    pub permission_mode: String,

    /// Show a live spinner / heartbeats during the planning turn.
    #[arg(long, short, default_value_t = false)]
    pub verbose: bool,

    /// Skip the optional question/answer round even if the agent asks.
    /// Useful in CI / non-interactive contexts.
    #[arg(long, default_value_t = false)]
    pub no_questions: bool,

    /// Maximum number of question/answer rounds before the plan is forced to
    /// finalize.
    #[arg(long, default_value_t = 3)]
    pub max_question_rounds: u32,
}

#[derive(clap::Args, Debug)]
pub struct ReviewArgs {
    /// Path to the markdown spec the implementation was driven from.
    pub spec: PathBuf,

    /// Which agent to drive.
    #[arg(long, value_enum, default_value_t = AgentKind::Claude)]
    pub agent: AgentKind,

    /// Override the model.
    #[arg(long)]
    pub model: Option<String>,

    /// Git ref to diff against. Defaults to detected base (HEAD~ or main).
    #[arg(long)]
    pub base: Option<String>,

    /// Working directory for the child agent process.
    #[arg(long)]
    pub cwd: Option<PathBuf>,

    /// If set, also write the review report to this path.
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Permission mode passed to the child agent. Review is mostly read-only
    /// (`git diff`, file reads) so the default is `bypassPermissions` to keep
    /// the run non-interactive.
    #[arg(long, default_value = "bypassPermissions")]
    pub permission_mode: String,

    /// Make a skill available to the review agent. Accepts a path to a
    /// `SKILL.md`, a single skill directory, or a parent directory of skill
    /// directories (e.g. `.claude/skills/`). Repeatable. If omitted,
    /// `.claude/skills/` in the agent cwd is auto-discovered.
    #[arg(long = "skill", value_name = "PATH")]
    pub skills: Vec<PathBuf>,

    /// Show a live spinner and periodic heartbeats during the review turn.
    #[arg(long, short, default_value_t = false)]
    pub verbose: bool,
}

#[derive(clap::Args, Debug)]
pub struct ExecArgs {
    /// Path to the markdown spec / todo file. Optional with `--continue`
    /// (the spec path is then read from `.printer/exec.json`).
    pub spec: Option<PathBuf>,

    /// Resume a previous `printer exec` from `.printer/exec.json` instead of
    /// starting fresh. If the previous run finished cleanly, jumps to review.
    /// If the previous review finished cleanly, exits without doing anything.
    #[arg(long = "continue", default_value_t = false)]
    pub r#continue: bool,

    /// Which agent to drive.
    #[arg(long, value_enum, default_value_t = AgentKind::Claude)]
    pub agent: AgentKind,

    /// Override the model (passed through to the agent).
    #[arg(long)]
    pub model: Option<String>,

    /// Working directory for the child agent process.
    #[arg(long)]
    pub cwd: Option<PathBuf>,

    /// Permission mode passed to the child agent.
    #[arg(long, default_value = "bypassPermissions")]
    pub permission_mode: String,

    /// Live spinner / heartbeats during long turns.
    #[arg(long, short, default_value_t = false)]
    pub verbose: bool,

    // --- run-phase only ---
    /// Hard cap on driver turns during the run phase (excluding bootstrap).
    #[arg(long, default_value_t = 40)]
    pub max_turns: u32,

    /// Cumulative input tokens at which the run phase rotates the session.
    #[arg(long, default_value_t = 150_000)]
    pub compact_at: u64,

    // --- review-phase only ---
    /// Git ref to diff against during review. Defaults to detected base
    /// (`main` → `master` → `HEAD~1`).
    #[arg(long)]
    pub base: Option<String>,

    /// If set, also write the review report to this path.
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Make a skill available to the review agent. Same semantics as
    /// `printer review --skill`. Repeatable.
    #[arg(long = "skill", value_name = "PATH")]
    pub skills: Vec<PathBuf>,

    /// Skip auto-spawning a `codegraph watch` daemon for the duration of
    /// the exec (run + review). By default printer launches one if
    /// `codegraph` is installed.
    #[arg(long, default_value_t = false)]
    pub no_codegraph_watch: bool,

    /// Skip the "no plugins installed" interactive check. Use this in CI where
    /// stdin is not a terminal and the run is intentionally plugin-free.
    #[arg(long, default_value_t = false)]
    pub skip_plugin_check: bool,

    /// Maximum number of review cycles (review → fix → re-review). Each
    /// non-PASS verdict triggers a fix pass and another review, so this is
    /// the cap on round-trips before exec gives up. Defaults to
    /// `DEFAULT_MAX_REVIEW_PASSES` in `exec.rs` (currently 3). Set to 1 to
    /// disable the cycle entirely (single review, no fix pass).
    #[arg(long)]
    pub max_review_passes: Option<u32>,
}

#[derive(clap::Args, Debug)]
pub struct HistoryArgs {
    /// Working directory containing `.printer/history.json`.
    #[arg(long)]
    pub cwd: Option<PathBuf>,

    /// Emit the raw JSON instead of a human-readable summary.
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(clap::Args, Debug)]
pub struct HooksArgs {
    #[command(subcommand)]
    pub command: HooksCommand,
}

#[derive(Subcommand, Debug)]
pub enum HooksCommand {
    /// List every hook contributed by every installed plugin.
    List(HooksListArgs),
}

#[derive(clap::Args, Debug)]
pub struct HooksListArgs {
    /// Filter to a single event (e.g. `after_review`).
    #[arg(long)]
    pub event: Option<String>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum AgentKind {
    Claude,
    Opencode,
}
