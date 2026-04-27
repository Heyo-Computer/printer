use crate::cli::InitArgs;
use anyhow::{Context, Result, bail};
use std::path::PathBuf;

pub fn init(args: InitArgs) -> Result<()> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("spec.md"));
    if path.exists() && !args.force {
        bail!(
            "{} already exists; pass --force to overwrite",
            path.display()
        );
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating parent dir {}", parent.display()))?;
    }
    let body = template(&args.title);
    std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;
    eprintln!("[printer] wrote {}", path.display());
    eprintln!(
        "Edit the checklist, then run `printer run {}` to drive the work.",
        path.display()
    );
    Ok(())
}

fn template(title: &str) -> String {
    format!(
        "# {title}

A short description of what this project is and why we're doing it.

## Tasks

- [ ] First task — short imperative title for one unit of work
  Optional indented description (2-space indent). Multi-line is fine.
  Blank lines inside the description are preserved.

- [ ] Second task — replace this with a real task or delete it

- [x] Use `[x]` for items that were done before this spec existed; the
  driver will create them already in the `done` state.

<!--
Spec format reference (full docs in the printer README):
  * Lines starting with `- [ ]`, `- [x]`, `* [ ]`, `+ [ ]` (etc.) at
    column 0 are tasks. The text after the checkbox is the title.
  * Lines indented by 2 spaces or one tab below a task become its
    description body.
  * Any unindented non-task line ends the current task's description.
  * Re-runs of `printer run {title_path}` are idempotent — items are
    matched to existing tasks by a stable anchor derived from this
    file's path + the task title.
-->
",
        title = title,
        // %-encode-style hint for users who renamed the file later
        title_path = "<this-file>",
    )
}
