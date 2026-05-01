//! Agent Client Protocol (ACP) transport.
//!
//! Holds a long-lived `tokio::process::Child` running an ACP server (e.g.
//! `claude-code-acp`, Poolside) and speaks newline-delimited JSON-RPC 2.0 over
//! its stdio. Unlike the one-shot `claude --print` / `opencode run` backends,
//! the same child persists across `Session::turn` calls — `session/new` is
//! sent once on bootstrap and `session/prompt` is sent per turn.
//!
//! T-017 scope: blocking-turn semantics. Streaming-to-stderr, Ctrl-C
//! cancellation via `session/cancel`, and permission-mode mapping are tracked
//! separately on T-020.
//!
//! See https://agentclientprotocol.com for the wire spec.

use crate::agent::TurnOutcome;
use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, mpsc, oneshot};

/// JSON-RPC response payload returned to a pending request waiter.
#[derive(Debug)]
pub enum RpcResult {
    Ok(Value),
    Err(Value),
}

/// One server-pushed `session/update` notification.
#[derive(Debug, Clone)]
pub struct SessionUpdate {
    pub session_id: String,
    pub update: Value,
}

/// Long-lived client for one ACP server child.
pub struct AcpClient {
    child: Child,
    writer: Arc<Mutex<ChildStdin>>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<RpcResult>>>>,
    next_id: AtomicU64,
    notifications: Mutex<mpsc::UnboundedReceiver<SessionUpdate>>,
    _reader_handle: tokio::task::JoinHandle<()>,
    session_id: Mutex<Option<String>>,
}

