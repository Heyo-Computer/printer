use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

use crate::storage::{self, CsvData};

#[derive(Properties, PartialEq, Clone)]
pub struct KanbanBoardProps {
    pub book: String,
    pub filename: String,
}

const SCHEMA: &[&str] = &["id", "title", "status", "notes", "order"];
const DEFAULT_STATUSES: &[&str] = &["todo", "in_progress", "done"];

#[derive(Debug, Clone, PartialEq)]
struct Card {
    id: String,
    title: String,
    status: String,
    notes: String,
    order: i64,
}

fn col_index(headers: &[String], name: &str) -> Option<usize> {
    headers.iter().position(|h| h == name)
}

fn parse_cards(data: &CsvData) -> Vec<Card> {
    let id_i = col_index(&data.headers, "id");
    let title_i = col_index(&data.headers, "title");
    let status_i = col_index(&data.headers, "status");
    let notes_i = col_index(&data.headers, "notes");
    let order_i = col_index(&data.headers, "order");
    data.rows
        .iter()
        .map(|row| Card {
            id: id_i.and_then(|i| row.get(i).cloned()).unwrap_or_default(),
            title: title_i.and_then(|i| row.get(i).cloned()).unwrap_or_default(),
            status: status_i
                .and_then(|i| row.get(i).cloned())
                .unwrap_or_else(|| DEFAULT_STATUSES[0].to_string()),
            notes: notes_i.and_then(|i| row.get(i).cloned()).unwrap_or_default(),
            order: order_i
                .and_then(|i| row.get(i).and_then(|s| s.parse::<i64>().ok()))
                .unwrap_or(0),
        })
        .collect()
}

fn cards_to_data(cards: &[Card]) -> CsvData {
    let headers: Vec<String> = SCHEMA.iter().map(|s| s.to_string()).collect();
    let rows = cards
        .iter()
        .map(|c| {
            vec![
                c.id.clone(),
                c.title.clone(),
                c.status.clone(),
                c.notes.clone(),
                c.order.to_string(),
            ]
        })
        .collect();
    CsvData { headers, rows }
}

fn statuses_in_order(cards: &[Card]) -> Vec<String> {
    let mut seen: Vec<String> = DEFAULT_STATUSES.iter().map(|s| s.to_string()).collect();
    for c in cards {
        if !seen.contains(&c.status) {
            seen.push(c.status.clone());
        }
    }
    seen
}

#[derive(Properties, PartialEq, Clone)]
struct CardViewProps {
    card: Card,
    is_editing: bool,
    on_move: Callback<i32>,
    on_delete: Callback<()>,
    on_toggle_edit: Callback<()>,
    on_edit_title: Callback<String>,
    on_edit_notes: Callback<String>,
}

#[function_component(CardView)]
fn card_view(p: &CardViewProps) -> Html {
    let on_move_left = {
        let on_move = p.on_move.clone();
        Callback::from(move |_: MouseEvent| on_move.emit(-1))
    };
    let on_move_right = {
        let on_move = p.on_move.clone();
        Callback::from(move |_: MouseEvent| on_move.emit(1))
    };
    let on_delete = {
        let cb = p.on_delete.clone();
        Callback::from(move |_: MouseEvent| cb.emit(()))
    };
    let on_toggle = {
        let cb = p.on_toggle_edit.clone();
        Callback::from(move |_: MouseEvent| cb.emit(()))
    };
    let on_title_input = {
        let cb = p.on_edit_title.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(t) = e.target_dyn_into::<HtmlInputElement>() {
                cb.emit(t.value());
            }
        })
    };
    let on_notes_input = {
        let cb = p.on_edit_notes.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(t) = e.target_dyn_into::<HtmlTextAreaElement>() {
                cb.emit(t.value());
            }
        })
    };

    html! {
        <div class="kanban-card">
            if p.is_editing {
                <input
                    class="kanban-card-title-edit"
                    value={p.card.title.clone()}
                    oninput={on_title_input}
                />
                <textarea
                    class="kanban-card-notes-edit"
                    value={p.card.notes.clone()}
                    oninput={on_notes_input}
                />
            } else {
                <div class="kanban-card-title">{ &p.card.title }</div>
                if !p.card.notes.is_empty() {
                    <div class="kanban-card-notes">{ &p.card.notes }</div>
                }
            }
            <div class="kanban-card-actions">
                <button onclick={on_move_left}>{"←"}</button>
                <button onclick={on_toggle}>
                    { if p.is_editing { "Done" } else { "Edit" } }
                </button>
                <button onclick={on_delete}>{"Delete"}</button>
                <button onclick={on_move_right}>{"→"}</button>
            </div>
        </div>
    }
}

