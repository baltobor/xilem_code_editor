// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Bracket-noise deny-list for the LSP trigger-char path.
//!
//! Single responsibility: name the chars rust-analyzer ships
//! as triggers but the editor wants to ignore. Lives apart
//! from `is_trigger_char` so the policy is a one-line list,
//! not buried in the trigger-check body.

/// rust-analyzer declares `(`, `)`, `[`, `]`, `<`, `>`, `'`,
/// and `"` as completion trigger characters. We deliberately
/// suppress them — typing `is_finite(` or closing `)`
/// re-triggered top-level scope completion (`self::`,
/// `crate::`) that's never what the user wanted. Curly braces
/// `{` `}` join the list for the same reason: typing
/// `match l {` should land the caret in an empty block, not
/// pre-fill it with whatever (`loop`, `let`, …) the popup
/// happened to be showing as the user crossed the bracket.
/// The genuinely useful triggers (`.` for method-on-receiver
/// and the second `:` of `::`) aren't in this list and still
/// fire.
pub fn is_noisy_trigger_char(ch: char) -> bool {
    matches!(
        ch,
        '(' | ')' | '[' | ']' | '<' | '>' | '\'' | '"' | '{' | '}'
    )
}