impl AcpClient {
    /// Spawn the ACP server. `bin` and `args` are the launch command. If
    /// `command_wrapper` is `Some`, the argv is shell-quoted and substituted
    /// for `{child}` in the wrapper template, then run via `sh -c` — same
    /// machinery the one-shot backends use to dispatch into a sandbox.
    pub async fn spawn(
        bin: &str,
        args: &[String],
        cwd: Option<&Path>,
        command_wrapper: Option<&str>,
        env: &BTreeMap<String, String>,
    ) -> Result<Self> {
        let mut argv: Vec<String> = Vec::with_capacity(args.len() + 1);
        argv.push(bin.to_string());
        for a in args {
            argv.push(a.clone());
        }

        let mut cmd = if let Some(template) = command_wrapper {
            let quoted = crate::drivers::shell_quote_argv(&argv);
            let resolved = template.replace("{child}", &quoted);
            let mut c = Command::new("sh");
            c.arg("-c").arg(&resolved);
            c
        } else {
            let mut c = Command::new(&argv[0]);
            c.args(&argv[1..]);
            c
        };
        if let Some(c) = cwd {
            cmd.current_dir(c);
        }
        for (k, v) in env {
            cmd.env(k, v);
        }
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        #[cfg(unix)]
        cmd.process_group(0);

        let mut child = cmd
            .spawn()
            .with_context(|| format!("spawning ACP server `{bin}`"))?;
        let stdin = child.stdin.take().context("ACP child has no stdin pipe")?;
        let stdout = child.stdout.take().context("ACP child has no stdout pipe")?;
        let stderr = child.stderr.take();

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<RpcResult>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (notif_tx, notif_rx) = mpsc::unbounded_channel::<SessionUpdate>();

        let pending_clone = pending.clone();
        let reader_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let value: Value = match serde_json::from_str(trimmed) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[acp] dropping unparseable line ({e}): {trimmed}");
                        continue;
                    }
                };
                dispatch_message(value, &pending_clone, &notif_tx).await;
            }
        });

        // Forward stderr verbatim — surfaces server diagnostics to the user.
        if let Some(err) = stderr {
            tokio::spawn(async move {
                let mut reader = BufReader::new(err).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    eprintln!("[acp:server] {line}");
                }
            });
        }

        Ok(Self {
            child,
            writer: Arc::new(Mutex::new(stdin)),
            pending,
            next_id: AtomicU64::new(1),
            notifications: Mutex::new(notif_rx),
            _reader_handle: reader_handle,
            session_id: Mutex::new(None),
        })
    }

    /// Send `initialize` per ACP spec. Must succeed before `session/new`.
    pub async fn initialize(&self) -> Result<Value> {
        // protocolVersion is the integer schema version; v1 of the spec uses 1.
        let params = json!({
            "protocolVersion": 1,
            "clientCapabilities": { "fs": { "readTextFile": false, "writeTextFile": false } },
        });
        self.request("initialize", params).await
    }

    /// Send `session/new` and remember the returned `sessionId`.
    pub async fn session_new(&self, cwd: &Path) -> Result<String> {
        let params = json!({
            "cwd": cwd.to_string_lossy(),
            "mcpServers": [],
        });
        let result = self.request("session/new", params).await?;
        let id = result
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("session/new response missing sessionId: {result}"))?
            .to_string();
        *self.session_id.lock().await = Some(id.clone());
        Ok(id)
    }

    /// Send a single `session/prompt` and block until the request resolves.
    /// Drains any `session/update` notifications that arrive in the meantime
    /// and concatenates their text content blocks into `result_text`.
    ///
    /// Token usage is currently not surfaced (ACP doesn't standardize it; some
    /// servers include it on the final `agent_message_chunk`'s metadata).
    /// `TokenUsage::default()` is returned and the compaction trigger will
    /// behave as if the session never crosses the threshold — that's fine for
    /// T-017's blocking-turn scope; T-020 can wire usage in.
    pub async fn prompt_blocking(&self, session_id: &str, prompt: &str) -> Result<TurnOutcome> {
        let params = json!({
            "sessionId": session_id,
            "prompt": [{ "type": "text", "text": prompt }],
        });

        let result_fut = self.request("session/prompt", params);
        tokio::pin!(result_fut);

        let mut text = String::new();
        let mut notifications = self.notifications.lock().await;
        loop {
            tokio::select! {
                biased;
                rpc = &mut result_fut => {
                    let value = rpc?;
                    // Drain any remaining notifications already buffered.
                    while let Ok(update) = notifications.try_recv() {
                        if update.session_id == session_id {
                            collect_update_text(&update.update, &mut text);
                        }
                    }
                    let _ = value; // stopReason / metadata not surfaced yet
                    return Ok(TurnOutcome { result_text: text, ..Default::default() });
                }
                Some(update) = notifications.recv() => {
                    if update.session_id == session_id {
                        collect_update_text(&update.update, &mut text);
                    }
                }
            }
        }
    }

    /// Send a JSON-RPC 2.0 request and await its response.
    async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        let frame = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let line = serde_json::to_string(&frame)?;
        {
            let mut w = self.writer.lock().await;
            w.write_all(line.as_bytes()).await?;
            w.write_all(b"\n").await?;
            w.flush().await?;
        }

        match rx.await {
            Ok(RpcResult::Ok(v)) => Ok(v),
            Ok(RpcResult::Err(e)) => Err(anyhow!("ACP {method} failed: {e}")),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(anyhow!("ACP {method} response channel dropped (server exited?)"))
            }
        }
    }

    /// Issue `session/cancel` then `start_kill` the child. Best-effort — used
    /// on Drop and on Ctrl-C (the latter is wired in T-020).
    #[allow(dead_code)]
    pub async fn shutdown(mut self) {
        if let Some(id) = self.session_id.lock().await.clone() {
            let _ = self
                .request_no_wait("session/cancel", json!({ "sessionId": id }))
                .await;
        }
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
    }

    #[allow(dead_code)]
    async fn request_no_wait(&self, method: &str, params: Value) -> Result<()> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let frame = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let line = serde_json::to_string(&frame)?;
        let mut w = self.writer.lock().await;
        w.write_all(line.as_bytes()).await?;
        w.write_all(b"\n").await?;
        w.flush().await?;
        Ok(())
    }
}

impl Drop for AcpClient {
    fn drop(&mut self) {
        // Best-effort: the async shutdown() above is preferred, but if the
        // client is dropped without it, kill the child synchronously to avoid
        // an orphan.
        let _ = self.child.start_kill();
    }
}

