// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Word-boundary helpers — `prev_word_boundary` and
//! `next_word_boundary`. Pure functions over a `Document`, used by
//! both movement (Ctrl+Left / Ctrl+Right) and editing
//! (delete_word / backdelete_word).

use crate::document::Document;

pub fn prev_word_boundary(doc: &dyn Document, offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }
    let text = doc.text();
    let mut char_idx = text.byte_to_char(offset);

    // Skip whitespace backwards
    while char_idx > 0 {
        let c = text.char(char_idx - 1);
        if !c.is_whitespace() {
            break;
        }
        char_idx -= 1;
    }

    // Skip word characters backwards
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    if char_idx > 0 {
        let first_char = text.char(char_idx - 1);
        if is_word(first_char) {
            // Skip word chars
            while char_idx > 0 && is_word(text.char(char_idx - 1)) {
                char_idx -= 1;
            }
        } else {
            // Skip punctuation chars
            while char_idx > 0 {
                let c = text.char(char_idx - 1);
                if is_word(c) || c.is_whitespace() {
                    break;
                }
                char_idx -= 1;
            }
        }
    }

    text.char_to_byte(char_idx)
}
pub fn next_word_boundary(doc: &dyn Document, offset: usize) -> usize {
    let text = doc.text();
    let total_chars = text.len_chars();
    let mut char_idx = text.byte_to_char(offset);

    if char_idx >= total_chars {
        return doc.len_bytes();
    }

    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let first_char = text.char(char_idx);

    if is_word(first_char) {
        // Skip word chars forward
        while char_idx < total_chars && is_word(text.char(char_idx)) {
            char_idx += 1;
        }
    } else if !first_char.is_whitespace() {
        // Skip punctuation forward
        while char_idx < total_chars {
            let c = text.char(char_idx);
            if is_word(c) || c.is_whitespace() {
                break;
            }
            char_idx += 1;
        }
    }

    // Skip whitespace forward
    while char_idx < total_chars && text.char(char_idx).is_whitespace() {
        char_idx += 1;
    }

    text.char_to_byte(char_idx)
}
