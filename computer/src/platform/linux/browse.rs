use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run(url: &str) -> Result<()> {
    validate(url)?;
    let browser = detect();
    spawn(&browser, url)
}

#[derive(Debug, Clone)]
enum Browser {
    /// Firefox-family: takes `--profile <dir>` and reuses the instance bound
    /// to that profile via remoting.
    Firefox(String),
    /// Chromium-family: takes `--user-data-dir=<dir>` and reuses the instance
    /// bound to that data dir.
    Chromium(String),
    /// Unrecognized default; fall through to xdg-open.
    Unknown,
}

fn spawn(browser: &Browser, url: &str) -> Result<()> {
    match browser {
        Browser::Firefox(bin) => {
            let dir = profile_dir("firefox")?;
            Command::new(bin)
                .arg("--profile")
                .arg(&dir)
                .arg(url)
                .spawn()
                .with_context(|| format!("failed to launch {bin} for {url}"))?;
        }
        Browser::Chromium(bin) => {
            let dir = profile_dir("chromium")?;
            Command::new(bin)
                .arg(format!("--user-data-dir={}", dir.display()))
                .arg(url)
                .spawn()
                .with_context(|| format!("failed to launch {bin} for {url}"))?;
        }
        Browser::Unknown => {
            Command::new("xdg-open")
                .arg(url)
                .spawn()
                .with_context(|| format!("failed to launch xdg-open for {url}"))?;
        }
    }
    Ok(())
}

/// `$XDG_CACHE_HOME/printer/browser-profile/<key>` (default `~/.cache/...`),
/// created on demand. Owned by printer, never shared with the user's daily
/// browser session — so Firefox's profile-lock dialog can't appear.
fn profile_dir(key: &str) -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .context("neither XDG_CACHE_HOME nor HOME is set")?;
    let dir = base.join("printer").join("browser-profile").join(key);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create profile dir {}", dir.display()))?;
    Ok(dir)
}

fn detect() -> Browser {
    let desktop = default_browser_desktop().unwrap_or_default();
    let desktop = desktop.trim().to_ascii_lowercase();

    // .desktop name → preferred binary candidates. First binary on $PATH wins.
    let candidates: &[(&[&str], fn(String) -> Browser)] = &[
        (&["firefox", "firefox-esr"], Browser::Firefox),
        // Firefox forks — all accept `--profile <dir>` the same way.
        (&["librewolf"], Browser::Firefox),
        (&["waterfox"], Browser::Firefox),
        (&["icecat"], Browser::Firefox),
        (&["zen-twilight", "zen-browser", "zen"], Browser::Firefox),
        (
            &["google-chrome-stable", "google-chrome", "chrome"],
            Browser::Chromium,
        ),
        (&["chromium", "chromium-browser"], Browser::Chromium),
        (&["brave-browser", "brave"], Browser::Chromium),
        (&["microsoft-edge-stable", "microsoft-edge"], Browser::Chromium),
        (&["vivaldi-stable", "vivaldi"], Browser::Chromium),
    ];

    for (bins, ctor) in candidates {
        if bins.iter().any(|b| desktop.contains(b)) {
            for bin in *bins {
                if on_path(bin) {
                    return ctor((*bin).to_string());
                }
            }
        }
    }
    Browser::Unknown
}

fn default_browser_desktop() -> Option<String> {
    let out = Command::new("xdg-settings")
        .args(["get", "default-web-browser"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn on_path(bin: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|p| is_executable(&p.join(bin)))
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(m) => m.is_file() && m.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

fn validate(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        bail!("url must not be empty");
    }
    if !(trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("file://"))
    {
        bail!("url must start with http://, https://, or file://");
    }
    Ok(())
}
