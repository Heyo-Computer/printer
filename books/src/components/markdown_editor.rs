use pulldown_cmark::{html, Options, Parser};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlTextAreaElement;
use yew::prelude::*;

use crate::storage;

#[derive(Properties, PartialEq, Clone)]
pub struct MarkdownEditorProps {
    pub book: String,
    pub filename: String,
}

fn render_markdown(src: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(src, opts);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

#[function_component(MarkdownEditor)]
pub fn markdown_editor(props: &MarkdownEditorProps) -> Html {
    let source = use_state(String::new);
    let saved = use_state(String::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let saving = use_state(|| false);

    {
        let book = props.book.clone();
        let filename = props.filename.clone();
        let source = source.clone();
        let saved = saved.clone();
        let loading = loading.clone();
        let error = error.clone();
        use_effect_with((book.clone(), filename.clone()), move |_| {
            loading.set(true);
            spawn_local(async move {
                match storage::read_file(&book, &filename).await {
                    Ok(text) => {
                        source.set(text.clone());
                        saved.set(text);
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
            || {}
        });
    }

    let dirty = *source != *saved;

    let on_input = {
        let source = source.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(target) = e.target_dyn_into::<HtmlTextAreaElement>() {
                source.set(target.value());
            }
        })
    };

    let do_save = {
        let book = props.book.clone();
        let filename = props.filename.clone();
        let source = source.clone();
        let saved = saved.clone();
        let saving = saving.clone();
        let error = error.clone();
        Callback::from(move |_| {
            if *saving {
                return;
            }
            let book = book.clone();
            let filename = filename.clone();
            let current: String = (*source).clone();
            let saved = saved.clone();
            let saving = saving.clone();
            let error = error.clone();
            saving.set(true);
            spawn_local(async move {
                match storage::write_file(&book, &filename, &current).await {
                    Ok(()) => {
                        saved.set(current);
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                saving.set(false);
            });
        })
    };

    let on_blur = {
        let do_save = do_save.clone();
        Callback::from(move |_: FocusEvent| {
            do_save.emit(());
        })
    };

    let on_save_click = {
        let do_save = do_save.clone();
        Callback::from(move |_: MouseEvent| do_save.emit(()))
    };

    let preview_html = render_markdown(&source);
    let preview = Html::from_html_unchecked(AttrValue::from(preview_html));

    if *loading {
        return html! { <div class="markdown-editor"><p>{"Loading…"}</p></div> };
    }

    html! {
        <div class="markdown-editor">
            <div class="editor-toolbar">
                <span class="filename">{ &props.filename }</span>
                <span class="dirty-indicator">
                    { if dirty { "● unsaved" } else { "✓ saved" } }
                </span>
                <button onclick={on_save_click} disabled={!dirty || *saving}>
                    { if *saving { "Saving…" } else { "Save" } }
                </button>
            </div>
            if let Some(msg) = &*error {
                <p class="error">{ format!("Error: {msg}") }</p>
            }
            <div class="editor-panes">
                <textarea
                    class="editor-source"
                    value={(*source).clone()}
                    oninput={on_input}
                    onblur={on_blur}
                />
                <div class="editor-preview">
                    { preview }
                </div>
            </div>
        </div>
    }
}
