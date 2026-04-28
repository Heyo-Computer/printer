use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, event_created_child,
    globals::{GlobalListContents, registry_queue_init},
    protocol::wl_registry,
};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};

#[derive(Serialize, Debug, Clone, Default)]
pub struct WindowInfo {
    pub identifier: String,
    pub title: String,
    pub app_id: String,
}

struct App {
    handles: HashMap<u32, WindowInfo>,
    finished: bool,
    initial_done: bool,
    pending_done: usize,
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for App {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtForeignToplevelListV1, ()> for App {
    fn event(
        state: &mut Self,
        _: &ExtForeignToplevelListV1,
        event: ext_foreign_toplevel_list_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } => {
                state.handles.insert(toplevel.id().protocol_id(), WindowInfo::default());
                state.pending_done += 1;
            }
            ext_foreign_toplevel_list_v1::Event::Finished => {
                state.finished = true;
            }
            _ => {}
        }
    }

    event_created_child!(App, ExtForeignToplevelListV1, [
        0 => (ExtForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for App {
    fn event(
        state: &mut Self,
        proxy: &ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let key = proxy.id().protocol_id();
        let info = state.handles.entry(key).or_default();
        match event {
            ext_foreign_toplevel_handle_v1::Event::Title { title } => info.title = title,
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => info.app_id = app_id,
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => info.identifier = identifier,
            ext_foreign_toplevel_handle_v1::Event::Done => {
                state.initial_done = true;
            }
            ext_foreign_toplevel_handle_v1::Event::Closed => {
                state.handles.remove(&key);
            }
            _ => {}
        }
    }
}

pub fn collect() -> Result<Vec<WindowInfo>> {
    let conn = Connection::connect_to_env().context("connect to wayland")?;
    let (globals, mut event_queue) = registry_queue_init::<App>(&conn).context("registry init")?;
    let qh = event_queue.handle();

    let _list: ExtForeignToplevelListV1 = globals
        .bind::<ExtForeignToplevelListV1, _, _>(&qh, 1..=1, ())
        .context("compositor doesn't support ext_foreign_toplevel_list_v1")?;

    let mut app = App {
        handles: HashMap::new(),
        finished: false,
        initial_done: false,
        pending_done: 0,
    };

    // Roundtrip a few times to receive toplevel + done events.
    for _ in 0..4 {
        event_queue.roundtrip(&mut app)?;
        if app.initial_done && app.pending_done > 0 {
            break;
        }
    }

    let mut out: Vec<WindowInfo> = app.handles.into_values().filter(|w| !w.identifier.is_empty() || !w.app_id.is_empty() || !w.title.is_empty()).collect();
    out.sort_by(|a, b| a.app_id.cmp(&b.app_id).then(a.title.cmp(&b.title)));
    let _ = app.finished;
    Ok(out)
}

pub fn run(json: bool) -> Result<()> {
    let windows = collect()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&windows)?);
    } else if windows.is_empty() {
        println!("(no toplevels reported)");
    } else {
        for w in &windows {
            println!("{}\t{}\t{}", w.identifier, w.app_id, w.title);
        }
    }
    Ok(())
}
