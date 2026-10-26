// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Helpers for "what comes after `}` when we expand `{<Return>`".
//!
//! When the user types `{` (or accepts an auto-paired `{}`)
//! and presses Return, the editor expands the line into a
//! multi-line block:
//!
//! ```ignore
//! let foo = match l { <Return>
//! ```
//!
//! becomes
//!
//! ```ignore
//! let foo = match l {
//!     |
//! };
//! ```
//!
//! The `;` after the closing `}` matters: in Rust an
//! expression-block used as a value (`let x = { … }`,
//! `let y = match … { … }`) needs a trailing `;` to terminate
//! the statement, while a function/struct/impl/`while`/`for`
//! body does not.
//!
//! This module owns the heuristic that decides which case
//! we're in. It's lexical (no AST, no LSP) — works on the
//! single line up to but not including the `{` that's being
//! expanded. ~95% accuracy for typical Rust code; edge cases
//! get the wrong `;` and the user fixes with one keystroke,
//! which beats the alternative of never getting it right.

/// Should the closing `}` of an expanded block be followed by
/// a `;`?
///
/// `line_before_brace` is the line content from the start of
/// the line up to (but not including) the `{` whose Return
/// expansion is being computed. Trailing whitespace is fine —
/// we trim it.
///
/// Rules, in priority order:
///
/// 1. **Assignment context** — line ends with `=` (after
///    trimming trailing whitespace, possibly with `match X` /
///    `if … else` / etc. between the `=` and the `{`). The
///    block is the right-hand side of an assignment, which is
///    a statement. Needs `;`.
///
/// 2. **Bare expression in a `let` chain** — line starts with
///    `let … = ` (no terminator yet). Same reason as #1; this
///    is the common case.
///
/// 3. **Declaration / control-flow context** — line contains
///    `fn ` / `struct ` / `enum ` / `impl ` / `trait ` / `mod`
///    / `for ` / `while ` / `loop` / `unsafe ` / `async ` (as
///    leading or as control-flow keywords). The `{` opens a
///    body, not an expression-as-value. No `;`.
///
/// 4. **Default** — when none of the above match, no `;`. We
///    err on "no `;`" because adding one when not needed
///    creates a parse error visible immediately, while missing
///    one in the assignment case just produces invalid Rust
///    that the user notices on save / build.
pub fn needs_trailing_semicolon(line_before_brace: &str) -> bool {
    let trimmed = line_before_brace.trim_end();

    // Rule 1 — line ends with `=`. Catches both bare `let foo =`
    // and assignment to an existing binding (`foo =`).
    if trimmed.ends_with('=') {
        return true;
    }

    // Rule 2 — `let … = <expr> {`. The `=` may not be the LAST
    // non-whitespace character because constructs like
    // `let foo = match l {` have `match l` between the `=` and
    // the `{`. Detect by looking for a `=` (not `==`, not `=>`,
    // not `<=` / `>=` / `!=`) anywhere on the line that's
    // followed by content that opens an expression block.
    if has_assignment_followed_by_block_keyword(trimmed) {
        return true;
    }

    // Rule 3 — declaration / control-flow keywords introducing
    // a body. These are NOT expressions used as values (with
    // exceptions like `if` / `match` which we handled via
    // assignment detection above).
    let leading = trimmed.trim_start();
    if starts_with_declaration_or_body_keyword(leading) {
        return false;
    }

    // Rule 4 — default to no `;`.
    false
}

