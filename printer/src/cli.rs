use clap::{Parser, Subcommand, ValueEnum};
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
    /// Review the working tree against the original spec.
    Review(ReviewArgs),
    /// File-based task tracking (create / list / start / done / ...).
    #[command(subcommand_help_heading = "Task subcommands")]
    Task(crate::tasks::TaskArgs),
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

    /// Show a live spinner and periodic heartbeats during the review turn.
    #[arg(long, short, default_value_t = false)]
    pub verbose: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum AgentKind {
    Claude,
    Opencode,
}
