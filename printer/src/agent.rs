use crate::cli::AgentKind;
use serde::Deserialize;
use std::path::Path;
use tokio::process::Command;
use uuid::Uuid;

/// Outcome of a single turn, normalized across agents.
#[derive(Debug, Default, Clone)]
pub struct TurnOutcome {
    pub result_text: String,
    pub input_tokens: u64,
}

/// Parsed shape of `claude --print --output-format json`.
#[derive(Debug, Deserialize)]
struct ClaudeJsonResult {
    #[serde(default)]
    result: String,
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: u64,
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
}

impl<'a> AgentInvocation<'a> {
    /// Build a fresh-session command (sets the session id).
    pub fn bootstrap(&self, session_id: &Uuid, prompt: &str) -> Command {
        match self.kind {
            AgentKind::Claude => self.claude_cmd(Some(session_id), None, prompt),
            AgentKind::Opencode => self.opencode_cmd(Some(session_id), false, prompt),
        }
    }

    /// Build a resume-session command.
    pub fn resume(&self, session_id: &Uuid, prompt: &str) -> Command {
        match self.kind {
            AgentKind::Claude => self.claude_cmd(None, Some(session_id), prompt),
            AgentKind::Opencode => self.opencode_cmd(Some(session_id), true, prompt),
        }
    }

    fn claude_cmd(&self, session_id: Option<&Uuid>, resume: Option<&Uuid>, prompt: &str) -> Command {
        let mut cmd = Command::new("claude");
        cmd.arg("--print")
            .arg("--output-format")
            .arg("json")
            .arg("--permission-mode")
            .arg(self.permission_mode);
        if let Some(id) = session_id {
            cmd.arg("--session-id").arg(id.to_string());
        }
        if let Some(id) = resume {
            cmd.arg("--resume").arg(id.to_string());
        }
        if let Some(model) = self.model {
            cmd.arg("--model").arg(model);
        }
        if let Some(cwd) = self.cwd {
            cmd.current_dir(cwd);
        }
        cmd.arg(prompt);
        cmd
    }

    fn opencode_cmd(&self, session_id: Option<&Uuid>, resume: bool, prompt: &str) -> Command {
        let mut cmd = Command::new("opencode");
        cmd.arg("run").arg("--prompt").arg(prompt);
        if let Some(id) = session_id {
            cmd.arg("--session").arg(id.to_string());
        }
        if resume {
            cmd.arg("--continue");
        }
        if let Some(model) = self.model {
            cmd.arg("--model").arg(model);
        }
        if let Some(cwd) = self.cwd {
            cmd.current_dir(cwd);
        }
        cmd
    }

    /// Parse stdout from a completed agent process into a normalized outcome.
    pub fn parse_outcome(&self, stdout: String, fallback_session: &Uuid) -> anyhow::Result<TurnOutcome> {
        match self.kind {
            AgentKind::Claude => parse_claude(stdout, fallback_session),
            AgentKind::Opencode => parse_opencode(stdout, fallback_session),
        }
    }
}

fn parse_claude(stdout: String, _fallback_session: &Uuid) -> anyhow::Result<TurnOutcome> {
    let parsed: ClaudeJsonResult = serde_json::from_str(stdout.trim())
        .map_err(|e| anyhow::anyhow!("failed to parse claude JSON output: {e}\n--- stdout ---\n{stdout}"))?;
    let usage = parsed.usage.unwrap_or(ClaudeUsage {
        input_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
    });
    let input_tokens = usage.input_tokens
        + usage.cache_creation_input_tokens
        + usage.cache_read_input_tokens;
    Ok(TurnOutcome {
        result_text: parsed.result,
        input_tokens,
    })
}

fn parse_opencode(stdout: String, _fallback_session: &Uuid) -> anyhow::Result<TurnOutcome> {
    Ok(TurnOutcome {
        result_text: stdout,
        input_tokens: 0,
    })
}
