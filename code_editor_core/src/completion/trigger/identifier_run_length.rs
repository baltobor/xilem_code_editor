// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! `identifier_run_length_ending_at` — count identifier-like
//! chars immediately before a byte position.

use super::is_identifier_char::is_identifier_char;
use crate::document::Document;

/// Count how many identifier chars sit immediately before
/// (and up through) the byte position `caret_byte` in the doc.
/// Used by `should_auto_trigger` to enforce the min-prefix gate.
pub fn identifier_run_length_ending_at(caret_byte: usize, doc: &dyn Document) -> usize {
    let text = doc.text();
    let total_bytes = doc.len_bytes();
    let clamped = caret_byte.min(total_bytes);
    if clamped == 0 {
        return 0;
    }
    let mut char_idx = text.byte_to_char(clamped);
    if char_idx == 0 {
        return 0;
    }
    let mut count = 0usize;
    while char_idx > 0 {
        let prev = text.char(char_idx - 1);
        if !is_identifier_char(prev) {
            break;
        }
        count += 1;
        char_idx -= 1;
    }
    count
}
