use anyhow::{Context, Result, anyhow, bail};
use core_graphics::display::{CGDirectDisplayID, CGDisplay};
use std::io::{BufWriter, Write};

pub fn run(target_output: Option<&str>, file: Option<&str>) -> Result<()> {
    super::perms::require_screen_recording().context("screen recording permission")?;

    let display_id = pick_display(target_output)?;
    let display = CGDisplay::new(display_id);
    let image = display
        .image()
        .ok_or_else(|| anyhow!("CGDisplayCreateImage returned null for display {display_id}"))?;

    let width = image.width() as u32;
    let height = image.height() as u32;
    let bytes_per_row = image.bytes_per_row();
    let bits_per_pixel = image.bits_per_pixel();
    if bits_per_pixel != 32 {
        bail!("unsupported bits-per-pixel from CGImage: {bits_per_pixel}");
    }

    let data = image.data();
    let src = data.bytes();
    let rgba = bgra_to_rgba(src, width, height, bytes_per_row);

    let img = image::RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| anyhow!("invalid image dimensions {width}x{height}"))?;
    let mut png: Vec<u8> = Vec::with_capacity((width as usize) * (height as usize));
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;

    let mut out: Box<dyn Write> = match file {
        Some(path) => Box::new(BufWriter::new(
            std::fs::File::create(path).context("create output file")?,
        )),
        None => Box::new(BufWriter::new(std::io::stdout().lock())),
    };
    out.write_all(&png)?;
    out.flush()?;
    Ok(())
}

fn pick_display(target: Option<&str>) -> Result<CGDirectDisplayID> {
    let ids =
        CGDisplay::active_displays().map_err(|e| anyhow!("CGGetActiveDisplayList failed: {e}"))?;
    if ids.is_empty() {
        bail!("no active displays");
    }
    let Some(name) = target else {
        return Ok(ids[0]);
    };
    let want = name.strip_prefix("display-").unwrap_or(name);
    let parsed: CGDirectDisplayID = want
        .parse()
        .with_context(|| format!("output name {name:?} is not display-<id>"))?;
    if ids.contains(&parsed) {
        Ok(parsed)
    } else {
        bail!("no active display with id {parsed} (try `computer outputs`)");
    }
}

fn bgra_to_rgba(src: &[u8], w: u32, h: u32, stride: usize) -> Vec<u8> {
    let row_bytes = (w as usize) * 4;
    let mut out = vec![0u8; (w as usize) * (h as usize) * 4];
    for y in 0..(h as usize) {
        let row = &src[y * stride..y * stride + row_bytes];
        let dst = &mut out[y * row_bytes..(y + 1) * row_bytes];
        for x in 0..(w as usize) {
            let s = &row[x * 4..x * 4 + 4];
            let d = &mut dst[x * 4..x * 4 + 4];
            d[0] = s[2]; // R
            d[1] = s[1]; // G
            d[2] = s[0]; // B
            d[3] = s[3]; // A
        }
    }
    out
}
