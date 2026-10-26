// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use code_editor_core::highlight::HighlightCategory;

/// The ordered list of capture names for syntax highlighting.
/// tree-sitter-highlight maps highlight events to indices into this array.
pub const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "comment",
    "comment.documentation",
    "comment.line",
    "comment.block",
    "constant",
    "constant.builtin",
    "constructor",
    "function",
    "function.builtin",
    "function.macro",
    "function.method",
    "keyword",
    "label",
    "number",
    "operator",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "string",
    "string.special",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
    // Markdown's grammar (tree-sitter-md) emits these dotted
    // names. Without them in the list, `text.title` /
    // `text.literal` / etc. fail the longest-prefix match and
    // silently drop to "no highlight" — which is why .md files
    // used to render as plain text. The actual brush mapping
    // happens in `editor::highlight::capture_to_category`.
    "text.title",
    "text.literal",
    "text.uri",
    "text.reference",
    "text.emphasis",
    "text.strong",
    "punctuation.special",
];

/// Map a capture-name index (into `HIGHLIGHT_NAMES`) to a `HighlightCategory`.
pub fn capture_to_category(idx: usize) -> HighlightCategory {
    match HIGHLIGHT_NAMES.get(idx).copied().unwrap_or("") {
        "keyword" => HighlightCategory::Keyword,
        "string" | "string.special" => HighlightCategory::String,
        s if s.starts_with("comment") => HighlightCategory::Comment,
        "function.macro" => HighlightCategory::Macro,
        "function" | "function.builtin" | "function.method" => HighlightCategory::Function,
        "type" | "type.builtin" => HighlightCategory::Type,
        "constructor" => HighlightCategory::Constructor,
        "number" | "constant" | "constant.builtin" => HighlightCategory::Number,
        "operator" => HighlightCategory::Operator,
        "punctuation" | "punctuation.bracket" | "punctuation.delimiter" | "punctuation.special" => {
            HighlightCategory::Punctuation
        }
        "attribute" | "tag" => HighlightCategory::Attribute,
        "property" => HighlightCategory::Property,
        // Markdown captures from tree-sitter-md. Headings get
        // their own category (Heading) so the theme can give
        // them a punchy colour distinct from the soft Type
        // shade tuned for Rust types. Inline code & URLs →
        // String; link references → Function; bold → Keyword;
        // italic → Variable.
        "text.title" => HighlightCategory::Heading,
        "text.literal" | "text.uri" => HighlightCategory::String,
        "text.reference" => HighlightCategory::Function,
        "text.strong" => HighlightCategory::Keyword,
        "text.emphasis" => HighlightCategory::Variable,
        // `variable.builtin` covers `self` (and `Self`) in
        // tree-sitter-rust's highlights query. Route it to
        // its own category so the theme can paint the
        // receiver in its dedicated colour. Plain
        // `variable` / `variable.parameter` / `label` stay
        // as the generic local-binding shape.
        "variable.builtin" => HighlightCategory::SelfKeyword,
        "variable" | "variable.parameter" | "label" => HighlightCategory::Variable,
        _ => HighlightCategory::Default,
    }
}
