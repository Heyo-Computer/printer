use super::store;
use anyhow::{Result, anyhow, bail};
use std::ffi::OsString;

/// Forward `printer <plugin> <args>...` to the plugin binary. On Unix we
/// `exec` so the plugin replaces `printer` in the process tree (clean
/// signal handling, single line in `ps`).
pub fn exec_external(args: &[OsString]) -> Result<()> {
    let (head, rest) = args
        .split_first()
        .ok_or_else(|| anyhow!("internal: external_subcommand received zero args"))?;
    let name = head
        .to_str()
        .ok_or_else(|| anyhow!("plugin name must be UTF-8: {:?}", head))?;

    let dir = store::plugin_dir(name)?;
    if !store::installed(name)? {
        bail!(
            "no such plugin `{name}`; install one with: printer add-plugin {name}\n\
             (or check `printer plugins` for what's currently installed)"
        );
    }
    let manifest = store::read_manifest(&dir)?;
    if manifest.binary.is_empty() {
        bail!(
            "plugin `{name}` is skill-only (no binary); it only contributes hooks/skills \
             to run/review and cannot be invoked directly via `printer {name}`."
        );
    }
    let bin = dir.join(&manifest.binary);
    if !bin.is_file() {
        bail!(
            "plugin `{name}` manifest points at {} but the binary is missing — try \
             `printer add-plugin {name} --force` to reinstall",
            bin.display()
        );
    }

    spawn_or_exec(&bin, rest)
}

#[cfg(unix)]
fn spawn_or_exec(bin: &std::path::Path, rest: &[OsString]) -> Result<()> {
    use std::os::unix::process::CommandExt;
    let err = std::process::Command::new(bin)
        .args(rest.iter().map(|a| a.as_os_str()))
        .exec();
    Err(anyhow!("exec {} failed: {err}", bin.display()))
}

#[cfg(not(unix))]
fn spawn_or_exec(bin: &std::path::Path, rest: &[OsString]) -> Result<()> {
    use anyhow::Context;
    let status = std::process::Command::new(bin)
        .args(rest.iter().map(|a| a.as_os_str()))
        .status()
        .with_context(|| format!("spawning {}", bin.display()))?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
