// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Bracket-pairing helpers — `find_matching_bracket`, `is_bracket`,
//! `find_nearby_bracket`. Used by:
//!   - `editor::input::pointer` for double-click bracket-jump,
//!   - `editor::input::key` for bracket auto-close ghost commit.

use crate::document::Document;

// MARK: Free functions — bracket pairing + word boundaries

/// Check if a character is an opening bracket.
fn is_opening_bracket(c: char) -> bool {
    matches!(c, '(' | '{' | '[' | '<')
}

/// Check if a character is a closing bracket.
fn is_closing_bracket(c: char) -> bool {
    matches!(c, ')' | '}' | ']' | '>')
}

/// Get the matching bracket for a given bracket character.
fn matching_bracket(c: char) -> Option<char> {
    match c {
        '(' => Some(')'),
        ')' => Some('('),
        '{' => Some('}'),
        '}' => Some('{'),
        '[' => Some(']'),
        ']' => Some('['),
        '<' => Some('>'),
        '>' => Some('<'),
        _ => None,
    }
}

/// Find the matching bracket offset for the bracket at the given byte offset.
/// Returns None if the character is not a bracket or no match is found.
pub fn find_matching_bracket(doc: &dyn Document, offset: usize) -> Option<usize> {
    let text = doc.text();
    let total_bytes = doc.len_bytes();

    if offset >= total_bytes {
        return None;
    }

    let char_idx = text.byte_to_char(offset);
    if char_idx >= text.len_chars() {
        return None;
    }

    let bracket = text.char(char_idx);
    let match_char = matching_bracket(bracket)?;

    if is_opening_bracket(bracket) {
        // Scan forward
        let mut depth = 1i32;
        let mut idx = char_idx + 1;
        while idx < text.len_chars() {
            let c = text.char(idx);
            if c == bracket {
                depth += 1;
            } else if c == match_char {
                depth -= 1;
                if depth == 0 {
                    return Some(text.char_to_byte(idx));
                }
            }
            idx += 1;
        }
    } else {
        // Scan backward
        let mut depth = 1i32;
        let mut idx = char_idx;
        while idx > 0 {
            idx -= 1;
            let c = text.char(idx);
            if c == bracket {
                depth += 1;
            } else if c == match_char {
                depth -= 1;
                if depth == 0 {
                    return Some(text.char_to_byte(idx));
                }
            }
        }
    }

    None
}

/// Check if a character is any kind of bracket.
pub fn is_bracket(c: char) -> bool {
    is_opening_bracket(c) || is_closing_bracket(c)
}

/// Find a bracket at or near the given offset (forgiving search).
/// Looks at the current position, then left, then right.
/// Returns (bracket_offset, bracket_char) if found.
pub fn find_nearby_bracket(doc: &dyn Document, offset: usize) -> Option<(usize, char)> {
    let text = doc.text();
    let total_chars = text.len_chars();

    if total_chars == 0 {
        return None;
    }

    let char_idx = text.byte_to_char(offset.min(doc.len_bytes()));
    let char_idx = char_idx.min(total_chars.saturating_sub(1));

    // Check current position
    if char_idx < total_chars {
        let c = text.char(char_idx);
        if is_bracket(c) {
            return Some((text.char_to_byte(char_idx), c));
        }
    }

    // Check one position to the left
    if char_idx > 0 {
        let c = text.char(char_idx - 1);
        if is_bracket(c) {
            return Some((text.char_to_byte(char_idx - 1), c));
        }
    }

    // Check one position to the right
    if char_idx + 1 < total_chars {
        let c = text.char(char_idx + 1);
        if is_bracket(c) {
            return Some((text.char_to_byte(char_idx + 1), c));
        }
    }

    None
}
