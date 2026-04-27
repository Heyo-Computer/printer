# Books

A desktop app for personal knowledge bases using markdown and csv primitives. Built with Tauri + Yew.

## Storage layout

Each book lives in its own folder under the OS app data directory:

```
{app_data_dir}/books/
  {book_name}/
    book.json          # manifest (name, description, ordered entries)
    notes.md           # markdown primitive
    tasks.csv          # csv primitive
    roadmap.csv        # kanban primitive (csv-backed, see schema below)
```

`book.json` lists ordered entries with `{ id, name, kind, filename }` where `kind` is one of `markdown`, `csv`, or `kanban`.

## Kanban CSV schema

Kanban primitives are stored as CSV files with the following columns:

| Column   | Description                                        |
| -------- | -------------------------------------------------- |
| `id`     | Stable card identifier (uuid)                      |
| `title`  | Card title                                         |
| `status` | Column the card belongs to (e.g. `todo`, `done`)   |
| `notes`  | Free-form notes / description                      |
| `order`  | Integer ordering within the status column          |

The kanban view groups rows by `status` and orders cards within each column by `order` ascending.

## Development

```
cargo tauri dev
```

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
