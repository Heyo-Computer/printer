use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Tsx,
}

impl Language {
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "rs" => Some(Language::Rust),
            "py" | "pyi" => Some(Language::Python),
            "js" | "mjs" | "cjs" | "jsx" => Some(Language::JavaScript),
            "ts" | "mts" | "cts" => Some(Language::TypeScript),
            "tsx" => Some(Language::Tsx),
            _ => None,
        }
    }

    pub fn ts_language(self) -> tree_sitter::Language {
        match self {
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            Language::Python => tree_sitter_python::LANGUAGE.into(),
            Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        }
    }

    /// Tree-sitter node kinds we treat as symbol roots, paired with the
    /// `SymbolKind` they map to. Order doesn't matter.
    pub fn symbol_node_kinds(self) -> &'static [(&'static str, SymbolKind)] {
        match self {
            Language::Rust => &[
                ("function_item", SymbolKind::Function),
                ("struct_item", SymbolKind::Struct),
                ("enum_item", SymbolKind::Enum),
                ("trait_item", SymbolKind::Trait),
                ("impl_item", SymbolKind::Impl),
                ("mod_item", SymbolKind::Module),
                ("type_item", SymbolKind::TypeAlias),
                ("const_item", SymbolKind::Const),
                ("static_item", SymbolKind::Const),
                ("union_item", SymbolKind::Struct),
                ("macro_definition", SymbolKind::Function),
            ],
            Language::Python => &[
                ("function_definition", SymbolKind::Function),
                ("class_definition", SymbolKind::Class),
            ],
            Language::JavaScript | Language::TypeScript | Language::Tsx => &[
                ("function_declaration", SymbolKind::Function),
                ("generator_function_declaration", SymbolKind::Function),
                ("class_declaration", SymbolKind::Class),
                ("method_definition", SymbolKind::Method),
                ("interface_declaration", SymbolKind::Trait),
                ("type_alias_declaration", SymbolKind::TypeAlias),
                ("enum_declaration", SymbolKind::Enum),
            ],
        }
    }

    /// Path-style separator used when joining a symbol's name onto its parent
    /// to form a qualified name.
    pub fn qualified_separator(self) -> &'static str {
        match self {
            Language::Rust => "::",
            _ => ".",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
            Language::Tsx => "tsx",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Trait,
    Impl,
    TypeAlias,
    Const,
    Module,
}

impl SymbolKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "function" => Some(Self::Function),
            "method" => Some(Self::Method),
            "class" => Some(Self::Class),
            "struct" => Some(Self::Struct),
            "enum" => Some(Self::Enum),
            "trait" => Some(Self::Trait),
            "impl" => Some(Self::Impl),
            "type_alias" | "type" => Some(Self::TypeAlias),
            "const" => Some(Self::Const),
            "module" => Some(Self::Module),
            _ => None,
        }
    }
}
