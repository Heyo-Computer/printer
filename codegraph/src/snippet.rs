use anyhow::{Result, anyhow, bail};
use serde::Serialize;

use crate::parse::ParsedFile;
use crate::symbols::{self, Symbol};

#[derive(Serialize, Debug)]
pub struct Snippet {
    pub file: String,
    pub symbol: Option<String>,
    pub start_line: u32,
    pub end_line: u32,
    pub source: String,
}

pub fn by_symbol(parsed: &ParsedFile, query: &str) -> Result<(Symbol, String)> {
    let symbols = symbols::extract(parsed);
    let matches: Vec<&Symbol> = symbols
        .iter()
        .filter(|s| s.qualified == query || s.name == query)
        .collect();
    let chosen = match matches.len() {
        0 => return Err(anyhow!("no symbol named `{query}` found")),
        1 => matches[0].clone(),
        _ => {
            // Prefer an exact qualified match if any.
            let exact = matches
                .iter()
                .find(|s| s.qualified == query)
                .copied()
                .cloned();
            match exact {
                Some(s) => s,
                None => {
                    let names: Vec<_> = matches.iter().map(|s| s.qualified.clone()).collect();
                    return Err(anyhow!("symbol `{query}` is ambiguous: {names:?}"));
                }
            }
        }
    };
    let src = parsed.source.as_bytes();
    let body = String::from_utf8_lossy(&src[chosen.start_byte..chosen.end_byte]).into_owned();
    Ok((chosen, body))
}

pub fn by_lines(parsed: &ParsedFile, range: &str) -> Result<(u32, u32, String)> {
    let (start, end) = parse_range(range)?;
    let lines: Vec<&str> = parsed.source.lines().collect();
    if start == 0 || start as usize > lines.len() {
        bail!("start line {start} out of range (file has {} lines)", lines.len());
    }
    let end = end.min(lines.len() as u32);
    let slice = &lines[(start - 1) as usize..end as usize];
    Ok((start, end, slice.join("\n")))
}

fn parse_range(s: &str) -> Result<(u32, u32)> {
    let (a, b) = s
        .split_once(':')
        .or_else(|| s.split_once('-'))
        .ok_or_else(|| anyhow!("expected `start:end` or `start-end`, got `{s}`"))?;
    let start: u32 = a.parse().map_err(|_| anyhow!("bad start line `{a}`"))?;
    let end: u32 = b.parse().map_err(|_| anyhow!("bad end line `{b}`"))?;
    if end < start {
        bail!("end line {end} is before start line {start}");
    }
    Ok((start, end))
}
