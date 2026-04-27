use std::collections::{HashMap, HashSet};

use web_sys::{window, HtmlInputElement, HtmlSelectElement, KeyboardEvent};
use yew::prelude::*;

use crate::storage::{BookManifest, BookSummary, PrimitiveEntry, PrimitiveKind};

const STORAGE_KEY_EXPANDED: &str = "books.sidebar.expanded";

#[derive(Debug, Clone, PartialEq)]
pub struct Selection {
    pub book: String,
    pub primitive: Option<PrimitiveEntry>,
}

#[derive(Properties, PartialEq, Clone)]
pub struct SidebarProps {
    pub books: Vec<BookSummary>,
    pub manifests: HashMap<String, BookManifest>,
    pub active: Option<Selection>,
    pub on_select: Callback<Selection>,
    pub on_request_manifest: Callback<String>,
    pub on_add_primitive: Callback<(String, String, PrimitiveKind)>,
    pub on_delete_primitive: Callback<(String, String)>,
}

fn read_expanded() -> HashSet<String> {
    let Some(win) = window() else { return HashSet::new() };
    let Ok(Some(storage)) = win.local_storage() else { return HashSet::new() };
    let raw = storage
        .get_item(STORAGE_KEY_EXPANDED)
        .ok()
        .flatten()
        .unwrap_or_default();
    if raw.is_empty() {
        return HashSet::new();
    }
    serde_json::from_str::<Vec<String>>(&raw)
        .map(|v| v.into_iter().collect())
        .unwrap_or_default()
}

fn write_expanded(set: &HashSet<String>) {
    let Some(win) = window() else { return };
    let Ok(Some(storage)) = win.local_storage() else { return };
    let v: Vec<&String> = set.iter().collect();
    if let Ok(raw) = serde_json::to_string(&v) {
        let _ = storage.set_item(STORAGE_KEY_EXPANDED, &raw);
    }
}

#[derive(Clone, PartialEq)]
struct FlatRow {
    book: String,
    primitive: Option<PrimitiveEntry>,
}

fn build_flat(
    books: &[BookSummary],
    manifests: &HashMap<String, BookManifest>,
    expanded: &HashSet<String>,
) -> Vec<FlatRow> {
    let mut rows = Vec::new();
    for b in books {
        rows.push(FlatRow {
            book: b.name.clone(),
            primitive: None,
        });
        if expanded.contains(&b.name) {
            if let Some(m) = manifests.get(&b.name) {
                for entry in &m.entries {
                    rows.push(FlatRow {
                        book: b.name.clone(),
                        primitive: Some(entry.clone()),
                    });
                }
            }
        }
    }
    rows
}

fn row_matches(row: &FlatRow, sel: &Option<Selection>) -> bool {
    let Some(s) = sel else { return false };
    if s.book != row.book {
        return false;
    }
    match (&row.primitive, &s.primitive) {
        (None, None) => true,
        (Some(a), Some(b)) => a.id == b.id,
        _ => false,
    }
}

#[derive(Properties, PartialEq, Clone)]
struct AddPrimitiveFormProps {
    on_submit: Callback<(String, PrimitiveKind)>,
    on_cancel: Callback<()>,
}

#[function_component(AddPrimitiveForm)]
fn add_primitive_form(props: &AddPrimitiveFormProps) -> Html {
    let name = use_state(String::new);
    let kind = use_state(|| PrimitiveKind::Markdown);

    let on_name_input = {
        let name = name.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(t) = e.target_dyn_into::<HtmlInputElement>() {
                name.set(t.value());
            }
        })
    };

    let on_kind_change = {
        let kind = kind.clone();
        Callback::from(move |e: Event| {
            if let Some(t) = e.target_dyn_into::<HtmlSelectElement>() {
                let k = match t.value().as_str() {
                    "csv" => PrimitiveKind::Csv,
                    "kanban" => PrimitiveKind::Kanban,
                    _ => PrimitiveKind::Markdown,
                };
                kind.set(k);
            }
        })
    };

    let on_submit = {
        let name = name.clone();
        let kind = kind.clone();
        let cb = props.on_submit.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let n = (*name).trim().to_string();
            if n.is_empty() {
                return;
            }
            cb.emit((n, (*kind).clone()));
            name.set(String::new());
        })
    };

    let on_cancel = {
        let cb = props.on_cancel.clone();
        Callback::from(move |_: MouseEvent| cb.emit(()))
    };

    html! {
        <form class="add-prim-form" onsubmit={on_submit}>
            <input
                placeholder="Name"
                value={(*name).clone()}
                oninput={on_name_input}
            />
            <select onchange={on_kind_change}>
                <option value="markdown">{"Markdown"}</option>
                <option value="csv">{"CSV"}</option>
                <option value="kanban">{"Kanban"}</option>
            </select>
            <button type="submit">{"Add"}</button>
            <button type="button" onclick={on_cancel}>{"Cancel"}</button>
        </form>
    }
}

