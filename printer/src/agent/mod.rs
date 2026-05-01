pub mod acp;

use crate::cli::AgentKind;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use tokio::process::Command;
use uuid::Uuid;

/// Per-turn token breakdown, normalized across agents.
#[derive(Debug, Default, Clone, Copy)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

impl TokenUsage {
    /// Sum of all input-side tokens (new input + cache creation + cache read).
    pub fn input_total(&self) -> u64 {
        self.input_tokens + self.cache_creation_input_tokens + self.cache_read_input_tokens
    }

    /// Grand total: input-side + output.
    pub fn grand_total(&self) -> u64 {
        self.input_total() + self.output_tokens
    }

    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_creation_input_tokens += other.cache_creation_input_tokens;
        self.cache_read_input_tokens += other.cache_read_input_tokens;
    }
}

impl fmt::Display for TokenUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} total (input: {} new + {} cache-write + {} cache-read; output: {})",
            self.grand_total(),
            self.input_tokens,
            self.cache_creation_input_tokens,
            self.cache_read_input_tokens,
            self.output_tokens,
        )
    }
}

/// Outcome of a single turn, normalized across agents.
#[derive(Debug, Default, Clone)]
pub struct TurnOutcome {
    pub result_text: String,
    pub usage: TokenUsage,
}

impl TurnOutcome {
    /// Convenience accessor for the input-side total used by compaction logic.
    pub fn input_tokens(&self) -> u64 {
        self.usage.input_total()
    }
}

/// Parsed shape of `claude --print --output-format json`.
#[derive(Debug, Deserialize)]
struct ClaudeJsonResult {
    #[serde(default)]
    result: String,
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

#[derive(Debug, Default, Deserialize)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
}

pub struct AgentInvocation<'a> {
    pub kind: AgentKind,
    pub model: Option<&'a str>,
    pub cwd: Option<&'a Path>,
    pub permission_mode: &'a str,
    /// If set, the constructed agent command is shell-quoted and substituted
    /// for `{child}` in this template, then run via `sh -c`. Used by the
    /// sandbox driver to dispatch the agent inside a VM (see `drivers.rs`).
    pub command_wrapper: Option<&'a str>,
    /// Launch command for an ACP agent server. Required when `kind` is
    /// `AgentKind::Acp`. Ignored for one-shot backends. When the user picks
    /// a plugin-contributed agent (`--agent acp:<name>`) the call site
    /// resolves the name to a command via `crate::agents::AgentSet` before
    /// constructing this struct, so the ACP transport stays plugin-agnostic.
    pub acp_bin: Option<&'a str>,
    /// Extra args appended to the ACP server invocation.
    pub acp_args: &'a [String],
    /// Environment to pass to the spawned ACP server child. Empty for the
    /// one-shot backends.
    pub acp_env: &'a BTreeMap<String, String>,
}

impl<'a> AgentInvocation<'a> {
    /// Build a fresh-session command (sets the session id).
    pub fn bootstrap(&self, session_id: &Uuid, prompt: &str) -> Command {
        let argv = match &self.kind {
            AgentKind::Claude => self.claude_argv(Some(session_id), None, prompt),
            AgentKind::Opencode => self.opencode_argv(Some(session_id), false, prompt),
            AgentKind::Acp { .. } => panic!(
                "AgentInvocation::bootstrap() must not be called for ACP agents; use the ACP path in Session::turn"
            ),
        };
        self.build_command(argv)
    }

    /// Build a resume-session command.
    pub fn resume(&self, session_id: &Uuid, prompt: &str) -> Command {
        let argv = match &self.kind {
            AgentKind::Claude => self.claude_argv(None, Some(session_id), prompt),
            AgentKind::Opencode => self.opencode_argv(Some(session_id), true, prompt),
            AgentKind::Acp { .. } => panic!(
                "AgentInvocation::resume() must not be called for ACP agents; use the ACP path in Session::turn"
            ),
        };
        self.build_command(argv)
    }

    fn build_command(&self, argv: Vec<String>) -> Command {
        if let Some(template) = self.command_wrapper {
            let quoted = crate::drivers::shell_quote_argv(&argv);
            let resolved = template.replace("{child}", &quoted);
            let mut cmd = Command::new("sh");
            cmd.arg("-c").arg(&resolved);
            if let Some(cwd) = self.cwd {
                cmd.current_dir(cwd);
            }
            cmd
        } else {
            let mut cmd = Command::new(&argv[0]);
            cmd.args(&argv[1..]);
            if let Some(cwd) = self.cwd {
                cmd.current_dir(cwd);
            }
            cmd
        }
    }

    fn claude_argv(&self, session_id: Option<&Uuid>, resume: Option<&Uuid>, prompt: &str) -> Vec<String> {
        let mut v: Vec<String> = vec![
            "claude".into(),
            "--print".into(),
            "--output-format".into(),
            "json".into(),
            "--permission-mode".into(),
            self.permission_mode.to_string(),
        ];
        if let Some(id) = session_id {
            v.push("--session-id".into());
            v.push(id.to_string());
        }
        if let Some(id) = resume {
            v.push("--resume".into());
            v.push(id.to_string());
        }
        if let Some(model) = self.model {
            v.push("--model".into());
            v.push(model.to_string());
        }
        v.push(prompt.to_string());
        v
    }

    fn opencode_argv(&self, session_id: Option<&Uuid>, resume: bool, prompt: &str) -> Vec<String> {
        let mut v: Vec<String> = vec![
            "opencode".into(),
            "run".into(),
            "--prompt".into(),
            prompt.to_string(),
        ];
        if let Some(id) = session_id {
            v.push("--session".into());
            v.push(id.to_string());
        }
        if resume {
            v.push("--continue".into());
        }
        if let Some(model) = self.model {
            v.push("--model".into());
            v.push(model.to_string());
        }
        v
    }

    /// Parse stdout from a completed agent process into a normalized outcome.
    pub fn parse_outcome(&self, stdout: String, fallback_session: &Uuid) -> anyhow::Result<TurnOutcome> {
        match &self.kind {
            AgentKind::Claude => parse_claude(stdout, fallback_session),
            AgentKind::Opencode => parse_opencode(stdout, fallback_session),
            AgentKind::Acp { .. } => parse_opencode(stdout, fallback_session),
        }
    }
}

fn parse_claude(stdout: String, _fallback_session: &Uuid) -> anyhow::Result<TurnOutcome> {
    let parsed: ClaudeJsonResult = serde_json::from_str(stdout.trim())
        .map_err(|e| anyhow::anyhow!("failed to parse claude JSON output: {e}\n--- stdout ---\n{stdout}"))?;
    let raw = parsed.usage.unwrap_or_default();
    let usage = TokenUsage {
        input_tokens: raw.input_tokens,
        output_tokens: raw.output_tokens,
        cache_creation_input_tokens: raw.cache_creation_input_tokens,
        cache_read_input_tokens: raw.cache_read_input_tokens,
    };
    Ok(TurnOutcome {
        result_text: parsed.result,
        usage,
    })
}

fn parse_opencode(stdout: String, _fallback_session: &Uuid) -> anyhow::Result<TurnOutcome> {
    Ok(TurnOutcome {
        result_text: stdout,
        usage: TokenUsage::default(),
    })
}
