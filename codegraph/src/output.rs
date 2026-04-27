use anyhow::Result;
use serde::Serialize;

#[derive(Copy, Clone, Debug)]
pub enum Format {
    Json,
    Text,
}

pub fn print_json<T: Serialize>(v: &T) -> Result<()> {
    let s = serde_json::to_string_pretty(v)?;
    println!("{s}");
    Ok(())
}
