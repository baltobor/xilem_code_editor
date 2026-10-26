// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Highlighting tests ported from rust_conductor; they need every grammar.
#![cfg(feature = "all-languages")]

use code_editor_core::highlight::{HighlightCategory, HighlightSpan, split_into_lines};
use code_editor_core::provider::HighlightPostPass;

use crate::{SyntaxHighlighter, TodoCommentPass, registry};

/// Highlight, apply the TODO pass, split into lines — the editor's pipeline.
fn highlight_lines(
    hl: &mut SyntaxHighlighter,
    src: &str,
    language: &str,
) -> Vec<Vec<HighlightSpan>> {
    let mut spans = hl.highlight(src, language);
    TodoCommentPass.apply(src, language, &mut spans);
    split_into_lines(src, &spans)
}

#[test]
fn enum_variant_declarations_are_constructor() {
    // Variant names in an enum *declaration* (`Build`, `Rebuild`) are
    // captured by tree-sitter-rust as `@constructor` — the category the
    // italic toggle slants — while struct-variant fields (`from`, `to`)
    // are `property` and stay upright.
    let mut hl = SyntaxHighlighter::new();
    let line = "    Rebuild { from: u32, to: u32 },";
    let src = format!("pub enum Operation {{\n{line}\n}}\n");
    let spans = highlight_lines(&mut hl, &src, "rust");
    let variant = spans[1]
        .iter()
        .find(|s| line.get(s.byte_range.clone()) == Some("Rebuild"));
    assert_eq!(
        variant.map(|s| s.category),
        Some(HighlightCategory::Constructor),
        "enum variant declaration should be Constructor, got {:?}",
        spans[1],
    );
    let field = spans[1]
        .iter()
        .find(|s| line.get(s.byte_range.clone()) == Some("from"));
    assert_eq!(
        field.map(|s| s.category),
        Some(HighlightCategory::Property),
        "struct-variant field should be Property, got {:?}",
        spans[1],
    );
}

#[test]
fn highlights_rust_keywords() {
    let mut hl = SyntaxHighlighter::new();
    let spans = highlight_lines(&mut hl, "let x = 42;", "rust");
    assert!(!spans.is_empty());
    let line0 = &spans[0];
    assert!(
        line0
            .iter()
            .any(|s| s.category == HighlightCategory::Keyword)
    );
}

