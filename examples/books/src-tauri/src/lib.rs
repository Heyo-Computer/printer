mod storage;

use storage::{BookManifest, BookSummary, CsvData, PrimitiveEntry, PrimitiveKind, Result};
use tauri::AppHandle;

#[tauri::command]
fn ensure_data_dir(app: AppHandle) -> Result<String> {
    Ok(storage::ensure_data_dir(&app)?
        .to_string_lossy()
        .into_owned())
}

#[tauri::command]
fn list_books(app: AppHandle) -> Result<Vec<BookSummary>> {
    storage::list_books(&app)
}

#[tauri::command]
fn create_book(app: AppHandle, name: String, description: String) -> Result<BookManifest> {
    storage::create_book(&app, &name, &description)
}

#[tauri::command]
fn get_manifest(app: AppHandle, book: String) -> Result<BookManifest> {
    storage::get_manifest(&app, &book)
}

#[tauri::command]
fn list_book_contents(app: AppHandle, book: String) -> Result<Vec<PrimitiveEntry>> {
    storage::list_book_contents(&app, &book)
}

#[tauri::command]
fn read_file(app: AppHandle, book: String, filename: String) -> Result<String> {
    storage::read_file(&app, &book, &filename)
}

#[tauri::command]
fn write_file(app: AppHandle, book: String, filename: String, contents: String) -> Result<()> {
    storage::write_file(&app, &book, &filename, &contents)
}

#[tauri::command]
fn read_csv(app: AppHandle, book: String, filename: String) -> Result<CsvData> {
    storage::read_csv(&app, &book, &filename)
}

#[tauri::command]
fn write_csv(app: AppHandle, book: String, filename: String, data: CsvData) -> Result<()> {
    storage::write_csv(&app, &book, &filename, &data)
}

#[tauri::command]
fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[tauri::command]
fn add_primitive(
    app: AppHandle,
    book: String,
    name: String,
    kind: PrimitiveKind,
) -> Result<PrimitiveEntry> {
    let id = uuid::Uuid::new_v4().to_string();
    storage::add_primitive(&app, &book, &id, &name, kind)
}

#[tauri::command]
fn rename_primitive(
    app: AppHandle,
    book: String,
    id: String,
    new_name: String,
) -> Result<BookManifest> {
    storage::rename_primitive(&app, &book, &id, &new_name)
}

#[tauri::command]
fn reorder_primitives(
    app: AppHandle,
    book: String,
    ordered_ids: Vec<String>,
) -> Result<BookManifest> {
    storage::reorder_primitives(&app, &book, &ordered_ids)
}

#[tauri::command]
fn delete_primitive(app: AppHandle, book: String, id: String) -> Result<BookManifest> {
    storage::delete_primitive(&app, &book, &id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let _ = storage::ensure_data_dir(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ensure_data_dir,
            list_books,
            create_book,
            get_manifest,
            list_book_contents,
            read_file,
            write_file,
            read_csv,
            write_csv,
            generate_id,
            add_primitive,
            rename_primitive,
            reorder_primitives,
            delete_primitive,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
