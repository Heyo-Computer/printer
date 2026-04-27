use serde::Serialize;

use crate::symbols::Symbol;

#[derive(Serialize, Debug)]
pub struct OutlineNode {
    pub name: String,
    pub qualified: String,
    pub kind: crate::languages::SymbolKind,
    pub start_line: u32,
    pub end_line: u32,
    pub signature: String,
    pub children: Vec<OutlineNode>,
}

/// Build a hierarchical outline from a flat symbol list.
///
/// Symbols whose `parent` doesn't resolve to another symbol's `qualified`
/// name surface at the top level — that handles e.g. orphaned methods if
/// extraction missed a container.
pub fn build(symbols: &[Symbol]) -> Vec<OutlineNode> {
    let mut roots: Vec<OutlineNode> = Vec::new();
    for sym in symbols {
        let node = OutlineNode {
            name: sym.name.clone(),
            qualified: sym.qualified.clone(),
            kind: sym.kind,
            start_line: sym.start_line,
            end_line: sym.end_line,
            signature: sym.signature.clone(),
            children: Vec::new(),
        };
        match &sym.parent {
            Some(parent_q) => match find_mut(&mut roots, parent_q) {
                Some(p) => p.children.push(node),
                None => roots.push(node),
            },
            None => roots.push(node),
        }
    }
    roots
}

fn find_mut<'a>(nodes: &'a mut [OutlineNode], qualified: &str) -> Option<&'a mut OutlineNode> {
    for n in nodes {
        if n.qualified == qualified {
            return Some(n);
        }
        if let Some(found) = find_mut(&mut n.children, qualified) {
            return Some(found);
        }
    }
    None
}

pub fn render_text(nodes: &[OutlineNode], depth: usize, out: &mut String) {
    for n in nodes {
        let indent = "  ".repeat(depth);
        out.push_str(&format!(
            "{indent}{:?} {} ({}-{}) — {}\n",
            n.kind, n.qualified, n.start_line, n.end_line, n.signature
        ));
        render_text(&n.children, depth + 1, out);
    }
}
