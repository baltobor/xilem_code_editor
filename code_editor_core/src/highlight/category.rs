// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// Semantic category of a highlighted span. Pure data: the color for each
/// category is chosen by the theme's syntax palette in the widget layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HighlightCategory {
    Default,
    Keyword,
    String,
    Comment,
    Function,
    Type,
    Number,
    Operator,
    Punctuation,
    Macro,
    /// `// TODO:` / `// FIXME:` / `// XXX:` / `// HACK:` / `// BUG:` line
    /// comments. Tree-sitter routes them to `Comment`; a post-pass reclassifies
    /// any `Comment` whose first word after `//` is one of these markers
    /// (case-insensitive) so the theme can render them in a muted orange —
    /// less saturated than `Macro`, which is reserved for `format!` / `println!`.
    TodoComment,
    Attribute,
    Constructor,
    Property, // Struct fields, object properties
    Variable, // Local variables, parameters
    /// `self` and `Self` — the receiver / type-Self
    /// markers. tree-sitter-rust captures `self` as
    /// `variable.builtin`; we split that out so the theme
    /// can render the receiver in a distinct colour
    /// (brown by default) for at-a-glance "this is the
    /// receiver" reading.
    SelfKeyword,
    /// Markdown / similar headings. tree-sitter-md
    /// captures `# H1`-style headings as `text.title`. We
    /// used to fold that into `Type`, but `Type` in both
    /// light and dark themes is tuned for Rust-style type
    /// names (near-black in light, soft purple in dark) and
    /// reads as "barely distinguishable from default text"
    /// for big heading lines. Routing headings to a
    /// dedicated category lets the theme give them a strong,
    /// distinctive colour.
    Heading,
    // ANSI terminal colors for compiler output
    AnsiRed,
    AnsiGreen,
    AnsiYellow,
    AnsiBlue,
    AnsiMagenta,
    AnsiCyan,
    AnsiBold,
}
