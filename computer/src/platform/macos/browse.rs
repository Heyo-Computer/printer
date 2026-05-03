use anyhow::{Context, Result, bail};

pub fn run(url: &str) -> Result<()> {
    validate(url)?;
    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .with_context(|| format!("failed to launch open for {url}"))?;
    Ok(())
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
