//! Auto-spawn a `codegraph watch` daemon for the duration of a `run` or
//! `exec` so the on-disk index stays fresh while the agent works.
//!
//! Best-effort: if the `codegraph` binary isn't installed, we log a one-line
//! note and keep going — no agent run should fail because of the index
//! daemon. The spawned child is killed when the returned `Guard` is dropped.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::Result;
use tokio::process::{Child, Command};

pub struct Guard {
    _child: Child,
    pid: Option<u32>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        // The Child was spawned with kill_on_drop(true), so tokio will SIGKILL
        // the immediate child as part of its drop. Just emit a breadcrumb.
        if let Some(pid) = self.pid {
            eprintln!("[printer] stopping codegraph watch daemon (pid {pid})");
        }
    }
}

/// Try to spawn `codegraph watch <cwd>`. Returns:
/// - `Ok(Some(Guard))` on success.
/// - `Ok(None)` if the binary isn't installed.
/// - `Err(_)` only if we found the binary but the spawn itself failed.
pub fn try_spawn(cwd: &Path) -> Result<Option<Guard>> {
    let Some(bin) = locate_binary() else {
        eprintln!(
            "[printer] codegraph not found on PATH or in ~/.printer/plugins/codegraph/bin; \
             skipping watch daemon. Install with `make install-codegraph` to enable."
        );
        return Ok(None);
    };

    let log_path = cwd.join(".printer").join("codegraph-watch.log");
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let log = match std::fs::File::create(&log_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "[printer] could not open {} for codegraph watch log: {e}; skipping daemon",
                log_path.display()
            );
            return Ok(None);
        }
    };
    let log_err = log.try_clone().unwrap_or_else(|_| {
        // Fall back to a fresh handle; if even this fails we'll error below.
        std::fs::File::create(&log_path).expect("reopened log handle")
    });

    let mut cmd = Command::new(&bin);
    cmd.arg("watch")
        .arg(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .kill_on_drop(true);
    #[cfg(unix)]
    cmd.process_group(0);

    let child = cmd.spawn().map_err(|e| {
        anyhow::anyhow!("failed to spawn `{} watch`: {e}", bin.display())
    })?;
    let pid = child.id();
    eprintln!(
        "[printer] launched codegraph watch ({}); logs → {}",
        pid.map(|p| format!("pid {p}")).unwrap_or_else(|| "no pid".into()),
        log_path.display()
    );
    Ok(Some(Guard { _child: child, pid }))
}

fn locate_binary() -> Option<PathBuf> {
    if let Ok(p) = which("codegraph") {
        return Some(p);
    }
    if let Some(home) = std::env::var_os("HOME") {
        let candidate = PathBuf::from(home)
            .join(".printer")
            .join("plugins")
            .join("codegraph")
            .join("bin")
            .join("codegraph");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Minimal `which` so we don't pull in an extra crate for one lookup.
fn which(name: &str) -> Result<PathBuf> {
    let path_var = std::env::var_os("PATH").ok_or_else(|| anyhow::anyhow!("PATH unset"))?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    anyhow::bail!("`{name}` not found on PATH")
}
