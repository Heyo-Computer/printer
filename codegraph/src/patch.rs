use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct PatchReport {
    pub ok: bool,
    pub file: String,
    pub hunks_total: usize,
    pub hunks_applied: usize,
    /// 1-based hunk index that failed to apply, if any.
    pub failed_hunk: Option<usize>,
    pub failure: Option<String>,
    pub bytes_written: Option<usize>,
}

pub struct PatchOpts<'a> {
    pub file: &'a Path,
    pub diff_path: Option<&'a Path>,
    pub check_only: bool,
    pub allow_outside: bool,
}

pub fn run(opts: PatchOpts<'_>) -> Result<PatchReport> {
    let file = if opts.file.is_absolute() {
        opts.file.to_path_buf()
    } else {
        std::env::current_dir()?.join(opts.file)
    };
    let cwd = std::env::current_dir()?;
    if !opts.allow_outside {
        let canon_file = file.canonicalize().unwrap_or_else(|_| file.clone());
        let canon_cwd = cwd.canonicalize().unwrap_or(cwd.clone());
        if !canon_file.starts_with(&canon_cwd) {
            bail!(
                "{} is outside the working directory; pass --allow-outside to override",
                file.display()
            );
        }
    }

    let diff = read_diff(opts.diff_path)?;
    let patch = diffy::Patch::from_str(&diff).context("parsing unified diff")?;
    let hunks_total = patch.hunks().len();

    let original = std::fs::read_to_string(&file)
        .with_context(|| format!("reading {}", file.display()))?;

    let file_display = file.display().to_string();

    match diffy::apply(&original, &patch) {
        Ok(new_contents) => {
            let bytes_written = if opts.check_only {
                None
            } else {
                atomic_write(&file, new_contents.as_bytes())?;
                Some(new_contents.len())
            };
            Ok(PatchReport {
                ok: true,
                file: file_display,
                hunks_total,
                hunks_applied: hunks_total,
                failed_hunk: None,
                failure: None,
                bytes_written,
            })
        }
        Err(e) => {
            let msg = e.to_string();
            // ApplyError's Display is "error applying hunk #N"; pull the index.
            let failed = msg
                .split('#')
                .nth(1)
                .and_then(|s| s.trim().parse::<usize>().ok());
            Ok(PatchReport {
                ok: false,
                file: file_display,
                hunks_total,
                hunks_applied: failed.map(|n| n.saturating_sub(1)).unwrap_or(0),
                failed_hunk: failed,
                failure: Some(msg),
                bytes_written: None,
            })
        }
    }
}

fn read_diff(diff_path: Option<&Path>) -> Result<String> {
    match diff_path {
        Some(p) => std::fs::read_to_string(p)
            .with_context(|| format!("reading diff from {}", p.display())),
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("reading diff from stdin")?;
            if buf.is_empty() {
                return Err(anyhow!("no diff supplied (empty stdin and no --diff path)"));
            }
            Ok(buf)
        }
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir: PathBuf = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let mut tmp = tempfile::NamedTempFile::new_in(&dir)
        .with_context(|| format!("creating tempfile in {}", dir.display()))?;
    tmp.write_all(bytes).context("writing patched contents")?;
    tmp.flush().ok();
    tmp.persist(path)
        .map_err(|e| anyhow!("persisting tempfile to {}: {}", path.display(), e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_simple_diff() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("hello.txt");
        std::fs::write(&file, "hello\nworld\n").unwrap();
        let diff = "\
--- a/hello.txt
+++ b/hello.txt
@@ -1,2 +1,2 @@
 hello
-world
+earth
";
        let diff_path = dir.path().join("d.patch");
        std::fs::write(&diff_path, diff).unwrap();

        let report = run(PatchOpts {
            file: &file,
            diff_path: Some(&diff_path),
            check_only: false,
            allow_outside: true,
        })
        .unwrap();
        assert!(report.ok, "{report:?}");
        let result = std::fs::read_to_string(&file).unwrap();
        assert_eq!(result, "hello\nearth\n");
    }

    #[test]
    fn check_does_not_write() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("hello.txt");
        std::fs::write(&file, "hello\nworld\n").unwrap();
        let diff = "\
--- a/hello.txt
+++ b/hello.txt
@@ -1,2 +1,2 @@
 hello
-world
+earth
";
        let diff_path = dir.path().join("d.patch");
        std::fs::write(&diff_path, diff).unwrap();

        let report = run(PatchOpts {
            file: &file,
            diff_path: Some(&diff_path),
            check_only: true,
            allow_outside: true,
        })
        .unwrap();
        assert!(report.ok);
        assert_eq!(report.bytes_written, None);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "hello\nworld\n");
    }

    #[test]
    fn failed_hunk_leaves_file_intact() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("hello.txt");
        std::fs::write(&file, "totally different content\n").unwrap();
        let diff = "\
--- a/hello.txt
+++ b/hello.txt
@@ -1,2 +1,2 @@
 hello
-world
+earth
";
        let diff_path = dir.path().join("d.patch");
        std::fs::write(&diff_path, diff).unwrap();

        let report = run(PatchOpts {
            file: &file,
            diff_path: Some(&diff_path),
            check_only: false,
            allow_outside: true,
        })
        .unwrap();
        assert!(!report.ok);
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "totally different content\n"
        );
    }
}
