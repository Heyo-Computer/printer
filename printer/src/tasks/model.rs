use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Open,
    InProgress,
    Blocked,
    Done,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::InProgress => "in_progress",
            Status::Blocked => "blocked",
            Status::Done => "done",
        }
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Frontmatter portion of a task file. Body lives separately and is
/// round-tripped as opaque text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMeta {
    pub id: String,
    pub title: String,
    pub status: Status,
    #[serde(default = "default_priority")]
    pub priority: u8,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub blocked_reason: String,
    /// Stable identifier linking this task back to a spec item. Empty when
    /// the task was created by hand (`printer task create`) rather than from
    /// a spec sync. Used by `printer run` to be idempotent on re-sync.
    #[serde(default)]
    pub spec_anchor: String,
}

fn default_priority() -> u8 {
    3
}

/// Full in-memory task: frontmatter + opaque body markdown.
#[derive(Debug, Clone)]
pub struct Task {
    pub meta: TaskMeta,
    pub body: String,
}

impl Task {
    pub fn new(id: String, title: String) -> Self {
        let now = now_iso();
        Self {
            meta: TaskMeta {
                id,
                title,
                status: Status::Open,
                priority: 3,
                created_at: now.clone(),
                updated_at: now,
                owner: String::new(),
                labels: Vec::new(),
                depends_on: Vec::new(),
                blocked_reason: String::new(),
                spec_anchor: String::new(),
            },
            body: String::new(),
        }
    }

    pub fn touch(&mut self) {
        self.meta.updated_at = now_iso();
    }
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Format an integer id index as the canonical `T-NNN` form.
pub fn format_id(n: u32) -> String {
    format!("T-{n:03}")
}

/// Parse a task id like "T-007" or "t-7" into its numeric index.
pub fn parse_id(s: &str) -> Result<u32> {
    let rest = s
        .strip_prefix("T-")
        .or_else(|| s.strip_prefix("t-"))
        .ok_or_else(|| anyhow!("bad task id `{s}`: expected form T-NNN"))?;
    rest.parse::<u32>()
        .with_context(|| format!("bad task id `{s}`: numeric part is not a number"))
}

/// Render a Task back to the on-disk file format.
pub fn to_file_string(task: &Task) -> Result<String> {
    let fm = toml::to_string(&task.meta).context("serializing frontmatter")?;
    let body = if task.body.is_empty() || task.body.starts_with('\n') {
        task.body.clone()
    } else {
        format!("\n{}", task.body)
    };
    Ok(format!("+++\n{fm}+++{body}"))
}

/// Parse a file's contents into a Task.
pub fn from_file_string(raw: &str) -> Result<Task> {
    let after_open = raw
        .strip_prefix("+++\n")
        .or_else(|| raw.strip_prefix("+++\r\n"))
        .ok_or_else(|| anyhow!("task file does not start with `+++`"))?;
    let close_idx = after_open
        .find("\n+++")
        .ok_or_else(|| anyhow!("task file missing closing `+++`"))?;
    let fm_str = &after_open[..close_idx];
    let after_close = &after_open[close_idx + "\n+++".len()..];
    let body = after_close
        .strip_prefix('\n')
        .or_else(|| after_close.strip_prefix("\r\n"))
        .unwrap_or(after_close)
        .to_string();

    let meta: TaskMeta = toml::from_str(fm_str).context("parsing frontmatter")?;
    Ok(Task { meta, body })
}
