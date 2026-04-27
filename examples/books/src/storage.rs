use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrimitiveKind {
    Markdown,
    Csv,
    Kanban,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrimitiveEntry {
    pub id: String,
    pub name: String,
    pub kind: PrimitiveKind,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BookManifest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub entries: Vec<PrimitiveEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BookSummary {
    pub name: String,
    pub description: String,
    pub entry_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CsvData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ApiError(pub String);

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn js_err_to_string(e: JsValue) -> ApiError {
    ApiError(
        e.as_string()
            .or_else(|| js_sys::JSON::stringify(&e).ok().and_then(|s| s.as_string()))
            .unwrap_or_else(|| "unknown error".to_string()),
    )
}

async fn call<A: Serialize, R: for<'de> Deserialize<'de>>(
    cmd: &str,
    args: &A,
) -> Result<R, ApiError> {
    let args_js = to_value(args).map_err(|e| ApiError(e.to_string()))?;
    let res = invoke(cmd, args_js).await.map_err(js_err_to_string)?;
    from_value(res).map_err(|e| ApiError(e.to_string()))
}

#[derive(Serialize)]
struct Empty {}

#[derive(Serialize)]
struct CreateBookArgs<'a> {
    name: &'a str,
    description: &'a str,
}

#[derive(Serialize)]
struct BookArgs<'a> {
    book: &'a str,
}

#[derive(Serialize)]
struct FileArgs<'a> {
    book: &'a str,
    filename: &'a str,
}

#[derive(Serialize)]
struct WriteFileArgs<'a> {
    book: &'a str,
    filename: &'a str,
    contents: &'a str,
}

pub async fn ensure_data_dir() -> Result<String, ApiError> {
    call("ensure_data_dir", &Empty {}).await
}

pub async fn list_books() -> Result<Vec<BookSummary>, ApiError> {
    call("list_books", &Empty {}).await
}

pub async fn create_book(name: &str, description: &str) -> Result<BookManifest, ApiError> {
    call("create_book", &CreateBookArgs { name, description }).await
}

pub async fn get_manifest(book: &str) -> Result<BookManifest, ApiError> {
    call("get_manifest", &BookArgs { book }).await
}

pub async fn list_book_contents(book: &str) -> Result<Vec<PrimitiveEntry>, ApiError> {
    call("list_book_contents", &BookArgs { book }).await
}

pub async fn read_file(book: &str, filename: &str) -> Result<String, ApiError> {
    call("read_file", &FileArgs { book, filename }).await
}

pub async fn write_file(book: &str, filename: &str, contents: &str) -> Result<(), ApiError> {
    call(
        "write_file",
        &WriteFileArgs {
            book,
            filename,
            contents,
        },
    )
    .await
}

#[derive(Serialize)]
struct WriteCsvArgs<'a> {
    book: &'a str,
    filename: &'a str,
    data: &'a CsvData,
}

pub async fn read_csv(book: &str, filename: &str) -> Result<CsvData, ApiError> {
    call("read_csv", &FileArgs { book, filename }).await
}

pub async fn write_csv(book: &str, filename: &str, data: &CsvData) -> Result<(), ApiError> {
    call(
        "write_csv",
        &WriteCsvArgs {
            book,
            filename,
            data,
        },
    )
    .await
}

pub async fn generate_id() -> Result<String, ApiError> {
    call("generate_id", &Empty {}).await
}

#[derive(Serialize)]
struct AddPrimitiveArgs<'a> {
    book: &'a str,
    name: &'a str,
    kind: &'a PrimitiveKind,
}

#[derive(Serialize)]
struct RenamePrimitiveArgs<'a> {
    book: &'a str,
    id: &'a str,
    #[serde(rename = "newName")]
    new_name: &'a str,
}

#[derive(Serialize)]
struct ReorderArgs<'a> {
    book: &'a str,
    #[serde(rename = "orderedIds")]
    ordered_ids: &'a [String],
}

#[derive(Serialize)]
struct DeletePrimitiveArgs<'a> {
    book: &'a str,
    id: &'a str,
}

pub async fn add_primitive(
    book: &str,
    name: &str,
    kind: &PrimitiveKind,
) -> Result<PrimitiveEntry, ApiError> {
    call("add_primitive", &AddPrimitiveArgs { book, name, kind }).await
}

pub async fn rename_primitive(
    book: &str,
    id: &str,
    new_name: &str,
) -> Result<BookManifest, ApiError> {
    call(
        "rename_primitive",
        &RenamePrimitiveArgs { book, id, new_name },
    )
    .await
}

pub async fn reorder_primitives(
    book: &str,
    ordered_ids: &[String],
) -> Result<BookManifest, ApiError> {
    call("reorder_primitives", &ReorderArgs { book, ordered_ids }).await
}

pub async fn delete_primitive(book: &str, id: &str) -> Result<BookManifest, ApiError> {
    call("delete_primitive", &DeletePrimitiveArgs { book, id }).await
}
