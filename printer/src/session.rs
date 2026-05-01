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
    pub fn rotate(&mut self) {
        self.id = Uuid::new_v4();
        self.fresh = true;
        self.cumulative_input_tokens = 0;
        // For ACP, drop the existing client so the next turn re-spawns the
        // server and starts a fresh `session/new`. Mirrors what one-shot
        // backends do implicitly by spawning a new process per turn.
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
            let client = AcpClient::spawn(
                bin,
                self.agent.acp_args,
                self.agent.cwd,
                self.agent.command_wrapper,
                self.agent.acp_env,
            )
            .await
            .context("spawning ACP server")?;
            client.initialize().await.context("ACP initialize failed")?;
            let cwd = self
                .agent
                .cwd
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            let session_id = client
                .session_new(&cwd)
                .await
                .context("ACP session/new failed")?;
            eprintln!("[printer] acp server up; session/new id={session_id}");
            self.acp = Some(client);
            self.acp_session_id = Some(session_id);
        }

        let session_id = self.acp_session_id.clone().expect("acp session id set");
        let client = self.acp.as_ref().expect("acp client set");
        let outcome = client.prompt_blocking(&session_id, prompt).await?;

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