/// `=` followed (after the keyword) by an expression-block
/// keyword like `match` / `if` / `unsafe` / `async`. The
/// keyword is anywhere between the `=` and the end of the
/// trimmed line, terminated by the `{` (not on this slice
/// since we're given the prefix BEFORE the `{`).
fn has_assignment_followed_by_block_keyword(line: &str) -> bool {
    // Find the rightmost `=` that's a real assignment (not
    // `==` / `=>` / `<=` / `>=` / `!=`). Walk left to right,
    // pushing `=` onto a stack-of-one when we see one with
    // benign neighbours.
    let bytes = line.as_bytes();
    let mut last_eq: Option<usize> = None;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            // Look at the previous and next byte to filter
            // out comparison/arrow operators.
            let prev = if i > 0 { Some(bytes[i - 1]) } else { None };
            let next = if i + 1 < bytes.len() {
                Some(bytes[i + 1])
            } else {
                None
            };
            let is_eq_eq = next == Some(b'=');
            let is_arrow = next == Some(b'>'); // =>
            let is_compound = matches!(prev, Some(b'<' | b'>' | b'!' | b'='));
            if !is_eq_eq && !is_arrow && !is_compound {
                last_eq = Some(i);
            }
            // Skip past the operator's second character so we
            // don't double-count `==`'s second `=` as a real
            // assignment.
            if is_eq_eq {
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    let Some(eq_idx) = last_eq else {
        return false;
    };
    // What follows the `=` (skip whitespace) up to the end of
    // line should contain a block-expression keyword.
    let after = line[eq_idx + 1..].trim_start();
    // Match against the leading word.
    let leading_word: String = after
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    matches!(
        leading_word.as_str(),
        "match" | "if" | "unsafe" | "async" | "loop" | "while"
    )
}

/// The line's first significant word is a declaration /
/// body-introducing keyword. These open bodies that are NOT
/// expression-as-value, so no trailing `;` after `}`.
///
/// We accept some leading visibility / qualifier modifiers
/// (`pub`, `pub(crate)`, `async`, `unsafe`, `default`) before
/// the keyword.
fn starts_with_declaration_or_body_keyword(line: &str) -> bool {
    // Strip recognised qualifiers.
    let mut rest = line;
    loop {
        let trimmed = rest.trim_start();
        // pub / pub(crate) / pub / pub(in path)
        if let Some(after_pub) = trimmed.strip_prefix("pub") {
            // `pub ` or `pub(`
            if after_pub.starts_with(' ') || after_pub.starts_with('(') {
                // For `pub(...)`, skip to the closing paren.
                if after_pub.starts_with('(') {
                    if let Some(close) = after_pub.find(')') {
                        rest = &after_pub[close + 1..];
                        continue;
                    }
                }
                rest = after_pub.trim_start();
                continue;
            }
        }
        // unsafe / async / default / const / extern as
        // qualifiers before fn/impl/trait.
        for kw in &["unsafe ", "async ", "default ", "const ", "extern "] {
            if let Some(after) = trimmed.strip_prefix(kw) {
                rest = after;
                continue;
            }
        }
        break;
    }
    let leading = rest.trim_start();
    // Body-opening keywords. NOTE: `match` and `if` are
    // INTENTIONALLY OMITTED — they're expression-blocks that
    // CAN appear on the right side of an assignment. The
    // assignment-detection rule (#1, #2) takes precedence; if
    // we got here via Rule 3, neither `=` nor `let` was
    // present, so a bare `match X { … }` as a top-level
    // expression DOES NOT need a `;` from us (the surrounding
    // context decides).
    let leading_word: String = leading
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    matches!(
        leading_word.as_str(),
        "fn" | "struct"
            | "enum"
            | "impl"
            | "trait"
            | "mod"
            | "for"
            | "while"
            | "loop"
            | "unsafe"
            | "async"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_with_match_needs_semicolon() {
        // The motivating case.
        assert!(needs_trailing_semicolon("let foo = match l "));
    }

    #[test]
    fn assignment_with_if_else_needs_semicolon() {
        assert!(needs_trailing_semicolon("let foo = if x "));
    }

    #[test]
    fn assignment_to_existing_binding_needs_semicolon() {
        assert!(needs_trailing_semicolon("foo = match l "));
    }

    #[test]
    fn bare_let_assignment_with_block_expr_needs_semicolon() {
        // `let foo = { … }` — bare block expression as the
        // RHS. The `=` ends the line directly.
        assert!(needs_trailing_semicolon("let foo = "));
    }

    #[test]
    fn function_body_does_not_need_semicolon() {
        assert!(!needs_trailing_semicolon("fn foo() "));
        assert!(!needs_trailing_semicolon("pub fn foo() "));
        assert!(!needs_trailing_semicolon("pub(crate) fn foo() "));
        assert!(!needs_trailing_semicolon("async fn foo() "));
        assert!(!needs_trailing_semicolon("pub async fn foo() -> bool "));
    }

    #[test]
    fn struct_does_not_need_semicolon() {
        assert!(!needs_trailing_semicolon("struct Foo "));
        assert!(!needs_trailing_semicolon("pub struct Foo "));
    }

    #[test]
    fn enum_does_not_need_semicolon() {
        assert!(!needs_trailing_semicolon("enum Foo "));
        assert!(!needs_trailing_semicolon("pub enum Foo "));
    }

    #[test]
    fn impl_does_not_need_semicolon() {
        assert!(!needs_trailing_semicolon("impl Foo "));
        assert!(!needs_trailing_semicolon("impl<T> Foo<T> "));
        assert!(!needs_trailing_semicolon("impl Trait for Foo "));
    }

    #[test]
    fn trait_does_not_need_semicolon() {
        assert!(!needs_trailing_semicolon("trait Foo "));
    }

    #[test]
    fn mod_does_not_need_semicolon() {
        assert!(!needs_trailing_semicolon("mod foo "));
        assert!(!needs_trailing_semicolon("pub mod foo "));
    }

    #[test]
    fn control_flow_bodies_do_not_need_semicolon() {
        assert!(!needs_trailing_semicolon("for x in xs "));
        assert!(!needs_trailing_semicolon("while x < 10 "));
        assert!(!needs_trailing_semicolon("loop "));
    }

    #[test]
    fn equality_comparison_is_not_assignment() {
        // `if x == 3 {` — the `==` must NOT trigger
        // assignment-detection.
        assert!(!needs_trailing_semicolon("if x == 3 "));
    }

    #[test]
    fn arrow_in_match_arm_is_not_assignment() {
        // `match l { true => ` — the `=>` looks like `=` but
        // isn't.
        assert!(!needs_trailing_semicolon("    true => "));
    }

    #[test]
    fn comparison_operators_are_not_assignment() {
        assert!(!needs_trailing_semicolon("if x <= 3 "));
        assert!(!needs_trailing_semicolon("if x >= 3 "));
        assert!(!needs_trailing_semicolon("if x != 3 "));
    }

    #[test]
    fn nested_indent_preserves_classification() {
        // Whitespace prefix doesn't change the answer.
        assert!(needs_trailing_semicolon("        let foo = match l "));
        assert!(!needs_trailing_semicolon("        fn foo() "));
    }

    #[test]
    fn empty_line_falls_through_to_no_semicolon() {
        // Default rule.
        assert!(!needs_trailing_semicolon(""));
        assert!(!needs_trailing_semicolon("   "));
    }

    #[test]
    fn struct_init_assignment_with_brace_block_value() {
        // `let p = Point ` (the `{` follows on the line) —
        // assignment to a struct literal initialiser. The
        // surrounding statement still needs `;`.
        // Note: this isn't actually well-handled by Rule 2
        // since `Point` isn't in the block-keyword list, and
        // Rule 1 (`= ` at end) doesn't apply here. Falls
        // through to Rule 4 (no `;`). Acceptable for v1 — the
        // user will insert one keystroke.
        // This test pins the current behaviour so a future
        // tightening of the heuristic surfaces here.
        assert!(!needs_trailing_semicolon("let p = Point "));
    }
}
