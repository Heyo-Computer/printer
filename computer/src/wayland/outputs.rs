use anyhow::{Context, Result};
use serde::Serialize;
use smithay_client_toolkit::{
    delegate_output, delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use wayland_client::{Connection, QueueHandle, globals::registry_queue_init, protocol::wl_output};

#[derive(Serialize, Debug, Clone)]
pub struct OutputInfo {
    pub name: String,
    pub make: String,
    pub model: String,
    pub description: String,
    pub x: i32,
    pub y: i32,
    pub width_px: i32,
    pub height_px: i32,
    pub refresh_mhz: i32,
    pub scale: i32,
    pub transform: String,
}

struct App {
    registry_state: RegistryState,
    output_state: OutputState,
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers!(OutputState);
}

delegate_output!(App);
delegate_registry!(App);

pub fn collect() -> Result<Vec<OutputInfo>> {
    let conn = Connection::connect_to_env().context("connect to wayland")?;
    let (globals, mut event_queue) = registry_queue_init::<App>(&conn).context("registry init")?;
    let qh = event_queue.handle();

    let mut app = App {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
    };

    // Pump the queue until output info is populated.
    event_queue.roundtrip(&mut app)?;
    event_queue.roundtrip(&mut app)?;

    let mut result = Vec::new();
    for output in app.output_state.outputs() {
        let info = match app.output_state.info(&output) {
            Some(i) => i,
            None => continue,
        };
        let (x, y) = info.location;
        let (width_px, height_px, refresh_mhz) = info
            .modes
            .iter()
            .find(|m| m.current)
            .map(|m| (m.dimensions.0, m.dimensions.1, m.refresh_rate))
            .unwrap_or((info.logical_size.unwrap_or((0, 0)).0, info.logical_size.unwrap_or((0, 0)).1, 0));

        result.push(OutputInfo {
            name: info.name.clone().unwrap_or_default(),
            make: info.make.clone(),
            model: info.model.clone(),
            description: info.description.clone().unwrap_or_default(),
            x,
            y,
            width_px,
            height_px,
            refresh_mhz,
            scale: info.scale_factor,
            transform: format!("{:?}", info.transform),
        });
    }
    Ok(result)
}

pub fn run(json: bool) -> Result<()> {
    let outputs = collect()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&outputs)?);
    } else if outputs.is_empty() {
        println!("(no outputs reported)");
    } else {
        for o in &outputs {
            println!(
                "{}\t{}x{}@{}.{:03}Hz\tpos={},{}\tscale={}\t{} {}",
                o.name,
                o.width_px,
                o.height_px,
                o.refresh_mhz / 1000,
                o.refresh_mhz % 1000,
                o.x,
                o.y,
                o.scale,
                o.make,
                o.model,
            );
        }
    }
    Ok(())
}
