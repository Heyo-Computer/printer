use crate::agent::acp::AcpClient;
use crate::agent::{AgentInvocation, TokenUsage, TurnOutcome};
use crate::cli::AgentKind;
use anyhow::{Context, Result};
use std::io::IsTerminal;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use uuid::Uuid;

pub struct Session<'a> {
    pub id: Uuid,
    pub turn_count: u32,
    /// Largest single-turn input-side token total seen in the *current* (not
    /// yet rotated) session. Used by the compaction trigger only.
    pub cumulative_input_tokens: u64,
    /// Sum of every turn's token usage for the lifetime of this Session,
    /// including across rotations. This is the operation-level total.
    pub usage_total: TokenUsage,
    pub agent: AgentInvocation<'a>,
    pub verbose: bool,
    fresh: bool,
    acp: Option<AcpClient>,
    acp_session_id: Option<String>,
}

impl<'a> Session<'a> {
    pub fn new(agent: AgentInvocation<'a>) -> Self {
        Self {
            id: Uuid::new_v4(),
            turn_count: 0,
            cumulative_input_tokens: 0,
            usage_total: TokenUsage::default(),
            agent,
            verbose: false,
            fresh: true,
            acp: None,
            acp_session_id: None,
        }
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Mint a new session id; the next turn will bootstrap a fresh session.
    /// Used for compaction-by-rotation.
    pub async fn rotate(&mut self) {
        self.id = Uuid::new_v4();
        self.fresh = true;
        self.cumulative_input_tokens = 0;
        // For ACP, shut down the existing client so the previous server's
        // process group is reaped before we drop it; otherwise the watcher
        // task would only fire when the parent process exits. Then re-null
        // so the next turn spawns a fresh server and starts a new
        // `session/new`. Mirrors what one-shot backends do implicitly by
        // spawning a new process per turn.
        if let Some(client) = self.acp.as_ref() {
            client.shutdown().await;
        }
        self.acp = None;
        self.acp_session_id = None;
    }

    pub async fn turn(&mut self, prompt: &str) -> Result<TurnOutcome> {
        if matches!(self.agent.kind, AgentKind::Acp { .. }) {
            return self.turn_acp(prompt).await;
        }
        let mut cmd = if self.fresh {
            self.agent.bootstrap(&self.id, prompt)
        } else {
            self.agent.resume(&self.id, prompt)
        };
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());
        // Put the child into its own process group so we can kill the whole
        // subtree on Ctrl-C — otherwise orphaned grandchildren can keep pipes
        // open and stall our cleanup readers indefinitely.
        #[cfg(unix)]
        cmd.process_group(0);

        let started = Instant::now();
        eprintln!(
            "[printer] turn {} starting (session {}{})",
            self.turn_count + 1,
            short(&self.id),
            if self.fresh { ", fresh" } else { ", resumed" }
        );

        let mut child = cmd.spawn().context("failed to spawn agent process")?;

        let stop = Arc::new(AtomicBool::new(false));
        let agent_active = Arc::new(AtomicBool::new(false));

        // Stream stderr from the child to our own stderr in real time so the
        // user can watch progress.
        let child_stderr = child.stderr.take();
        let verbose = self.verbose;
        let tty = std::io::stderr().is_terminal();
        let agent_active_clone = agent_active.clone();
        let stderr_task = tokio::spawn(async move {
            if let Some(stderr) = child_stderr {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    agent_active_clone.store(true, Ordering::Relaxed);
                    if verbose && tty {
                        // Clear any spinner line, then print the agent line.
                        eprint!("\r\x1b[2K");
                    }
                    eprintln!("[agent] {line}");
                }
            }
        });

        // Optional spinner / heartbeat task.
        let spinner_task = if self.verbose {
            let stop = stop.clone();
            let agent_active = agent_active.clone();
            Some(tokio::spawn(async move {
                heartbeat_loop(stop, agent_active, started, tty).await;
            }))
        } else {
            None
        };