async fn dispatch_message(
    value: Value,
    pending: &Arc<Mutex<HashMap<u64, oneshot::Sender<RpcResult>>>>,
    notif_tx: &mpsc::UnboundedSender<SessionUpdate>,
) {
    // Response: has `id` and (`result` or `error`), no `method`.
    if value.get("method").is_none() {
        let id = value.get("id").and_then(|v| v.as_u64());
        if let Some(id) = id {
            let mut map = pending.lock().await;
            if let Some(tx) = map.remove(&id) {
                if let Some(err) = value.get("error") {
                    let _ = tx.send(RpcResult::Err(err.clone()));
                } else {
                    let result = value.get("result").cloned().unwrap_or(Value::Null);
                    let _ = tx.send(RpcResult::Ok(result));
                }
            }
        }
        return;
    }

    // Notification: has `method`, may or may not have `id`. ACP server-pushed
    // updates use `session/update` with a `sessionId` + `update` payload.
    let method = value
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if method == "session/update" {
        let params = value.get("params").cloned().unwrap_or(Value::Null);
        let session_id = params
            .get("sessionId")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let update = params.get("update").cloned().unwrap_or(Value::Null);
        let _ = notif_tx.send(SessionUpdate { session_id, update });
    }
    // Other server→client requests (fs/read_text_file, permission prompts,
    // etc.) are not handled in T-017 — they would block the server forever
    // when triggered. Document and revisit in T-020.
}

/// Walk an `update` payload and append any text content found. ACP content
/// blocks may appear as `agent_message_chunk` / `agent_thought_chunk` /
/// `tool_call` / etc.; we collect anything with a `text` field for now.
fn collect_update_text(update: &Value, sink: &mut String) {
    // Common shapes:
    //   { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "..." } }
    //   { "sessionUpdate": "agent_message_chunk", "content": [{ "type": "text", "text": "..." }] }
    if let Some(content) = update.get("content") {
        collect_content_text(content, sink);
    }
}

fn collect_content_text(content: &Value, sink: &mut String) {
    match content {
        Value::Array(items) => {
            for item in items {
                collect_content_text(item, sink);
            }
        }
        Value::Object(map) => {
            if map.get("type").and_then(|v| v.as_str()) == Some("text")
                && let Some(t) = map.get("text").and_then(|v| v.as_str())
            {
                sink.push_str(t);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dispatches_response_to_pending_waiter() {
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<RpcResult>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (notif_tx, _notif_rx) = mpsc::unbounded_channel::<SessionUpdate>();
        let (tx, rx) = oneshot::channel();
        pending.lock().await.insert(7, tx);

        let msg = json!({"jsonrpc":"2.0","id":7,"result":{"hello":"world"}});
        dispatch_message(msg, &pending, &notif_tx).await;

        let got = rx.await.unwrap();
        match got {
            RpcResult::Ok(v) => assert_eq!(v, json!({"hello":"world"})),
            RpcResult::Err(e) => panic!("expected ok, got err: {e}"),
        }
    }

    #[tokio::test]
    async fn dispatches_error_response() {
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<RpcResult>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (notif_tx, _notif_rx) = mpsc::unbounded_channel::<SessionUpdate>();
        let (tx, rx) = oneshot::channel();
        pending.lock().await.insert(3, tx);

        let msg = json!({"jsonrpc":"2.0","id":3,"error":{"code":-32601,"message":"nope"}});
        dispatch_message(msg, &pending, &notif_tx).await;

        match rx.await.unwrap() {
            RpcResult::Err(v) => assert_eq!(v["code"], -32601),
            RpcResult::Ok(_) => panic!("expected err"),
        }
    }

    #[tokio::test]
    async fn forwards_session_update_notification() {
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<RpcResult>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (notif_tx, mut notif_rx) = mpsc::unbounded_channel::<SessionUpdate>();

        let msg = json!({
            "jsonrpc":"2.0",
            "method":"session/update",
            "params":{"sessionId":"sess-1","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"hi"}}}
        });
        dispatch_message(msg, &pending, &notif_tx).await;

        let got = notif_rx.recv().await.unwrap();
        assert_eq!(got.session_id, "sess-1");
        let mut buf = String::new();
        collect_update_text(&got.update, &mut buf);
        assert_eq!(buf, "hi");
    }

    #[test]
    fn collect_content_text_handles_array_and_object() {
        let mut buf = String::new();
        let v = json!([
            {"type":"text","text":"hello "},
            {"type":"text","text":"world"},
            {"type":"image","data":"…"}
        ]);
        collect_content_text(&v, &mut buf);
        assert_eq!(buf, "hello world");
    }
}
