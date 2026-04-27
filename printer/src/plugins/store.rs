use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    /// Path to the binary, relative to the plugin's directory.
    pub binary: String,
    pub installed_at: String,
    pub source: Source,
    /// Lifecycle hooks the plugin registers. See `HOOKS.md`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<crate::hooks::HookSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Source {
    /// Cloned from git and built with `cargo install --path … --root …`.
    Git {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rev: Option<String>,
    },
    /// Built from a local source directory with `cargo install --path …`.
    Path {
        path: String,
    },
    /// Installed by running an arbitrary shell command (e.g. a vendor's
    /// `curl … | sh` installer). The binary lives wherever that command
    /// puts it; the resolved absolute path is stored in `Manifest::binary`.
    Shell {
        command: String,
    },
}

/// Resolve the per-user data directory `~/.printer/`. Created if missing.
pub fn data_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| anyhow!("$HOME is not set; cannot resolve ~/.printer"))?;
    let dir = PathBuf::from(home).join(".printer");
    fs::create_dir_all(&dir)
        .with_context(|| format!("creating data dir {}", dir.display()))?;
    Ok(dir)
}

pub fn plugins_dir() -> Result<PathBuf> {
    let dir = data_dir()?.join("plugins");
    fs::create_dir_all(&dir)
        .with_context(|| format!("creating plugins dir {}", dir.display()))?;
    Ok(dir)
}

pub fn plugin_dir(name: &str) -> Result<PathBuf> {
    Ok(plugins_dir()?.join(name))
}

pub fn manifest_path(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join("plugin.toml")
}

pub fn read_manifest(plugin_dir: &Path) -> Result<Manifest> {
    let path = manifest_path(plugin_dir);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("reading manifest {}", path.display()))?;
    let m: Manifest =
        toml::from_str(&raw).with_context(|| format!("parsing manifest {}", path.display()))?;
    Ok(m)
}

pub fn write_manifest(plugin_dir: &Path, manifest: &Manifest) -> Result<()> {
    let path = manifest_path(plugin_dir);
    let body = toml::to_string(manifest).context("serializing manifest")?;
    let tmp = path.with_extension("toml.tmp");
    fs::write(&tmp, body).with_context(|| format!("writing temp manifest {}", tmp.display()))?;
    fs::rename(&tmp, &path)
        .with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

/// True iff the plugin appears installed (manifest present).
pub fn installed(name: &str) -> Result<bool> {
    let dir = plugin_dir(name)?;
    Ok(manifest_path(&dir).is_file())
}

/// List installed plugins (sorted by name).
pub fn list_installed() -> Result<()> {
    let plugins = plugins_dir()?;
    let mut found: Vec<Manifest> = Vec::new();
    for entry in fs::read_dir(&plugins)
        .with_context(|| format!("reading {}", plugins.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let mp = manifest_path(&entry.path());
        if !mp.is_file() {
            continue;
        }
        match read_manifest(&entry.path()) {
            Ok(m) => found.push(m),
            Err(e) => eprintln!(
                "warning: skipping unreadable plugin at {}: {e}",
                entry.path().display()
            ),
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));

    if found.is_empty() {
        println!("(no plugins installed; try `printer add-plugin <name>`)");
        return Ok(());
    }

    let name_w = found.iter().map(|m| m.name.len()).max().unwrap_or(4).max(4);
    let ver_w = found.iter().map(|m| m.version.len()).max().unwrap_or(7).max(7);
    println!(
        "{:<name_w$}  {:<ver_w$}  SOURCE",
        "NAME",
        "VERSION",
        name_w = name_w,
        ver_w = ver_w
    );
    for m in &found {
        let src = match &m.source {
            Source::Git { url, rev } => match rev {
                Some(r) => format!("git {url}@{}", &r[..r.len().min(8)]),
                None => format!("git {url}"),
            },
            Source::Path { path } => format!("path {path}"),
            Source::Shell { command } => {
                let trimmed = command.trim();
                if trimmed.len() > 60 {
                    format!("shell {}…", &trimmed[..57])
                } else {
                    format!("shell {trimmed}")
                }
            }
        };
        println!(
            "{:<name_w$}  {:<ver_w$}  {}",
            m.name,
            m.version,
            src,
            name_w = name_w,
            ver_w = ver_w
        );
    }
    Ok(())
}
