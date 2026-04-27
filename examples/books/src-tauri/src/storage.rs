use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const BOOKS_DIR: &str = "books";
const MANIFEST_FILE: &str = "book.json";

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("tauri error: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),
    #[error("invalid book or file name: {0}")]
    InvalidName(String),
    #[error("not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

impl serde::Serialize for StorageError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrimitiveKind {
    Markdown,
    Csv,
    Kanban,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveEntry {
    pub id: String,
    pub name: String,
    pub kind: PrimitiveKind,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookManifest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub entries: Vec<PrimitiveEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSummary {
    pub name: String,
    pub description: String,
    pub entry_count: usize,
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name.starts_with('.')
    {
        return Err(StorageError::InvalidName(name.to_string()));
    }
    Ok(())
}

pub fn books_root(app: &AppHandle) -> Result<PathBuf> {
    let base = app.path().app_data_dir()?;
    let root = base.join(BOOKS_DIR);
    if !root.exists() {
        fs::create_dir_all(&root)?;
    }
    Ok(root)
}

fn book_dir(app: &AppHandle, book: &str) -> Result<PathBuf> {
    validate_name(book)?;
    Ok(books_root(app)?.join(book))
}

fn file_path(app: &AppHandle, book: &str, filename: &str) -> Result<PathBuf> {
    validate_name(filename)?;
    Ok(book_dir(app, book)?.join(filename))
}

fn read_manifest(dir: &Path) -> Result<BookManifest> {
    let path = dir.join(MANIFEST_FILE);
    let raw = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn write_manifest(dir: &Path, manifest: &BookManifest) -> Result<()> {
    let path = dir.join(MANIFEST_FILE);
    let raw = serde_json::to_string_pretty(manifest)?;
    fs::write(path, raw)?;
    Ok(())
}

pub fn ensure_data_dir(app: &AppHandle) -> Result<PathBuf> {
    books_root(app)
}

pub fn list_books(app: &AppHandle) -> Result<Vec<BookSummary>> {
    let root = books_root(app)?;
    let mut out = Vec::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest = match read_manifest(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        out.push(BookSummary {
            name: manifest.name,
            description: manifest.description,
            entry_count: manifest.entries.len(),
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

pub fn create_book(app: &AppHandle, name: &str, description: &str) -> Result<BookManifest> {
    validate_name(name)?;
    let dir = books_root(app)?.join(name);
    if dir.exists() {
        return Err(StorageError::InvalidName(format!(
            "book already exists: {name}"
        )));
    }
    fs::create_dir_all(&dir)?;
    let manifest = BookManifest {
        name: name.to_string(),
        description: description.to_string(),
        entries: Vec::new(),
    };
    write_manifest(&dir, &manifest)?;
    Ok(manifest)
}

pub fn get_manifest(app: &AppHandle, book: &str) -> Result<BookManifest> {
    let dir = book_dir(app, book)?;
    if !dir.exists() {
        return Err(StorageError::NotFound(book.to_string()));
    }
    read_manifest(&dir)
}

pub fn list_book_contents(app: &AppHandle, book: &str) -> Result<Vec<PrimitiveEntry>> {
    Ok(get_manifest(app, book)?.entries)
}

pub fn read_file(app: &AppHandle, book: &str, filename: &str) -> Result<String> {
    let path = file_path(app, book, filename)?;
    if !path.exists() {
        return Err(StorageError::NotFound(format!("{book}/{filename}")));
    }
    Ok(fs::read_to_string(path)?)
}

pub fn write_file(app: &AppHandle, book: &str, filename: &str, contents: &str) -> Result<()> {
    let dir = book_dir(app, book)?;
    if !dir.exists() {
        return Err(StorageError::NotFound(book.to_string()));
    }
    validate_name(filename)?;
    fs::write(dir.join(filename), contents)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub fn read_csv(app: &AppHandle, book: &str, filename: &str) -> Result<CsvData> {
    let path = file_path(app, book, filename)?;
    if !path.exists() {
        return Ok(CsvData {
            headers: Vec::new(),
            rows: Vec::new(),
        });
    }
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;
    let headers = rdr
        .headers()?
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for record in rdr.records() {
        let record = record?;
        let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
        rows.push(row);
    }
    Ok(CsvData { headers, rows })
}

fn generate_filename(name: &str, kind: &PrimitiveKind, taken: &[String]) -> String {
    let base: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let base = if base.is_empty() { "untitled".to_string() } else { base };
    let ext = match kind {
        PrimitiveKind::Markdown => "md",
        PrimitiveKind::Csv | PrimitiveKind::Kanban => "csv",
    };
    let mut candidate = format!("{base}.{ext}");
    let mut n = 1;
    while taken.iter().any(|t| t == &candidate) {
        n += 1;
        candidate = format!("{base}-{n}.{ext}");
    }
    candidate
}

pub fn add_primitive(
    app: &AppHandle,
    book: &str,
    id: &str,
    name: &str,
    kind: PrimitiveKind,
) -> Result<PrimitiveEntry> {
    let dir = book_dir(app, book)?;
    if !dir.exists() {
        return Err(StorageError::NotFound(book.to_string()));
    }
    let mut manifest = read_manifest(&dir)?;
    let taken: Vec<String> = manifest.entries.iter().map(|e| e.filename.clone()).collect();
    let filename = generate_filename(name, &kind, &taken);
    validate_name(&filename)?;
    let path = dir.join(&filename);
    if path.exists() {
        return Err(StorageError::InvalidName(format!(
            "file already exists: {filename}"
        )));
    }
    let initial = match kind {
        PrimitiveKind::Markdown => format!("# {name}\n"),
        PrimitiveKind::Csv => String::new(),
        PrimitiveKind::Kanban => "id,title,status,notes,order\n".to_string(),
    };
    fs::write(&path, initial)?;
    let entry = PrimitiveEntry {
        id: id.to_string(),
        name: name.to_string(),
        kind,
        filename,
    };
    manifest.entries.push(entry.clone());
    write_manifest(&dir, &manifest)?;
    Ok(entry)
}

pub fn rename_primitive(
    app: &AppHandle,
    book: &str,
    id: &str,
    new_name: &str,
) -> Result<BookManifest> {
    let dir = book_dir(app, book)?;
    let mut manifest = read_manifest(&dir)?;
    let entry = manifest
        .entries
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or_else(|| StorageError::NotFound(format!("primitive {id}")))?;
    entry.name = new_name.to_string();
    write_manifest(&dir, &manifest)?;
    Ok(manifest)
}

pub fn reorder_primitives(
    app: &AppHandle,
    book: &str,
    ordered_ids: &[String],
) -> Result<BookManifest> {
    let dir = book_dir(app, book)?;
    let mut manifest = read_manifest(&dir)?;
    let mut by_id: std::collections::HashMap<String, PrimitiveEntry> = manifest
        .entries
        .drain(..)
        .map(|e| (e.id.clone(), e))
        .collect();
    let mut reordered = Vec::with_capacity(by_id.len());
    for id in ordered_ids {
        if let Some(entry) = by_id.remove(id) {
            reordered.push(entry);
        }
    }
    for (_, entry) in by_id.into_iter() {
        reordered.push(entry);
    }
    manifest.entries = reordered;
    write_manifest(&dir, &manifest)?;
    Ok(manifest)
}

pub fn delete_primitive(app: &AppHandle, book: &str, id: &str) -> Result<BookManifest> {
    let dir = book_dir(app, book)?;
    let mut manifest = read_manifest(&dir)?;
    let pos = manifest
        .entries
        .iter()
        .position(|e| e.id == id)
        .ok_or_else(|| StorageError::NotFound(format!("primitive {id}")))?;
    let entry = manifest.entries.remove(pos);
    let path = dir.join(&entry.filename);
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    write_manifest(&dir, &manifest)?;
    Ok(manifest)
}

pub fn write_csv(app: &AppHandle, book: &str, filename: &str, data: &CsvData) -> Result<()> {
    let dir = book_dir(app, book)?;
    if !dir.exists() {
        return Err(StorageError::NotFound(book.to_string()));
    }
    validate_name(filename)?;
    let path = dir.join(filename);
    let mut wtr = csv::WriterBuilder::new()
        .flexible(true)
        .from_path(path)?;
    if !data.headers.is_empty() {
        wtr.write_record(&data.headers)?;
    }
    for row in &data.rows {
        wtr.write_record(row)?;
    }
    wtr.flush()?;
    Ok(())
}
