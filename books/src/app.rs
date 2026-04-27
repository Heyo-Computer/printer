use std::collections::HashMap;

use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::components::sidebar::Selection;
use crate::components::{CsvEditor, KanbanBoard, MarkdownEditor, Sidebar};
use crate::storage::{self, BookManifest, BookSummary, PrimitiveKind};

#[function_component(App)]
pub fn app() -> Html {
    let books = use_state(Vec::<BookSummary>::new);
    let manifests: UseStateHandle<HashMap<String, BookManifest>> = use_state(HashMap::new);
    let active: UseStateHandle<Option<Selection>> = use_state(|| None);
    let error = use_state(|| Option::<String>::None);
    let new_name = use_state(String::new);
    let new_desc = use_state(String::new);

    let reload_books = {
        let books = books.clone();
        let error = error.clone();
        std::rc::Rc::new(move || {
            let books = books.clone();
            let error = error.clone();
            spawn_local(async move {
                match storage::list_books().await {
                    Ok(list) => {
                        books.set(list);
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
            });
        })
    };

    let load_manifest = {
        let manifests = manifests.clone();
        let error = error.clone();
        std::rc::Rc::new(move |book: String| {
            let manifests = manifests.clone();
            let error = error.clone();
            spawn_local(async move {
                match storage::get_manifest(&book).await {
                    Ok(m) => {
                        let mut next = (*manifests).clone();
                        next.insert(book, m);
                        manifests.set(next);
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
            });
        })
    };

    {
        let reload_books = reload_books.clone();
        use_effect_with((), move |_| {
            reload_books();
            || {}
        });
    }

    let on_select = {
        let active = active.clone();
        Callback::from(move |sel: Selection| active.set(Some(sel)))
    };

    let on_request_manifest = {
        let load_manifest = load_manifest.clone();
        Callback::from(move |book: String| load_manifest(book))
    };

    let on_add_primitive = {
        let load_manifest = load_manifest.clone();
        let reload_books = reload_books.clone();
        let error = error.clone();
        Callback::from(move |(book, name, kind): (String, String, PrimitiveKind)| {
            let load_manifest = load_manifest.clone();
            let reload_books = reload_books.clone();
            let error = error.clone();
            spawn_local(async move {
                match storage::add_primitive(&book, &name, &kind).await {
                    Ok(_) => {
                        load_manifest(book);
                        reload_books();
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
            });
        })
    };

    let on_delete_primitive = {
        let load_manifest = load_manifest.clone();
        let reload_books = reload_books.clone();
        let active = active.clone();
        let error = error.clone();
        Callback::from(move |(book, id): (String, String)| {
            let load_manifest = load_manifest.clone();
            let reload_books = reload_books.clone();
            let active = active.clone();
            let error = error.clone();
            spawn_local(async move {
                match storage::delete_primitive(&book, &id).await {
                    Ok(_) => {
                        if let Some(sel) = (*active).clone() {
                            if sel.book == book
                                && sel.primitive.as_ref().map(|p| &p.id) == Some(&id)
                            {
                                active.set(None);
                            }
                        }
                        load_manifest(book);
                        reload_books();
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
            });
        })
    };

    let on_name_input = {
        let new_name = new_name.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(t) = e.target_dyn_into::<HtmlInputElement>() {
                new_name.set(t.value());
            }
        })
    };
    let on_desc_input = {
        let new_desc = new_desc.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(t) = e.target_dyn_into::<HtmlInputElement>() {
                new_desc.set(t.value());
            }
        })
    };

    let on_create = {
        let new_name = new_name.clone();
        let new_desc = new_desc.clone();
        let reload_books = reload_books.clone();
        let error = error.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let name = (*new_name).trim().to_string();
            if name.is_empty() {
                return;
            }
            let description = (*new_desc).clone();
            let new_name = new_name.clone();
            let new_desc = new_desc.clone();
            let reload_books = reload_books.clone();
            let error = error.clone();
            spawn_local(async move {
                match storage::create_book(&name, &description).await {
                    Ok(_) => {
                        new_name.set(String::new());
                        new_desc.set(String::new());
                        reload_books();
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
            });
        })
    };

    let editor = match active.as_ref() {
        None => html! {
            <div class="editor-pane-empty">
                {"Select a primitive from the sidebar to start editing."}
            </div>
        },
        Some(sel) => match &sel.primitive {
            None => html! {
                <div class="editor-pane-empty">
                    { format!("Book: {} — pick a primitive.", sel.book) }
                </div>
            },
            Some(prim) => match prim.kind {
                PrimitiveKind::Markdown => html! {
                    <MarkdownEditor
                        key={format!("{}/{}", sel.book, prim.id)}
                        book={sel.book.clone()}
                        filename={prim.filename.clone()}
                    />
                },
                PrimitiveKind::Csv => html! {
                    <CsvEditor
                        key={format!("{}/{}", sel.book, prim.id)}
                        book={sel.book.clone()}
                        filename={prim.filename.clone()}
                    />
                },
                PrimitiveKind::Kanban => html! {
                    <KanbanBoard
                        key={format!("{}/{}", sel.book, prim.id)}
                        book={sel.book.clone()}
                        filename={prim.filename.clone()}
                    />
                },
            },
        },
    };

    html! {
        <div class="app-shell">
            <Sidebar
                books={(*books).clone()}
                manifests={(*manifests).clone()}
                active={(*active).clone()}
                on_select={on_select}
                on_request_manifest={on_request_manifest}
                on_add_primitive={on_add_primitive}
                on_delete_primitive={on_delete_primitive}
            />
            <section class="editor-pane">
                if let Some(msg) = &*error {
                    <p class="error">{ format!("Error: {msg}") }</p>
                }
                { editor }
                <form class="new-book-form" onsubmit={on_create}>
                    <h2>{"New book"}</h2>
                    <input
                        placeholder="Name"
                        value={(*new_name).clone()}
                        oninput={on_name_input}
                    />
                    <input
                        placeholder="Description (optional)"
                        value={(*new_desc).clone()}
                        oninput={on_desc_input}
                    />
                    <button type="submit" disabled={(*new_name).trim().is_empty()}>
                        {"Create"}
                    </button>
                </form>
            </section>
        </div>
    }
}
