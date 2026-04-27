use anyhow::{Context, Result};
use tree_sitter::{Parser, Tree};

use crate::languages::Language;

pub struct ParsedFile {
    pub language: Language,
    pub source: String,
    pub tree: Tree,
}

pub fn parse_source(language: Language, source: String) -> Result<ParsedFile> {
    let mut parser = Parser::new();
    parser
        .set_language(&language.ts_language())
        .with_context(|| format!("loading tree-sitter parser for {}", language.name()))?;
    let tree = parser
        .parse(&source, None)
        .context("tree-sitter returned no parse tree")?;
    Ok(ParsedFile {
        language,
        source,
        tree,
    })
}

pub fn parse_path(path: &std::path::Path) -> Result<ParsedFile> {
    let language = Language::from_path(path)
        .with_context(|| format!("unsupported file extension for {}", path.display()))?;
    let source =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse_source(language, source)
}
