use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::event::{CreateKind, EventKind, ModifyKind, RemoveKind, RenameMode};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::index::{self, Index, UpdateOutcome};

pub struct WatchOpts {
    pub root: PathBuf,
    pub debounce: Duration,
}

pub fn run(opts: WatchOpts) -> Result<()> {
    let root = opts
        .root
        .canonicalize()
        .with_context(|| format!("canonicalizing {}", opts.root.display()))?;

    // Make sure we have an up-to-date index on disk before we start streaming
    // changes, so the first read after `codegraph watch` is already correct.
    let mut index = match Index::load(&root)? {
        Some(idx) if idx.root == root => {
            eprintln!("[codegraph] reusing existing index at {}", Index::path_for(&root).display());
            // Refresh stale entries via a normal build pass.
            let (rebuilt, report) = index::build(&root, false)?;
            eprintln!(
                "[codegraph] startup index: {} files (parsed {}, reused {}, failed {})",
                rebuilt.files.len(),
                report.indexed,
                report.reused,
                report.failed.len()
            );
            rebuilt
        }
        _ => {
            eprintln!("[codegraph] no existing index; building from scratch");
            let (built, report) = index::build(&root, true)?;
            eprintln!(
                "[codegraph] startup index: {} files (parsed {}, failed {})",
                built.files.len(),
                report.indexed,
                report.failed.len()
            );
            built
        }
    };
    index.save()?;

    let gitignore = build_gitignore(&root);

    let (tx, rx) = channel::<notify::Result<Event>>();
    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .context("constructing fs watcher")?;
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .with_context(|| format!("watching {}", root.display()))?;

    eprintln!(
        "[codegraph] watching {} (debounce {}ms); press Ctrl-C to stop",
        root.display(),
        opts.debounce.as_millis()
    );

    event_loop(&rx, &mut index, &root, &gitignore, opts.debounce)
}

fn event_loop(
    rx: &Receiver<notify::Result<Event>>,
    index: &mut Index,
    root: &Path,
    gitignore: &Option<Gitignore>,
    debounce: Duration,
) -> Result<()> {
    // Pending paths to revisit, plus paths the user explicitly removed (so we
    // don't re-stat and skip the delete).
    let mut dirty: HashSet<PathBuf> = HashSet::new();
    let mut deleted: HashSet<PathBuf> = HashSet::new();
    let mut window_started: Option<Instant> = None;

    loop {
        let timeout = window_started
            .map(|start| {
                let elapsed = start.elapsed();
                if elapsed >= debounce {
                    Duration::from_millis(0)
                } else {
                    debounce - elapsed
                }
            })
            .unwrap_or_else(|| Duration::from_secs(60 * 60));

        match rx.recv_timeout(timeout) {
            Ok(Ok(event)) => {
                ingest_event(&event, &mut dirty, &mut deleted);
                if !dirty.is_empty() || !deleted.is_empty() {
                    window_started.get_or_insert_with(Instant::now);
                }
            }
            Ok(Err(e)) => {
                eprintln!("[codegraph] watcher error: {e}");
            }
            Err(RecvTimeoutError::Timeout) => {
                if window_started.is_none() {
                    continue;
                }
                flush(index, root, gitignore, &mut dirty, &mut deleted)?;
                window_started = None;
            }
            Err(RecvTimeoutError::Disconnected) => {
                eprintln!("[codegraph] watcher channel closed; exiting");
                return Ok(());
            }
        }
    }
}

fn ingest_event(event: &Event, dirty: &mut HashSet<PathBuf>, deleted: &mut HashSet<PathBuf>) {
    match event.kind {
        EventKind::Create(CreateKind::File | CreateKind::Any | CreateKind::Other)
        | EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Any | ModifyKind::Other)
        | EventKind::Modify(ModifyKind::Metadata(_))
        | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
            for p in &event.paths {
                deleted.remove(p);
                dirty.insert(p.clone());
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::From))
        | EventKind::Remove(RemoveKind::File | RemoveKind::Any | RemoveKind::Other) => {
            for p in &event.paths {
                dirty.remove(p);
                deleted.insert(p.clone());
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            // `paths` is [from, to].
            if let [from, to, ..] = event.paths.as_slice() {
                dirty.remove(from);
                deleted.insert(from.clone());
                deleted.remove(to);
                dirty.insert(to.clone());
            }
        }
        _ => {}
    }
}

fn flush(
    index: &mut Index,
    root: &Path,
    gitignore: &Option<Gitignore>,
    dirty: &mut HashSet<PathBuf>,
    deleted: &mut HashSet<PathBuf>,
) -> Result<()> {
    let mut indexed = 0usize;
    let mut removed = 0usize;
    let mut failed: Vec<(String, String)> = Vec::new();

    for path in deleted.drain() {
        if !is_under(&path, root) {
            continue;
        }
        if index::remove_file(index, &path) {
            removed += 1;
        }
    }

    for path in dirty.drain() {
        if !is_under(&path, root) {
            continue;
        }
        if index::is_path_excluded(path.strip_prefix(root).unwrap_or(&path)) {
            continue;
        }
        if let Some(gi) = gitignore {
            // is_dir is unknown without a stat; assume false so file rules apply.
            let m = gi.matched(&path, false);
            if m.is_ignore() {
                continue;
            }
        }
        match index::update_file(index, &path) {
            Ok((UpdateOutcome::Indexed, _)) => indexed += 1,
            Ok((UpdateOutcome::Removed, _)) => removed += 1,
            Ok(_) => {}
            Err(e) => failed.push((path.display().to_string(), e.to_string())),
        }
    }

    if indexed == 0 && removed == 0 && failed.is_empty() {
        return Ok(());
    }

    index.save().context("saving index after watch flush")?;
    eprintln!(
        "[codegraph] flushed: indexed {indexed}, removed {removed}, failed {}",
        failed.len()
    );
    for (path, err) in failed {
        eprintln!("  failed: {path}: {err}");
    }
    Ok(())
}

fn is_under(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

fn build_gitignore(root: &Path) -> Option<Gitignore> {
    let mut builder = GitignoreBuilder::new(root);
    let candidate = root.join(".gitignore");
    if candidate.exists() {
        if let Some(err) = builder.add(&candidate) {
            eprintln!("[codegraph] warning: parsing {}: {err}", candidate.display());
        }
    }
    builder.build().ok()
}
