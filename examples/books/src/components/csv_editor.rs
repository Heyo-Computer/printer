use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::storage::{self, CsvData};

#[derive(Properties, PartialEq, Clone)]
pub struct CsvEditorProps {
    pub book: String,
    pub filename: String,
}

fn cols_count(data: &CsvData) -> usize {
    data.headers
        .len()
        .max(data.rows.iter().map(|r| r.len()).max().unwrap_or(0))
}

fn normalize(data: &mut CsvData) {
    let cols = cols_count(data);
    while data.headers.len() < cols {
        data.headers.push(format!("col{}", data.headers.len() + 1));
    }
    for row in &mut data.rows {
        while row.len() < cols {
            row.push(String::new());
        }
    }
}

#[function_component(CsvEditor)]
pub fn csv_editor(props: &CsvEditorProps) -> Html {
    let data = use_state(CsvData::default);
    let saved = use_state(CsvData::default);
    let loading = use_state(|| true);
    let saving = use_state(|| false);
    let error = use_state(|| Option::<String>::None);

    {
        let book = props.book.clone();
        let filename = props.filename.clone();
        let data = data.clone();
        let saved = saved.clone();
        let loading = loading.clone();
        let error = error.clone();
        use_effect_with((book.clone(), filename.clone()), move |_| {
            loading.set(true);
            spawn_local(async move {
                match storage::read_csv(&book, &filename).await {
                    Ok(mut d) => {
                        normalize(&mut d);
                        data.set(d.clone());
                        saved.set(d);
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
            || {}
        });
    }

    let dirty = *data != *saved;

    let mutate = {
        let data = data.clone();
        move |f: Box<dyn FnOnce(&mut CsvData)>| {
            let mut next = (*data).clone();
            f(&mut next);
            normalize(&mut next);
            data.set(next);
        }
    };

    let on_header_input = {
        let data = data.clone();
        Callback::from(move |(idx, value): (usize, String)| {
            let mut next = (*data).clone();
            if idx < next.headers.len() {
                next.headers[idx] = value;
            }
            data.set(next);
        })
    };

    let on_cell_input = {
        let data = data.clone();
        Callback::from(move |(r, c, value): (usize, usize, String)| {
            let mut next = (*data).clone();
            if r < next.rows.len() && c < next.rows[r].len() {
                next.rows[r][c] = value;
            }
            data.set(next);
        })
    };

    let add_row = {
        let mutate = mutate.clone();
        Callback::from(move |_: MouseEvent| {
            mutate(Box::new(|d| {
                let cols = cols_count(d).max(1);
                d.rows.push(vec![String::new(); cols]);
            }));
        })
    };

    let add_col = {
        let mutate = mutate.clone();
        Callback::from(move |_: MouseEvent| {
            mutate(Box::new(|d| {
                d.headers.push(format!("col{}", d.headers.len() + 1));
                for row in &mut d.rows {
                    row.push(String::new());
                }
            }));
        })
    };

    let remove_row = {
        let mutate = mutate.clone();
        Callback::from(move |idx: usize| {
            mutate(Box::new(move |d| {
                if idx < d.rows.len() {
                    d.rows.remove(idx);
                }
            }));
        })
    };

    let remove_col = {
        let mutate = mutate.clone();
        Callback::from(move |idx: usize| {
            mutate(Box::new(move |d| {
                if idx < d.headers.len() {
                    d.headers.remove(idx);
                }
                for row in &mut d.rows {
                    if idx < row.len() {
                        row.remove(idx);
                    }
                }
            }));
        })
    };

    let do_save = {
        let book = props.book.clone();
        let filename = props.filename.clone();
        let data = data.clone();
        let saved = saved.clone();
        let saving = saving.clone();
        let error = error.clone();
        Callback::from(move |_: MouseEvent| {
            if *saving {
                return;
            }
            let book = book.clone();
            let filename = filename.clone();
            let current = (*data).clone();
            let saved = saved.clone();
            let saving = saving.clone();
            let error = error.clone();
            saving.set(true);
            spawn_local(async move {
                match storage::write_csv(&book, &filename, &current).await {
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

    if *loading {
        return html! { <div class="csv-editor"><p>{"Loading…"}</p></div> };
    }

    let cols = cols_count(&data);

    html! {
        <div class="csv-editor">
            <div class="editor-toolbar">
                <span class="filename">{ &props.filename }</span>
                <span class="dirty-indicator">
                    { if dirty { "● unsaved" } else { "✓ saved" } }
                </span>
                <button onclick={add_row.clone()}>{"+ Row"}</button>
                <button onclick={add_col.clone()}>{"+ Column"}</button>
                <button onclick={do_save} disabled={!dirty || *saving}>
                    { if *saving { "Saving…" } else { "Save" } }
                </button>
            </div>
            if let Some(msg) = &*error {
                <p class="error">{ format!("Error: {msg}") }</p>
            }
            <div class="csv-table-wrap">
                <table class="csv-table">
                    <thead>
                        <tr>
                            { for (0..cols).map(|c| {
                                let on_header_input = on_header_input.clone();
                                let remove_col = remove_col.clone();
                                let value = data.headers.get(c).cloned().unwrap_or_default();
                                let oninput = Callback::from(move |e: InputEvent| {
                                    if let Some(t) = e.target_dyn_into::<HtmlInputElement>() {
                                        on_header_input.emit((c, t.value()));
                                    }
                                });
                                let onclick = Callback::from(move |_: MouseEvent| remove_col.emit(c));
                                html! {
                                    <th>
                                        <input class="csv-header" {value} {oninput} />
                                        <button class="csv-remove" {onclick} title="Remove column">{"×"}</button>
                                    </th>
                                }
                            }) }
                            <th class="csv-actions"></th>
                        </tr>
                    </thead>
                    <tbody>
                        { for data.rows.iter().enumerate().map(|(r, row)| {
                            let row = row.clone();
                            let remove_row = remove_row.clone();
                            let on_cell_input = on_cell_input.clone();
                            html! {
                                <tr>
                                    { for (0..cols).map(|c| {
                                        let value = row.get(c).cloned().unwrap_or_default();
                                        let on_cell_input = on_cell_input.clone();
                                        let oninput = Callback::from(move |e: InputEvent| {
                                            if let Some(t) = e.target_dyn_into::<HtmlInputElement>() {
                                                on_cell_input.emit((r, c, t.value()));
                                            }
                                        });
                                        html! {
                                            <td><input class="csv-cell" {value} {oninput} /></td>
                                        }
                                    }) }
                                    <td class="csv-actions">
                                        <button
                                            class="csv-remove"
                                            onclick={Callback::from(move |_: MouseEvent| remove_row.emit(r))}
                                            title="Remove row"
                                        >{"×"}</button>
                                    </td>
                                </tr>
                            }
                        }) }
                    </tbody>
                </table>
            </div>
        </div>
    }
}