#[function_component(KanbanBoard)]
pub fn kanban_board(props: &KanbanBoardProps) -> Html {
    let cards = use_state(Vec::<Card>::new);
    let loading = use_state(|| true);
    let saving = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let editing: UseStateHandle<Option<String>> = use_state(|| None);
    let new_title = use_state(String::new);

    {
        let book = props.book.clone();
        let filename = props.filename.clone();
        let cards = cards.clone();
        let loading = loading.clone();
        let error = error.clone();
        use_effect_with((book.clone(), filename.clone()), move |_| {
            loading.set(true);
            spawn_local(async move {
                match storage::read_csv(&book, &filename).await {
                    Ok(data) => {
                        cards.set(parse_cards(&data));
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
            || {}
        });
    }

    let persist: Callback<Vec<Card>> = {
        let book = props.book.clone();
        let filename = props.filename.clone();
        let cards = cards.clone();
        let saving = saving.clone();
        let error = error.clone();
        Callback::from(move |next: Vec<Card>| {
            cards.set(next.clone());
            let book = book.clone();
            let filename = filename.clone();
            let saving = saving.clone();
            let error = error.clone();
            saving.set(true);
            spawn_local(async move {
                let data = cards_to_data(&next);
                match storage::write_csv(&book, &filename, &data).await {
                    Ok(()) => error.set(None),
                    Err(e) => error.set(Some(e.to_string())),
                }
                saving.set(false);
            });
        })
    };

    let on_new_title_input = {
        let new_title = new_title.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(t) = e.target_dyn_into::<HtmlInputElement>() {
                new_title.set(t.value());
            }
        })
    };

    let add_card = {
        let cards = cards.clone();
        let new_title = new_title.clone();
        let persist = persist.clone();
        Callback::from(move |_: MouseEvent| {
            let title = (*new_title).trim().to_string();
            if title.is_empty() {
                return;
            }
            let mut next = (*cards).clone();
            let status = DEFAULT_STATUSES[0].to_string();
            let order = next
                .iter()
                .filter(|c| c.status == status)
                .map(|c| c.order)
                .max()
                .unwrap_or(-1)
                + 1;
            let persist = persist.clone();
            let new_title = new_title.clone();
            spawn_local(async move {
                let id = storage::generate_id().await.unwrap_or_default();
                next.push(Card {
                    id,
                    title,
                    status,
                    notes: String::new(),
                    order,
                });
                new_title.set(String::new());
                persist.emit(next);
            });
        })
    };

    if *loading {
        return html! { <div class="kanban"><p>{"Loading…"}</p></div> };
    }

    let statuses = statuses_in_order(&cards);

    let move_cb: Callback<(String, i32)> = {
        let cards = cards.clone();
        let persist = persist.clone();
        Callback::from(move |(id, dir): (String, i32)| {
            let mut next = (*cards).clone();
            let statuses = statuses_in_order(&next);
            let Some(card) = next.iter().find(|c| c.id == id) else {
                return;
            };
            let cur_idx = statuses.iter().position(|s| s == &card.status).unwrap_or(0);
            let new_idx = (cur_idx as i32 + dir).clamp(0, statuses.len() as i32 - 1) as usize;
            if new_idx == cur_idx {
                return;
            }
            let new_status = statuses[new_idx].clone();
            let new_order = next
                .iter()
                .filter(|c| c.status == new_status)
                .map(|c| c.order)
                .max()
                .unwrap_or(-1)
                + 1;
            if let Some(card) = next.iter_mut().find(|c| c.id == id) {
                card.status = new_status;
                card.order = new_order;
            }
            persist.emit(next);
        })
    };

    let delete_cb: Callback<String> = {
        let cards = cards.clone();
        let persist = persist.clone();
        Callback::from(move |id: String| {
            let mut next = (*cards).clone();
            next.retain(|c| c.id != id);
            persist.emit(next);
        })
    };

    let edit_title_cb: Callback<(String, String)> = {
        let cards = cards.clone();
        let persist = persist.clone();
        Callback::from(move |(id, value): (String, String)| {
            let mut next = (*cards).clone();
            if let Some(card) = next.iter_mut().find(|c| c.id == id) {
                card.title = value;
            }
            persist.emit(next);
        })
    };

    let edit_notes_cb: Callback<(String, String)> = {
        let cards = cards.clone();
        let persist = persist.clone();
        Callback::from(move |(id, value): (String, String)| {
            let mut next = (*cards).clone();
            if let Some(card) = next.iter_mut().find(|c| c.id == id) {
                card.notes = value;
            }
            persist.emit(next);
        })
    };

    let toggle_edit_cb: Callback<String> = {
        let editing = editing.clone();
        Callback::from(move |id: String| {
            if editing.as_deref() == Some(id.as_str()) {
                editing.set(None);
            } else {
                editing.set(Some(id));
            }
        })
    };

    html! {
        <div class="kanban">
            <div class="editor-toolbar">
                <span class="filename">{ &props.filename }</span>
                <input
                    class="kanban-new"
                    placeholder="New card title…"
                    value={(*new_title).clone()}
                    oninput={on_new_title_input}
                />
                <button onclick={add_card}>{"+ Add"}</button>
                if *saving {
                    <span class="dirty-indicator">{"Saving…"}</span>
                }
            </div>
            if let Some(msg) = &*error {
                <p class="error">{ format!("Error: {msg}") }</p>
            }
            <div class="kanban-columns">
                { for statuses.iter().map(|status| {
                    let mut col_cards: Vec<&Card> = cards.iter().filter(|c| &c.status == status).collect();
                    col_cards.sort_by_key(|c| c.order);
                    html! {
                        <div class="kanban-column">
                            <h3 class="kanban-status">{ status }</h3>
                            { for col_cards.into_iter().map(|c| {
                                let id = c.id.clone();
                                let is_editing = editing.as_deref() == Some(id.as_str());
                                let move_cb = move_cb.clone();
                                let delete_cb = delete_cb.clone();
                                let edit_title_cb = edit_title_cb.clone();
                                let edit_notes_cb = edit_notes_cb.clone();
                                let toggle_edit_cb = toggle_edit_cb.clone();
                                let id_m = id.clone();
                                let id_d = id.clone();
                                let id_te = id.clone();
                                let id_t = id.clone();
                                let id_n = id.clone();
                                html! {
                                    <CardView
                                        card={c.clone()}
                                        is_editing={is_editing}
                                        on_move={Callback::from(move |dir: i32| move_cb.emit((id_m.clone(), dir)))}
                                        on_delete={Callback::from(move |_| delete_cb.emit(id_d.clone()))}
                                        on_toggle_edit={Callback::from(move |_| toggle_edit_cb.emit(id_te.clone()))}
                                        on_edit_title={Callback::from(move |v: String| edit_title_cb.emit((id_t.clone(), v)))}
                                        on_edit_notes={Callback::from(move |v: String| edit_notes_cb.emit((id_n.clone(), v)))}
                                    />
                                }
                            }) }
                        </div>
                    }
                }) }
            </div>
        </div>
    }
}
