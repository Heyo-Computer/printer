use super::model::Status;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct TaskArgs {
    /// Override the tasks directory. Defaults to `<cwd>/.printer/tasks`.
    #[arg(long, global = true)]
    pub tasks_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: TaskCommand,
}

#[derive(Subcommand, Debug)]
pub enum TaskCommand {
    /// Create a new task.
    Create(CreateArgs),
    /// List tasks.
    List(ListArgs),
    /// Show a single task in full.
    Show(IdArgs),
    /// Print the ready queue (open tasks with all deps done).
    Ready,
    /// Claim a task (status -> in_progress).
    Start(StartArgs),
    /// Mark a task done.
    Done(DoneArgs),
    /// Mark a task blocked with a reason.
    Block(BlockArgs),
    /// Unblock a task (status -> open).
    Unblock(IdArgs),
    /// Release a claim (clear owner; status -> open).
    Release(IdArgs),
    /// Append a comment line under the `## Notes` section of a task.
    Comment(CommentArgs),
    /// Add or remove dependencies on a task.
    Depends(DependsArgs),
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    pub title: String,
    /// Description body. Use `-` to read from stdin.
    #[arg(long, short)]
    pub description: Option<String>,
    /// Priority 1 (highest) – 5 (lowest).
    #[arg(long, short, default_value_t = 3)]
    pub priority: u8,
    /// Comma-separated list of task ids this depends on.
    #[arg(long, value_delimiter = ',')]
    pub depends_on: Vec<String>,
    /// Comma-separated labels.
    #[arg(long, value_delimiter = ',')]
    pub labels: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long, value_enum)]
    pub status: Option<Status>,
    #[arg(long)]
    pub label: Option<String>,
    #[arg(long)]
    pub owner: Option<String>,
    /// Filter to tasks owned by the current user.
    #[arg(long, default_value_t = false)]
    pub mine: bool,
}

#[derive(Args, Debug)]
pub struct IdArgs {
    pub id: String,
}

#[derive(Args, Debug)]
pub struct StartArgs {
    pub id: String,
    /// Override the owner. Defaults to $USER.
    #[arg(long)]
    pub owner: Option<String>,
    /// Steamroll an existing owner (use after a crash to reclaim a task).
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct DoneArgs {
    pub id: String,
    /// Optional final note appended to the body.
    #[arg(long)]
    pub note: Option<String>,
}

#[derive(Args, Debug)]
pub struct BlockArgs {
    pub id: String,
    #[arg(long)]
    pub reason: String,
}

#[derive(Args, Debug)]
pub struct CommentArgs {
    pub id: String,
    pub text: String,
}

#[derive(Args, Debug)]
pub struct DependsArgs {
    pub id: String,
    /// Add these dependency ids.
    #[arg(long, value_delimiter = ',')]
    pub add: Vec<String>,
    /// Remove these dependency ids.
    #[arg(long, value_delimiter = ',')]
    pub remove: Vec<String>,
}