#[test]
fn highlights_rust_string() {
    let mut hl = SyntaxHighlighter::new();
    let spans = highlight_lines(&mut hl, r#"let s = "hello";"#, "rust");
    assert!(
        spans[0]
            .iter()
            .any(|s| s.category == HighlightCategory::String)
    );
}

#[test]
fn highlights_self_as_self_keyword() {
    // tree-sitter-rust captures `self` as
    // `variable.builtin`. We split that out into the
    // `SelfKeyword` category so the theme can paint
    // it in the dedicated brown brush.
    let mut hl = SyntaxHighlighter::new();
    let spans = highlight_lines(&mut hl, "fn foo(&self) { let x = self.field; }", "rust");
    // At least one `SelfKeyword` span — both `&self`
    // in the param list and `self.field` should fire.
    let self_span_count = spans[0]
        .iter()
        .filter(|s| s.category == HighlightCategory::SelfKeyword)
        .count();
    assert!(
        self_span_count >= 2,
        "expected at least 2 SelfKeyword spans (param + field access), got {}: {:?}",
        self_span_count,
        spans[0],
    );
    // None of the SelfKeyword spans should be the
    // generic Variable category — the split must
    // route `variable.builtin` exclusively to
    // SelfKeyword.
    assert!(
        !spans[0]
            .iter()
            .any(|s| s.category == HighlightCategory::Variable && {
                // The span byte range must not
                // overlap any SelfKeyword span.
                // Belt-and-braces — if tree-sitter
                // ever emitted both for the same
                // bytes, this catches it.
                spans[0].iter().any(|other| {
                    other.category == HighlightCategory::SelfKeyword
                        && other.byte_range == s.byte_range
                })
            }),
        "no span should carry both Variable and SelfKeyword for the same range",
    );
}

#[test]
fn plain_text_returns_empty_spans() {
    let mut hl = SyntaxHighlighter::new();
    let spans = highlight_lines(&mut hl, "hello world", "txt");
    assert!(spans[0].is_empty());
}

#[test]
fn multiline_spans_split_correctly() {
    let mut hl = SyntaxHighlighter::new();
    let src = "/* line one\nline two */\nlet x = 1;";
    let spans = highlight_lines(&mut hl, src, "rust");
    assert!(spans.len() >= 3);
    // Lines 0 and 1 should have comment spans
    assert!(
        spans[0]
            .iter()
            .any(|s| s.category == HighlightCategory::Comment)
    );
    assert!(
        spans[1]
            .iter()
            .any(|s| s.category == HighlightCategory::Comment)
    );
}

#[test]
fn todo_comments_get_dedicated_category() {
    // `// TODO` lines route to TodoComment so the theme can
    // paint them in a less-saturated orange than `format!`.
    // Tree-sitter classifies the entire `//` line as Comment;
    // our post-pass reclassifies just the TODO-prefixed ones.
    let mut hl = SyntaxHighlighter::new();
    let src = "// TODO: write more tests\n// regular comment\nfn x() {}\n";
    let spans = highlight_lines(&mut hl, src, "rust");

    let line0_has_todo = spans[0]
        .iter()
        .any(|s| s.category == HighlightCategory::TodoComment);
    assert!(
        line0_has_todo,
        "expected TodoComment on line 0, got {:?}",
        spans[0]
    );

    let line1_has_comment = spans[1]
        .iter()
        .any(|s| s.category == HighlightCategory::Comment);
    let line1_has_todo = spans[1]
        .iter()
        .any(|s| s.category == HighlightCategory::TodoComment);
    assert!(
        line1_has_comment && !line1_has_todo,
        "ordinary comment should stay Comment, got {:?}",
        spans[1]
    );
}

#[test]
fn todo_matcher_is_case_insensitive() {
    let mut hl = SyntaxHighlighter::new();
    let src = "// todo: lowercase\n// Todo: mixed\n// TODO! bang\n";
    let spans = highlight_lines(&mut hl, src, "rust");
    for (i, line) in spans.iter().take(3).enumerate() {
        assert!(
            line.iter()
                .any(|s| s.category == HighlightCategory::TodoComment),
            "line {i} should be TodoComment, got {:?}",
            line,
        );
    }
}

#[test]
fn todoist_is_not_a_todo() {
    // The matcher requires a word boundary after "todo".
    // `// todoist` is a plain comment.
    let mut hl = SyntaxHighlighter::new();
    let src = "// todoist scheduler\n";
    let spans = highlight_lines(&mut hl, src, "rust");
    assert!(
        !spans[0]
            .iter()
            .any(|s| s.category == HighlightCategory::TodoComment),
        "`// todoist` should not match TodoComment, got {:?}",
        spans[0],
    );
}

#[test]
fn fixme_comments_share_the_todo_category() {
    // `// FIXME` variants route to the same TodoComment category as
    // `// TODO`, while `// fixmeup` (no word boundary) stays Comment.
    let mut hl = SyntaxHighlighter::new();
    let src = "// FIXME: bug\n// fixme - later\n// Fixme\n// fixmeup helper\n";
    let spans = highlight_lines(&mut hl, src, "rust");
    for (i, line) in spans.iter().take(3).enumerate() {
        assert!(
            line.iter()
                .any(|s| s.category == HighlightCategory::TodoComment),
            "line {i} should be TodoComment, got {:?}",
            line,
        );
    }
    assert!(
        !spans[3]
            .iter()
            .any(|s| s.category == HighlightCategory::TodoComment),
        "`// fixmeup` should stay Comment, got {:?}",
        spans[3],
    );
}

#[test]
fn todo_matcher_handles_non_ascii_leading_char() {
    // Regression: `// → "Remove unused import"` panicked because the
    // matcher sliced `rest[..4]`, landing inside the 3-byte `→`.
    let mut hl = SyntaxHighlighter::new();
    let src = "// → \"Remove unused import\"\n";
    let spans = highlight_lines(&mut hl, src, "rust");
    assert!(
        !spans[0]
            .iter()
            .any(|s| s.category == HighlightCategory::TodoComment),
        "non-ASCII-prefixed comment should not match TodoComment, got {:?}",
        spans[0],
    );
}

#[test]
fn highlights_rust_macros() {
    let mut hl = SyntaxHighlighter::new();
    let spans = highlight_lines(&mut hl, r#"println!("hello");"#, "rust");
    assert!(
        spans[0]
            .iter()
            .any(|s| s.category == HighlightCategory::Macro)
    );
}

#[test]
fn highlights_c_code() {
    let mut hl = SyntaxHighlighter::new();
    let spans = highlight_lines(&mut hl, "int main() { return 0; }", "c");
    assert!(!spans.is_empty());
    // Should have keyword (int) and function (main)
    assert!(
        spans[0]
            .iter()
            .any(|s| s.category == HighlightCategory::Type
                || s.category == HighlightCategory::Keyword)
    );
}

#[test]
fn highlights_json() {
    let mut hl = SyntaxHighlighter::new();
    let spans = highlight_lines(&mut hl, r#"{"key": "value"}"#, "json");
    assert!(!spans.is_empty());
    assert!(
        spans[0]
            .iter()
            .any(|s| s.category == HighlightCategory::String)
    );
}

#[test]
fn highlights_just_recipe() {
    // A minimal justfile: a recipe with a string body.
    // The recipe header's `name` identifier is captured
    // as `@function` by the helix-flavored query; the
    // string body is captured as `@string`. We only
    // assert that the highlighter produces non-empty
    // spans and surfaces at least one Function or String
    // span — the exact mapping depends on `HIGHLIGHT_NAMES`
    // prefix-matching and isn't worth pinning here.
    let mut hl = SyntaxHighlighter::new();
    let src = "hello:\n    echo \"world\"\n";
    let spans = highlight_lines(&mut hl, src, "just");
    assert!(
        !spans.is_empty(),
        "highlighter returned no lines for justfile"
    );
    let any_categorized = spans
        .iter()
        .flatten()
        .any(|s| s.category != HighlightCategory::Default);
    assert!(
        any_categorized,
        "no syntax categories found in justfile spans"
    );
}

#[test]
fn highlights_proto_keywords() {
    // tree-sitter-proto captures `message` / `string` /
    // field type names via @keyword and @type. We just want
    // confirmation that *some* spans come back — previously
    // proto rendered as plain text because the language
    // had `highlight_config` returning None.
    let mut hl = SyntaxHighlighter::new();
    let src = "syntax = \"proto3\";\nmessage Foo { string name = 1; }\n";
    let spans = highlight_lines(&mut hl, src, "proto");
    let any_keyword = spans
        .iter()
        .flatten()
        .any(|s| s.category == HighlightCategory::Keyword);
    assert!(
        any_keyword,
        "expected a Keyword span on the proto sample, got {:?}",
        spans
    );
    let any_type = spans
        .iter()
        .flatten()
        .any(|s| s.category == HighlightCategory::Type);
    assert!(
        any_type,
        "expected a Type span on the proto sample, got {:?}",
        spans
    );
}

#[test]
fn highlights_markdown_headings_and_inline_code() {
    // tree-sitter-md emits dotted captures (`text.title`,
    // `text.literal`, …) that have to be present in
    // `HIGHLIGHT_NAMES` for the longest-prefix match to
    // succeed. Without them, .md files used to render
    // as plain text; this test pins the regression.
    let mut hl = SyntaxHighlighter::new();
    let src = "# Heading\n\nSome `inline` code.\n";
    let spans = highlight_lines(&mut hl, src, "markdown");
    let any_categorized = spans
        .iter()
        .flatten()
        .any(|s| s.category != HighlightCategory::Default);
    assert!(
        any_categorized,
        "no syntax categories found in markdown spans: {:?}",
        spans
    );
}

#[test]
fn markdown_h2_in_multi_line_document() {
    // Reproduce the user-reported screenshot shape: h1, a
    // paragraph, then h2. Headings now route to the
    // dedicated `Heading` category (was Type) so the
    // theme can give them a saturated colour distinct
    // from Rust type names.
    let mut hl = SyntaxHighlighter::new();
    let src = "# Rust Conductor\n\nA paragraph with **bold** text.\n\n## Why Rust Conductor?\n\nMore prose.\n";
    let spans = highlight_lines(&mut hl, src, "markdown");
    let h2_line = &spans[4];
    let heading_present = h2_line
        .iter()
        .any(|s| s.category == HighlightCategory::Heading);
    assert!(
        heading_present,
        "expected Heading span on h2 in multi-line doc, got {:?}",
        h2_line,
    );
}

#[test]
fn markdown_fenced_code_block_injects_rust() {
    // ```rust fences must inject the rust grammar so the
    // code body picks up keyword / string / function colours
    // instead of being painted as default text. The tree-
    // sitter-md INJECTION_QUERY_BLOCK matches the fence's
    // info_string and dispatches by language name; we
    // declare "rust" as an injection target so the lookup
    // resolves.
    let mut hl = SyntaxHighlighter::new();
    let src = "Some prose.\n\n```rust\nfn hello() { let x = 42; }\n```\n";
    let spans = highlight_lines(&mut hl, src, "markdown");
    // Line 3 is the rust body. It must contain at least one
    // Keyword (`fn`, `let`) span.
    let body = &spans[3];
    let has_keyword = body
        .iter()
        .any(|s| s.category == HighlightCategory::Keyword);
    assert!(
        has_keyword,
        "expected the ```rust fence body to receive a Keyword span (fn / let), got {:?}",
        body,
    );
}

#[test]
fn markdown_h2_real_heading_text_categorized() {
    // Tighter assertion than the multi-level test: a real
    // h2 line must produce a span whose category is the
    // dedicated `Heading` (not Type — Type stays for Rust
    // type names). The user reported h2 rendering as
    // default text in the editor; routing to Heading +
    // pinning a saturated `syntax_heading` colour fixed it.
    let mut hl = SyntaxHighlighter::new();
    let src = "## Why Rust Conductor?\n\nSome text\n";
    let spans = highlight_lines(&mut hl, src, "markdown");
    let heading_span = spans[0]
        .iter()
        .find(|s| s.category == HighlightCategory::Heading);
    assert!(
        heading_span.is_some(),
        "expected a Heading span on the h2 line, got {:?}",
        spans[0],
    );
}

#[test]
fn markdown_atx_h1_through_h6_are_highlighted() {
    // Pin every heading level — # through ###### — so a
    // future grammar-query change can't silently demote one
    // level to "default text". The block grammar exposes
    // `atx_heading.inline` on all six markers; if the
    // capture doesn't fire for a level, the (inline)
    // injection won't either and the heading text shows up
    // as default.
    let mut hl = SyntaxHighlighter::new();
    for level in 1..=6 {
        let hashes = "#".repeat(level);
        let src = format!("{hashes} Heading at level {level}\n");
        let spans = highlight_lines(&mut hl, &src, "markdown");
        let any_categorized = spans
            .iter()
            .flatten()
            .any(|s| s.category != HighlightCategory::Default);
        assert!(
            any_categorized,
            "heading level {} produced no spans: {:?}",
            level, spans,
        );
    }
}

#[test]
fn markdown_inline_emphasis_and_strong_are_highlighted() {
    // Block-grammar-only registration would leave a normal
    // prose paragraph with no inline markup highlighted.
    // The `(inline) → markdown_inline` injection from the
    // block grammar's injections.scm has to resolve, which
    // requires the inline grammar to be registered AND
    // listed as an injection target on the markdown
    // language. This test pins both.
    let mut hl = SyntaxHighlighter::new();
    let src = "A paragraph with **bold** text, *italic*, and a [link](https://example.com).\n";
    let spans = highlight_lines(&mut hl, src, "markdown");
    // The inline grammar maps `text.strong` → Keyword,
    // `text.emphasis` → Variable, `text.uri` / `text.literal`
    // → String, `text.reference` → Function. We just check
    // that a meaningful inline-flavoured category lands on
    // the line — the exact mapping is asserted by the
    // capture_to_category routing in this file.
    let inline_cats: Vec<_> = spans[0]
        .iter()
        .map(|s| s.category)
        .filter(|c| {
            matches!(
                c,
                HighlightCategory::Keyword
                    | HighlightCategory::Variable
                    | HighlightCategory::String
                    | HighlightCategory::Function
            )
        })
        .collect();
    assert!(
        !inline_cats.is_empty(),
        "expected at least one inline-grammar span (bold / italic / link / uri) on the paragraph, got {:?}",
        spans[0],
    );
}

#[test]
fn highlights_zig_code() {
    // A minimal Zig program: stdlib import + a `pub fn` with
    // a `print` call. The grammar should categorise the
    // keywords (`const`, `pub`, `fn`) and the string literal.
    let mut hl = SyntaxHighlighter::new();
    let src =
        "const std = @import(\"std\");\npub fn main() void { std.debug.print(\"hi\\n\", .{}); }\n";
    let spans = highlight_lines(&mut hl, src, "zig");
    assert!(!spans.is_empty(), "highlighter returned no lines for zig");
    let any_categorized = spans
        .iter()
        .flatten()
        .any(|s| s.category != HighlightCategory::Default);
    assert!(any_categorized, "no syntax categories found in zig spans");
    let any_keyword = spans
        .iter()
        .flatten()
        .any(|s| s.category == HighlightCategory::Keyword);
    assert!(
        any_keyword,
        "expected at least one keyword span in zig sample"
    );
}

#[test]
fn registry_lookup_by_extension() {
    let reg = registry();
    assert_eq!(reg.from_extension("rs").language_id(), "rust");
    assert_eq!(reg.from_extension("c").language_id(), "c");
    assert_eq!(reg.from_extension("cpp").language_id(), "cpp");
    assert_eq!(reg.from_extension("toml").language_id(), "toml");
    assert_eq!(reg.from_extension("md").language_id(), "markdown");
    assert_eq!(reg.from_extension("json").language_id(), "json");
    assert_eq!(reg.from_extension("yaml").language_id(), "yaml");
    assert_eq!(reg.from_extension("unknown").language_id(), "plaintext");
}

#[test]
fn registry_lookup_by_id() {
    let reg = registry();
    assert!(reg.from_id("rust").is_some());
    assert!(reg.from_id("c").is_some());
    assert!(reg.from_id("unknown").is_none());
}

#[test]
fn registry_routes_just_filename_to_just_language() {
    // The same filename-as-extension trick has to work
    // through the trait-driven `LanguageRegistry` lookup —
    // that's what the highlight pipeline actually calls.
    let reg = registry();
    assert_eq!(reg.from_extension("just").language_id(), "just");
    assert_eq!(reg.from_extension("Justfile").language_id(), "just");
    assert_eq!(reg.from_extension("justfile").language_id(), "just");
}

#[test]
fn registry_routes_zig_to_zig_language() {
    let reg = registry();
    assert_eq!(reg.from_extension("zig").language_id(), "zig");
    assert_eq!(reg.from_extension("zon").language_id(), "zig");
    assert!(reg.from_id("zig").is_some());
}

#[test]
fn every_extension_resolves_to_its_language() {
    let reg = registry();
    for (ext, id) in [
        ("rs", "rust"),
        ("RS", "rust"),
        ("h", "c"),
        ("hpp", "cpp"),
        ("lock", "toml"),
        ("markdown", "markdown"),
        ("yml", "yaml"),
        ("Justfile", "just"),
        ("zon", "zig"),
        ("pem", "certificate"),
        ("proto", "proto"),
        ("json", "json"),
        ("nope", "plaintext"),
    ] {
        assert_eq!(reg.from_extension(ext).language_id(), id, "extension {ext}");
    }
}