        // Read stdout in chunks concurrently with watching for Ctrl-C and the
        // child's exit. read_to_string would block until the child closes its
        // stdout pipe (i.e. exits), starving the ctrl_c branch — so we drive
        // the read inside the select loop instead.
        let mut stdout = child
            .stdout
            .take()
            .context("child has no stdout pipe")?;
        let mut stdout_bytes: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 8192];
        let mut read_done = false;

        let status = loop {
            tokio::select! {
                biased;
                _ = tokio::signal::ctrl_c() => {
                    if verbose && tty { eprint!("\r\x1b[2K"); }
                    eprintln!("[printer] interrupt received; killing child agent");
                    stop.store(true, Ordering::Relaxed);
                    kill_subtree(&mut child);
                    let _ = child.wait().await;
                    // Don't await the helper tasks — orphaned grandchildren
                    // can keep their stdio pipes open after the immediate
                    // child dies, which would block stderr_task forever.
                    // Aborting is non-blocking and the runtime will reap them.
                    if let Some(t) = spinner_task { t.abort(); }
                    stderr_task.abort();
                    anyhow::bail!("interrupted by user");
                }
                read = stdout.read(&mut chunk), if !read_done => {
                    match read? {
                        0 => { read_done = true; }
                        n => { stdout_bytes.extend_from_slice(&chunk[..n]); }
                    }
                }
                wait = child.wait(), if read_done => {
                    break wait?;
                }
            }
        };

        stop.store(true, Ordering::Relaxed);
        if let Some(t) = spinner_task {
            let _ = t.await;
            if tty && self.verbose {
                eprint!("\r\x1b[2K");
            }
        }
        let _ = stderr_task.await;

        let stdout_buf = String::from_utf8_lossy(&stdout_bytes).into_owned();

        if !status.success() {
            anyhow::bail!(
                "agent exited with status {status}\n--- stdout ---\n{stdout_buf}"
            );
        }

        let outcome = self.agent.parse_outcome(stdout_buf, &self.id)?;
        self.turn_count += 1;
        self.cumulative_input_tokens =
            outcome.input_tokens().max(self.cumulative_input_tokens);
        self.usage_total.add(&outcome.usage);
        eprintln!(
            "[printer] turn {} done in {:.1}s (turn: {}; op total: {})",
            self.turn_count,
            started.elapsed().as_secs_f32(),
            outcome.usage,
            self.usage_total,
        );
        // Subsequent turns resume the same session id (claude requires a new
        // uuid for --session-id, so we keep using --resume from now on).
        self.fresh = false;
        Ok(outcome)
    }

    /// ACP-backed turn. Lazily spawns the long-lived server on the first call
    /// (and again after `rotate()`), then sends one blocking `session/prompt`.
    /// Token usage is not surfaced (T-020 follow-up).
    async fn turn_acp(&mut self, prompt: &str) -> Result<TurnOutcome> {
        let started = Instant::now();
        eprintln!(
            "[printer] turn {} starting (acp session {}{})",
            self.turn_count + 1,
            short(&self.id),
            if self.fresh { ", fresh" } else { ", resumed" }
        );

        if self.acp.is_none() {
            let bin = self.agent.acp_bin.ok_or_else(|| {
                anyhow::anyhow!(
                    "--agent acp requires --acp-bin <command> (or pick a plugin-contributed agent via --agent acp:<name>)"
                )
            })?;

            // Permission-mode is Claude-CLI shaped (`--permission-mode <mode>`).
            // ACP doesn't standardize a permission policy — the server
            // enforces its own. Surface the value as an env var hint
            // (`PRINTER_PERMISSION_MODE`) and mention the limitation once,
            // so users running `--agent acp:* --permission-mode acceptEdits`
            // don't assume printer is overriding the server.
            eprintln!(
                "[printer] acp: --permission-mode is advisory; the ACP server enforces its own policy (mode={})",
                self.agent.permission_mode
            );
            let mut child_env = self.agent.acp_env.clone();
            child_env.insert(
                "PRINTER_PERMISSION_MODE".to_string(),
                self.agent.permission_mode.to_string(),
            );

            // Per-step progress: bootstrap can stall in spawn (slow image
            // pull, sandbox init), initialize (ACP child blocked on the
            // protocol handshake), or session/new (server-side workdir
            // setup — the poolside EROFS regression manifested here). Each
            // step prints both a "starting" and "ok in Ns" line so the
            // exact hung phase is obvious from the log.
            let argv_label = format_argv(bin, self.agent.acp_args);
            let t = Instant::now();
            eprintln!("[printer] acp: spawning server `{argv_label}`");
            let client = AcpClient::spawn(
                bin,
                self.agent.acp_args,
                self.agent.cwd,
                self.agent.command_wrapper,
                &child_env,
            )
            .await
            .context("spawning ACP server")?;
            eprintln!(
                "[printer] acp: server spawned ({:.1}s); sending initialize",
                t.elapsed().as_secs_f32()
            );

            let t = Instant::now();
            with_progress("initialize", self.verbose, client.initialize())
                .await
                .context("ACP initialize failed")?;
            eprintln!(
                "[printer] acp: initialize ok ({:.1}s); sending session/new",
                t.elapsed().as_secs_f32()
            );

            let cwd = self
                .agent
                .cwd
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            let t = Instant::now();
            let session_id = with_progress(
                "session/new",
                self.verbose,
                client.session_new(&cwd, Some(self.agent.permission_mode)),
            )
            .await
            .context("ACP session/new failed")?;
            eprintln!(
                "[printer] acp: session/new ok in {:.1}s (id={session_id})",
                t.elapsed().as_secs_f32()
            );
            self.acp = Some(client);
            self.acp_session_id = Some(session_id);
        }

        let session_id = self.acp_session_id.clone().expect("acp session id set");
        // Inner scope so `prompt_fut` (which borrows `client`, which borrows
        // `self.acp`) is dropped before we mutate `self.acp` on the
        // interrupt path. Without the scope, the borrow checker rejects
        // `self.acp = None` inside the select arm.
        let result: Result<TurnOutcome> = {
            let client = self.acp.as_ref().expect("acp client set");
            let prompt_fut = client.prompt_blocking(&session_id, prompt, self.verbose);
            tokio::pin!(prompt_fut);
            tokio::select! {
                biased;
                _ = tokio::signal::ctrl_c() => {
                    eprintln!("[printer] interrupt received; cancelling acp turn");
                    // Fire-and-forget session/cancel so a well-behaved server
                    // resolves the in-flight prompt with stopReason=cancelled
                    // instead of being killed mid-write.
                    let _ = client.cancel().await;
                    // Give the server up to 500ms to land that resolution;
                    // if it doesn't, fall through to SIGKILL on the process
                    // group via shutdown().
                    let drained = tokio::time::timeout(
                        std::time::Duration::from_millis(500),
                        &mut prompt_fut,
                    )
                    .await;
                    client.shutdown().await;
                    match drained {
                        Ok(_) => Err(anyhow::anyhow!("interrupted by user")),
                        Err(_) => Err(anyhow::anyhow!(
                            "interrupted by user (acp server did not honor session/cancel within 500ms; killed)"
                        )),
                    }
                }
                res = &mut prompt_fut => res,
            }
        };
        let outcome = match result {
            Ok(o) => o,
            Err(e) => {
                self.acp = None;
                self.acp_session_id = None;
                return Err(e);
            }
        };

        self.turn_count += 1;
        self.usage_total.add(&outcome.usage);
        eprintln!(
            "[printer] turn {} done in {:.1}s (acp; usage not surfaced — see T-020)",
            self.turn_count,
            started.elapsed().as_secs_f32(),
        );
        self.fresh = false;
        Ok(outcome)
    }
}

