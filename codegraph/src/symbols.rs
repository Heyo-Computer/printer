use serde::{Deserialize, Serialize};
use tree_sitter::Node;

use crate::languages::SymbolKind;
use crate::parse::ParsedFile;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub qualified: String,
    pub kind: SymbolKind,
    pub start_line: u32,
    pub end_line: u32,
    pub start_byte: usize,
    pub end_byte: usize,
    pub signature: String,
    pub parent: Option<String>,
}

pub fn extract(parsed: &ParsedFile) -> Vec<Symbol> {
    let kinds = parsed.language.symbol_node_kinds();
    let mut out = Vec::new();
    let mut stack: Vec<Symbol> = Vec::new();
    walk(parsed.tree.root_node(), parsed, kinds, &mut stack, &mut out);
    out
}

fn walk(
    node: Node,
    parsed: &ParsedFile,
    kinds: &[(&'static str, SymbolKind)],
    stack: &mut Vec<Symbol>,
    out: &mut Vec<Symbol>,
) {
    let symbol_kind = kinds
        .iter()
        .find_map(|(k, sk)| (*k == node.kind()).then_some(*sk));

    let pushed = if let Some(kind) = symbol_kind {
        let symbol = build_symbol(node, kind, parsed, stack);
        out.push(symbol.clone());
        stack.push(symbol);
        true
    } else {
        false
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, parsed, kinds, stack, out);
    }

    if pushed {
        stack.pop();
    }
}

fn build_symbol(node: Node, kind: SymbolKind, parsed: &ParsedFile, stack: &[Symbol]) -> Symbol {
    let name = extract_name(node, kind, parsed);
    let parent_qualified = stack.last().map(|s| s.qualified.clone());
    let sep = parsed.language.qualified_separator();
    let qualified = match &parent_qualified {
        Some(p) => format!("{p}{sep}{name}"),
        None => name.clone(),
    };
    let signature = first_line(parsed.source.as_bytes(), node.start_byte(), node.end_byte());
    let start = node.start_position();
    let end = node.end_position();
    Symbol {
        name,
        qualified,
        kind,
        start_line: (start.row + 1) as u32,
        end_line: (end.row + 1) as u32,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        signature,
        parent: parent_qualified,
    }
}

fn extract_name(node: Node, kind: SymbolKind, parsed: &ParsedFile) -> String {
    let src = parsed.source.as_bytes();
    // Rust impl blocks: name is the type being impl'd, possibly with "Trait for Type".
    if node.kind() == "impl_item" {
        let ty = node
            .child_by_field_name("type")
            .map(|n| node_text(n, src))
            .unwrap_or_default();
        if let Some(tr) = node.child_by_field_name("trait") {
            let tr_text = node_text(tr, src);
            if !ty.is_empty() {
                return format!("{tr_text} for {ty}");
            }
            return tr_text;
        }
        if !ty.is_empty() {
            return ty;
        }
    }
    if let Some(name_node) = node.child_by_field_name("name") {
        return node_text(name_node, src);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let k = child.kind();
        if matches!(
            k,
            "identifier" | "type_identifier" | "property_identifier" | "field_identifier"
        ) {
            return node_text(child, src);
        }
    }
    let _ = kind;
    "<anon>".to_string()
}

fn node_text(node: Node, src: &[u8]) -> String {
    String::from_utf8_lossy(&src[node.start_byte()..node.end_byte()]).into_owned()
}

fn first_line(src: &[u8], start: usize, end: usize) -> String {
    let end = end.min(src.len());
    if start >= end {
        return String::new();
    }
    let slice = &src[start..end];
    let nl = slice.iter().position(|&b| b == b'\n').unwrap_or(slice.len());
    String::from_utf8_lossy(&slice[..nl]).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::Language;
    use crate::parse;

    #[test]
    fn rust_top_level_and_nested() {
        let src = r#"
fn top() {}

struct Bar { x: i32 }

impl Bar {
    fn baz(&self) {}
}
"#
        .to_string();
        let parsed = parse::parse_source(Language::Rust, src).unwrap();
        let symbols = extract(&parsed);
        let names: Vec<_> = symbols.iter().map(|s| s.qualified.as_str()).collect();
        assert!(names.contains(&"top"), "got {:?}", names);
        assert!(names.contains(&"Bar"), "got {:?}", names);
        assert!(names.contains(&"Bar"), "got {:?}", names);
        // The impl block becomes a symbol named "Bar"; the method nests under it.
        assert!(
            symbols.iter().any(|s| s.qualified == "Bar::baz"),
            "got {:?}",
            names
        );
    }

    #[test]
    fn python_class_and_method() {
        let src = r#"
class Foo:
    def bar(self):
        pass

def free():
    pass
"#
        .to_string();
        let parsed = parse::parse_source(Language::Python, src).unwrap();
        let symbols = extract(&parsed);
        let names: Vec<_> = symbols.iter().map(|s| s.qualified.as_str()).collect();
        assert!(names.contains(&"Foo"));
        assert!(names.contains(&"Foo.bar"));
        assert!(names.contains(&"free"));
    }
}
