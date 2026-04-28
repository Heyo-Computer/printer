use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};

use crate::languages::Language;
use crate::parse;
use crate::symbols::{self, Symbol};

pub const INDEX_DIRNAME: &str = ".codegraph";
pub const INDEX_FILENAME: &str = "index.json";
pub const INDEX_VERSION: u32 = 1;

/// Directory names that are always skipped, in addition to whatever
/// `.gitignore` rules out. Kept in one place so the indexer and the watcher
/// agree on what to ignore.
pub const ALWAYS_EXCLUDED_DIRS: &[&str] = &[
    INDEX_DIRNAME,
    ".git",
    ".hg",
    ".svn",
    "target",
    "node_modules",
    "dist",
    "build",
    ".next",
    ".nuxt",
    ".venv",
    "venv",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".tox",
    ".cache",
];

/// True if any path component matches one of `ALWAYS_EXCLUDED_DIRS`.
pub fn is_path_excluded(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(|s| ALWAYS_EXCLUDED_DIRS.contains(&s))
            .unwrap_or(false)
    })
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEntry {
    pub mtime: u64,
    pub language: Language,
    pub symbols: Vec<Symbol>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Index {
    pub version: u32,
    pub root: PathBuf,
    /// Relative path (POSIX-style) → file entry.
    pub files: BTreeMap<String, FileEntry>,
}

impl Index {
    pub fn new(root: PathBuf) -> Self {
        Self {
            version: INDEX_VERSION,
            root,
            files: BTreeMap::new(),
        }
    }

    pub fn path_for(root: &Path) -> PathBuf {
        root.join(INDEX_DIRNAME).join(INDEX_FILENAME)
    }

    pub fn load(root: &Path) -> Result<Option<Self>> {
        let path = Self::path_for(root);
        if !path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let idx: Index = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(Some(idx))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path_for(&self.root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let json = serde_json::to_vec_pretty(self).context("serializing index")?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, &json).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, &path)
            .with_context(|| format!("renaming {} → {}", tmp.display(), path.display()))?;
        Ok(())
    }
}

pub struct BuildReport {
    pub indexed: usize,
    pub reused: usize,
    pub failed: Vec<(String, String)>,
}

pub fn build(root: &Path, force: bool) -> Result<(Index, BuildReport)> {
    let root = root
        .canonicalize()
        .with_context(|| format!("canonicalizing {}", root.display()))?;
    let mut existing = if force {
        None
    } else {
        Index::load(&root)?.filter(|i| i.root == root)
    };
    let mut index = Index::new(root.clone());
    let mut report = BuildReport {
        indexed: 0,
        reused: 0,
        failed: Vec::new(),
    };

    let walker = WalkBuilder::new(&root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .filter_entry(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|n| !ALWAYS_EXCLUDED_DIRS.contains(&n))
                .unwrap_or(true)
        })
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.path();
        let Some(language) = Language::from_path(path) else {
            continue;
        };
        let rel = match path.strip_prefix(&root) {
            Ok(p) => p.to_path_buf(),
            Err(_) => continue,
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        let mtime = file_mtime(path).unwrap_or(0);
        if let Some(prev) = existing.as_mut().and_then(|i| i.files.remove(&rel_str)) {
            if prev.mtime == mtime && prev.language == language {
                index.files.insert(rel_str.clone(), prev);
                report.reused += 1;
                continue;
            }
        }

        match parse::parse_path(path) {
            Ok(parsed) => {
                let symbols = symbols::extract(&parsed);
                index.files.insert(
                    rel_str.clone(),
                    FileEntry {
                        mtime,
                        language,
                        symbols,
                    },
                );
                report.indexed += 1;
            }
            Err(e) => {
                report.failed.push((rel_str, e.to_string()));
            }
        }
    }

    Ok((index, report))
}

/// Outcome of incrementally updating a single file in an existing index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOutcome {
    /// Indexed (created or refreshed).
    Indexed,
    /// File mtime + language unchanged; index entry was kept as-is.
    Unchanged,
    /// File is gone or unreadable; entry was removed if present.
    Removed,
    /// File extension is not a supported language; nothing to do.
    Skipped,
}

/// Update a single file in `index`. Returns the outcome and a relative path
/// (POSIX-style) string if the file lives under the index root.
pub fn update_file(index: &mut Index, abs_path: &Path) -> Result<(UpdateOutcome, Option<String>)> {
    let rel = match abs_path.strip_prefix(&index.root) {
        Ok(p) => p.to_path_buf(),
        Err(_) => return Ok((UpdateOutcome::Skipped, None)),
    };
    if is_path_excluded(&rel) {
        return Ok((UpdateOutcome::Skipped, None));
    }
    let rel_str = rel.to_string_lossy().replace('\\', "/");

    let Some(language) = Language::from_path(abs_path) else {
        return Ok((UpdateOutcome::Skipped, Some(rel_str)));
    };

    let meta = match std::fs::metadata(abs_path) {
        Ok(m) => m,
        Err(_) => {
            // Treat missing as a delete.
            let removed = index.files.remove(&rel_str).is_some();
            return Ok((
                if removed { UpdateOutcome::Removed } else { UpdateOutcome::Skipped },
                Some(rel_str),
            ));
        }
    };
    if !meta.is_file() {
        return Ok((UpdateOutcome::Skipped, Some(rel_str)));
    }
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Some(prev) = index.files.get(&rel_str) {
        if prev.mtime == mtime && prev.language == language {
            return Ok((UpdateOutcome::Unchanged, Some(rel_str)));
        }
    }

    let parsed = parse::parse_path(abs_path)
        .with_context(|| format!("parsing {}", abs_path.display()))?;
    let symbols = symbols::extract(&parsed);
    index.files.insert(
        rel_str.clone(),
        FileEntry {
            mtime,
            language,
            symbols,
        },
    );
    Ok((UpdateOutcome::Indexed, Some(rel_str)))
}

/// Drop an entry from the index. Returns true if something was removed.
pub fn remove_file(index: &mut Index, abs_path: &Path) -> bool {
    let Ok(rel) = abs_path.strip_prefix(&index.root) else {
        return false;
    };
    let rel_str = rel.to_string_lossy().replace('\\', "/");
    index.files.remove(&rel_str).is_some()
}

fn file_mtime(path: &Path) -> Option<u64> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    mtime
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_and_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn alpha() {}\n").unwrap();
        std::fs::write(dir.path().join("b.py"), "def beta():\n    pass\n").unwrap();

        let (index, report) = build(dir.path(), true).unwrap();
        assert!(report.indexed >= 2, "report: {:?}", report.indexed);
        index.save().unwrap();

        let loaded = Index::load(&dir.path().canonicalize().unwrap()).unwrap().unwrap();
        assert_eq!(loaded.files.len(), index.files.len());
        let any_symbol = loaded
            .files
            .values()
            .flat_map(|e| e.symbols.iter())
            .any(|s| s.name == "alpha" || s.name == "beta");
        assert!(any_symbol);
    }
}