async fn heartbeat_loop(
    stop: Arc<AtomicBool>,
    agent_active: Arc<AtomicBool>,
    started: Instant,
    tty: bool,
) {
    const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let mut i: usize = 0;
    let mut last_text_heartbeat = Instant::now();

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let elapsed = started.elapsed().as_secs();
        let active_marker = if agent_active.swap(false, Ordering::Relaxed) {
            "active"
        } else {
            "waiting"
        };

        if tty {
            eprint!(
                "\r\x1b[2K[printer] {} working… {}s ({})",
                FRAMES[i % FRAMES.len()],
                elapsed,
                active_marker
            );
            i = i.wrapping_add(1);
            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        } else {
            // Non-TTY: emit a textual heartbeat every ~10s, no animation.
            if last_text_heartbeat.elapsed().as_secs() >= 10 {
                eprintln!("[printer] still working… {elapsed}s ({active_marker})");
                last_text_heartbeat = Instant::now();
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
}

fn short(id: &Uuid) -> String {
    let s = id.to_string();
    s.chars().take(8).collect()
}

/// Wrap an in-flight async operation so the user can see it's still running.
/// While the future is pending and `verbose` is true, prints a one-line
/// "still waiting for <label>… <N>s" every 5s. Returns the future's output
/// unchanged. No-op in non-verbose mode (just awaits the future). Used to
/// give visibility into ACP bootstrap RPCs (`initialize`, `session/new`)
/// that otherwise go silent for as long as the server takes to answer.
async fn with_progress<F, T>(label: &str, verbose: bool, fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    if !verbose {
        return fut.await;
    }
    tokio::pin!(fut);
    let started = Instant::now();
    let mut hint_emitted = false;
    loop {
        tokio::select! {
            biased;
            v = &mut fut => return v,
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                let elapsed = started.elapsed().as_secs();
                eprintln!(
                    "[printer] acp: still waiting for {label}… {elapsed}s"
                );
                if !hint_emitted && elapsed >= 30 {
                    hint_emitted = true;
                    eprintln!(
                        "[printer] acp: {label} is taking >30s. \
                         If you're driving an ACP agent through a `heyvm` sandbox, this is \
                         likely the known `heyvm exec` stdout buffering issue: \
                         heyvm exec does not stream its child's stdout — it only flushes \
                         when the child exits, which deadlocks any persistent ACP server \
                         (the response is generated immediately but held in heyvm's buffer). \
                         Workaround: re-run with `--no-sandbox`, or pick a sandbox driver \
                         that streams stdio. Set PRINTER_ACP_TRACE=1 to see byte-level \
                         transport traces. Other possibilities (less likely if heyvm is \
                         in use): missing API key / unreachable auth files; child is \
                         writing to a read-only path inside the sandbox and silently \
                         retrying."
                    );
                }
            }
        }
    }
}

/// Render an ACP launch argv as a single human-readable token sequence for
/// progress logs. Truncates to keep the line short.
fn format_argv(bin: &str, args: &[String]) -> String {
    let mut s = bin.to_string();
    for a in args {
        s.push(' ');
        s.push_str(a);
    }
    if s.chars().count() > 80 {
        let head: String = s.chars().take(80).collect();
        format!("{head}…")
    } else {
        s
    }
}

/// Kill the child plus any descendants. We set process_group(0) when spawning,
/// so on Unix the child's pid is also its pgid and `kill(-pgid, SIGKILL)`
/// signals the whole subtree.
fn kill_subtree(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    {
        if let Some(pid) = child.id() {
            // SAFETY: kill() with a negative pid signals a process group; an
            // invalid pgid simply returns ESRCH. No memory is touched.
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
    }
    // Fallback / non-unix: at minimum kill the immediate child.
    let _ = child.start_kill();
}