#[function_component(Sidebar)]
pub fn sidebar(props: &SidebarProps) -> Html {
    let expanded = use_state(read_expanded);
    let add_for: UseStateHandle<Option<String>> = use_state(|| None);

    let toggle = {
        let expanded = expanded.clone();
        let on_request_manifest = props.on_request_manifest.clone();
        Callback::from(move |name: String| {
            let mut set = (*expanded).clone();
            if set.contains(&name) {
                set.remove(&name);
            } else {
                set.insert(name.clone());
                on_request_manifest.emit(name);
            }
            write_expanded(&set);
            expanded.set(set);
        })
    };

    let flat = build_flat(&props.books, &props.manifests, &expanded);

    let on_keydown = {
        let flat = flat.clone();
        let active = props.active.clone();
        let on_select = props.on_select.clone();
        let toggle = toggle.clone();
        Callback::from(move |e: KeyboardEvent| {
            let key = e.key();
            if !["ArrowUp", "ArrowDown", "Enter"].contains(&key.as_str()) {
                return;
            }
            if flat.is_empty() {
                return;
            }
            e.prevent_default();
            let cur_idx = flat.iter().position(|r| row_matches(r, &active));
            let next_idx = match (key.as_str(), cur_idx) {
                ("ArrowDown", Some(i)) => (i + 1).min(flat.len() - 1),
                ("ArrowDown", None) => 0,
                ("ArrowUp", Some(i)) => i.saturating_sub(1),
                ("ArrowUp", None) => 0,
                ("Enter", Some(i)) => {
                    let row = &flat[i];
                    if row.primitive.is_none() {
                        toggle.emit(row.book.clone());
                        return;
                    }
                    on_select.emit(Selection {
                        book: row.book.clone(),
                        primitive: row.primitive.clone(),
                    });
                    return;
                }
                _ => 0,
            };
            let row = &flat[next_idx];
            on_select.emit(Selection {
                book: row.book.clone(),
                primitive: row.primitive.clone(),
            });
        })
    };

    let book_items: Vec<Html> = props
        .books
        .iter()
        .map(|b| {
            let name = b.name.clone();
            let is_expanded = expanded.contains(&name);
            let manifest = props.manifests.get(&name).cloned();
            let book_active =
                matches!(&props.active, Some(s) if s.book == name && s.primitive.is_none());
            let on_select_book = {
                let on_select = props.on_select.clone();
                let name = name.clone();
                Callback::from(move |_: MouseEvent| {
                    on_select.emit(Selection {
                        book: name.clone(),
                        primitive: None,
                    });
                })
            };
            let on_toggle = {
                let toggle = toggle.clone();
                let name = name.clone();
                Callback::from(move |e: MouseEvent| {
                    e.stop_propagation();
                    toggle.emit(name.clone());
                })
            };

            let primitives_html: Html = if !is_expanded {
                Html::default()
            } else {
                let entries: Vec<Html> = manifest
                    .as_ref()
                    .map(|m| m.entries.clone())
                    .unwrap_or_default()
                    .into_iter()
                    .map(|entry| {
                        let prim_active = matches!(&props.active, Some(s) if s.book == name && s.primitive.as_ref().map(|p| &p.id) == Some(&entry.id));
                        let on_click = {
                            let on_select = props.on_select.clone();
                            let book = name.clone();
                            let entry = entry.clone();
                            Callback::from(move |_: MouseEvent| {
                                on_select.emit(Selection {
                                    book: book.clone(),
                                    primitive: Some(entry.clone()),
                                });
                            })
                        };
                        let on_del = {
                            let on_delete = props.on_delete_primitive.clone();
                            let book = name.clone();
                            let id = entry.id.clone();
                            Callback::from(move |e: MouseEvent| {
                                e.stop_propagation();
                                on_delete.emit((book.clone(), id.clone()));
                            })
                        };
                        let kind_label = match entry.kind {
                            PrimitiveKind::Markdown => "md",
                            PrimitiveKind::Csv => "csv",
                            PrimitiveKind::Kanban => "kanban",
                        };
                        html! {
                            <li class={classes!("sidebar-primitive", prim_active.then_some("active"))} onclick={on_click}>
                                <span class="prim-kind">{ kind_label }</span>
                                <span class="prim-name">{ &entry.name }</span>
                                <button class="prim-delete" onclick={on_del}>{"×"}</button>
                            </li>
                        }
                    })
                    .collect();

                let add_row: Html = if add_for.as_deref() == Some(name.as_str()) {
                    let on_submit = {
                        let on_add = props.on_add_primitive.clone();
                        let book = name.clone();
                        let add_for = add_for.clone();
                        Callback::from(move |(n, k): (String, PrimitiveKind)| {
                            on_add.emit((book.clone(), n, k));
                            add_for.set(None);
                        })
                    };
                    let on_cancel = {
                        let add_for = add_for.clone();
                        Callback::from(move |_| add_for.set(None))
                    };
                    html! {
                        <li class="sidebar-add-row">
                            <AddPrimitiveForm on_submit={on_submit} on_cancel={on_cancel} />
                        </li>
                    }
                } else {
                    let on_open = {
                        let add_for = add_for.clone();
                        let name = name.clone();
                        Callback::from(move |_: MouseEvent| add_for.set(Some(name.clone())))
                    };
                    html! {
                        <li class="sidebar-add-row">
                            <button class="sidebar-add-prim" onclick={on_open}>{"+ Add primitive"}</button>
                        </li>
                    }
                };

                html! {
                    <ul class="sidebar-primitives">
                        { for entries }
                        { add_row }
                    </ul>
                }
            };

            html! {
                <li class={classes!("sidebar-book", book_active.then_some("active"))}>
                    <div class="sidebar-book-row" onclick={on_select_book}>
                        <button class="sidebar-toggle" onclick={on_toggle}>
                            { if is_expanded { "▾" } else { "▸" } }
                        </button>
                        <span class="sidebar-book-name">{ &name }</span>
                    </div>
                    { primitives_html }
                </li>
            }
        })
        .collect();

    html! {
        <aside class="sidebar" tabindex="0" onkeydown={on_keydown}>
            <h2>{"Books"}</h2>
            <ul class="sidebar-books">
                { for book_items }
            </ul>
        </aside>
    }
}
