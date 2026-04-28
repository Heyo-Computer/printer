use anyhow::{Context, Result};
use core_graphics::display::{CGDirectDisplayID, CGDisplay};
use serde::Serialize;

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

pub fn collect() -> Result<Vec<OutputInfo>> {
    let ids = CGDisplay::active_displays()
        .map_err(|e| anyhow::anyhow!("CGGetActiveDisplayList failed: {e}"))?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(describe(id));
    }
    Ok(out)
}

fn describe(id: CGDirectDisplayID) -> OutputInfo {
    let d = CGDisplay::new(id);
    let bounds = d.bounds();
    let pixels_wide = d.pixels_wide() as i32;
    let pixels_high = d.pixels_high() as i32;
    let mode = d.display_mode();

    // refresh_rate is Hz as f64; convert to mHz to match Linux's units.
    let refresh_mhz = mode
        .as_ref()
        .map(|m| (m.refresh_rate() * 1000.0) as i32)
        .unwrap_or(0);

    // Backing scale = pixel size / point size (rounded).
    let scale = if bounds.size.width > 0.0 {
        ((pixels_wide as f64) / bounds.size.width).round() as i32
    } else {
        1
    };

    let mut description = if d.is_main() { "main".to_string() } else { String::new() };
    if d.is_builtin() {
        if !description.is_empty() {
            description.push_str(", ");
        }
        description.push_str("builtin");
    }

    OutputInfo {
        name: format!("display-{id}"),
        make: format!("vendor-{}", d.vendor_number()),
        model: format!("model-{}", d.model_number()),
        description,
        x: bounds.origin.x as i32,
        y: bounds.origin.y as i32,
        width_px: pixels_wide,
        height_px: pixels_high,
        refresh_mhz,
        scale,
        transform: format!("rotation={}", d.rotation() as i32),
    }
}

pub fn run(json: bool) -> Result<()> {
    let outputs = collect().context("enumerate displays")?;
    if json {
        println!("{}", serde_json::to_string_pretty(&outputs)?);
    } else if outputs.is_empty() {
        println!("(no displays reported)");
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
