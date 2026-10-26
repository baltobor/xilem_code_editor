// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Word-boundary + indentation helpers shared by pointer-click word
//! selection (`pointer.rs`) and Tab/Shift-Tab indent handling
//! (`key.rs`).
//!
//! All functions are pure given a `Document` reference; they don't
//! touch widget state, so they're easy to unit-test in isolation.

use crate::cursor::Cursor;
use crate::document::Document;

// MARK: Word-boundary + indentation helpers

pub fn find_word_start(doc: &dyn Document, offset: usize) -> usize {
    let text = doc.text();
    let mut char_idx = text.byte_to_char(offset);
    let is_word = |c: char| c.is_alphanumeric() || c == '_';

    // If we're on a word char, scan backwards to word start
    if char_idx > 0 && char_idx <= text.len_chars() {
        let c = if char_idx < text.len_chars() {
            text.char(char_idx)
        } else {
            text.char(char_idx - 1)
        };

        if is_word(c) {
            while char_idx > 0 && is_word(text.char(char_idx - 1)) {
                char_idx -= 1;
            }
        }
    }

    text.char_to_byte(char_idx)
}

/// Find the end of the word containing the given byte offset.
pub fn find_word_end(doc: &dyn Document, offset: usize) -> usize {
    let text = doc.text();
    let total_chars = text.len_chars();
    let mut char_idx = text.byte_to_char(offset);
    let is_word = |c: char| c.is_alphanumeric() || c == '_';

    if char_idx < total_chars && is_word(text.char(char_idx)) {
        while char_idx < total_chars && is_word(text.char(char_idx)) {
            char_idx += 1;
        }
    }

    text.char_to_byte(char_idx)
}

/// If `offset` falls inside (or past) the trailing whitespace of a
/// non-empty line, return the byte range covering that whitespace
/// up to (but not including) the line terminator. Returns `None`
/// when the line has no trailing whitespace, or when `offset`
/// points before the trailing-ws region (meaning the caller should
/// fall back to word selection).
pub fn trailing_whitespace_range(
    doc: &dyn Document,
    offset: usize,
) -> Option<std::ops::Range<usize>> {
    let line = doc.line_of_offset(offset);
    let line_start = doc.offset_of_line(line);
    let content = doc.line_content(line)?;
    let line_text = content.to_string();
    if line_text.is_empty() {
        return None;
    }
    // Walk back from the line end to find the start of the trailing
    // whitespace run. We exclude `\n` / `\r` because `line_content`
    // strips terminators already.
    let mut ws_start_in_line = line_text.len();
    for (idx, ch) in line_text.char_indices().rev() {
        if ch == ' ' || ch == '\t' {
            ws_start_in_line = idx;
        } else {
            break;
        }
    }
    if ws_start_in_line == line_text.len() {
        // Line ends in a non-whitespace character.
        return None;
    }
    let ws_start = line_start + ws_start_in_line;
    let ws_end = line_start + line_text.len();
    if offset >= ws_start && offset <= ws_end {
        Some(ws_start..ws_end)
    } else {
        None
    }
}

/// Helper used when bracket-matching fell through and we want either
/// a trailing-whitespace selection (if the click landed there) or a
/// word selection as last resort.
pub fn select_word_or_trailing_ws(doc: &dyn Document, cursor: &mut Cursor, offset: usize) {
    if let Some(ws_range) = trailing_whitespace_range(doc, offset) {
        cursor.selection.set_single(ws_range.start, ws_range.end);
        cursor.word_select_anchor = Some(ws_range.start);
        return;
    }
    let word_start = find_word_start(doc, offset);
    let word_end = find_word_end(doc, offset);
    cursor.selection.set_single(word_start, word_end);
    cursor.word_select_anchor = Some(offset);
}

/// Indent (or outdent) every line covered by the primary selection.
///
/// `tab_size` is the number of spaces inserted/removed per line.
/// `outdent = true` strips up to `tab_size` leading spaces (or one
/// leading hard tab) from each line; otherwise `tab_size` spaces are
/// prepended. The selection is preserved (extended to whole lines)
/// so the user can keep pressing Tab/Shift+Tab to re-indent.
pub fn indent_selection(
    doc: &mut dyn Document,
    cursor: &mut Cursor,
    tab_size: usize,
    outdent: bool,
) {
    let primary = cursor.selection.primary();
    let sel_start = primary.min();
    let sel_end = primary.max();
    let start_line = doc.line_of_offset(sel_start);
    // If selection ends exactly at a line start, the user usually
    // doesn't intend that line to be indented. Step back one byte
    // to compute the inclusive end-line.
    let end_offset_for_line = if sel_end > sel_start {
        sel_end.saturating_sub(1)
    } else {
        sel_end
    };
    let end_line = doc.line_of_offset(end_offset_for_line);

    let pad: String = " ".repeat(tab_size);

    // Walk lines bottom-up so earlier edits don't shift later offsets.
    let mut total_delta: i64 = 0;
    let mut first_line_delta: i64 = 0;
    for line in (start_line..=end_line).rev() {
        let line_start = doc.offset_of_line(line);
        if outdent {
            // Strip up to `tab_size` leading spaces, OR a single tab.
            let line_text = match doc.line_content(line) {
                Some(c) => c.to_string(),
                None => continue,
            };
            let mut to_strip = 0usize;
            let mut chars = line_text.chars();
            if let Some(first) = chars.next() {
                if first == '\t' {
                    to_strip = 1;
                } else if first == ' ' {
                    to_strip = 1;
                    for c in chars {
                        if c == ' ' && to_strip < tab_size {
                            to_strip += 1;
                        } else {
                            break;
                        }
                    }
                }
            }
            if to_strip > 0 {
                doc.edit(line_start..line_start + to_strip, "");
                total_delta -= to_strip as i64;
                if line == start_line {
                    first_line_delta = -(to_strip as i64);
                }
            }
        } else {
            doc.edit(line_start..line_start, &pad);
            total_delta += pad.len() as i64;
            if line == start_line {
                first_line_delta = pad.len() as i64;
            }
        }
    }

    // Re-anchor the primary selection so it still covers the affected
    // lines (ignoring multi-cursor for this command — block indent
    // is a primary-selection operation).
    let new_start = (sel_start as i64 + first_line_delta).max(0) as usize;
    let new_end = (sel_end as i64 + total_delta).max(new_start as i64) as usize;
    cursor.selection.set_single(new_start, new_end);
    cursor.horiz = None;
}

// MARK: tests
//
// These functions are pure given a Document — they don't touch widget
// or paint state — so they're easy to unit-test in isolation. The
// regressions they protect against (issue #33: multibyte word
// boundaries; trailing-ws double-click selection; Tab indent
// semantics) all happen at this layer.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::RopeDocument;

    fn doc_with_words() -> RopeDocument {
        RopeDocument::from_str("foo bar baz")
    }

    fn doc_with_trailing_spaces() -> RopeDocument {
        RopeDocument::from_str("hello   \nworld")
    }

    #[test]
    fn find_word_start_walks_back_to_word_beginning() {
        let doc = doc_with_words();
        // Cursor inside "bar" at offset 5 → start at offset 4.
        assert_eq!(find_word_start(&doc, 5), 4);
        // Cursor at the very start of a word → unchanged.
        assert_eq!(find_word_start(&doc, 4), 4);
        // Cursor in whitespace → unchanged (function only walks if
        // we're on a word char).
        assert_eq!(find_word_start(&doc, 3), 3);
    }

    #[test]
    fn find_word_end_walks_forward_to_word_end() {
        let doc = doc_with_words();
        // Cursor inside "foo" at offset 1 → end at offset 3.
        assert_eq!(find_word_end(&doc, 1), 3);
        // Cursor at start of "bar" → end at 7.
        assert_eq!(find_word_end(&doc, 4), 7);
        // Cursor in whitespace → unchanged.
        assert_eq!(find_word_end(&doc, 3), 3);
    }

    #[test]
    fn find_word_start_handles_underscore_as_word_char() {
        // `_` is a word char (matches editor convention for
        // identifiers like `bar_baz`).
        let doc = RopeDocument::from_str("foo_bar baz");
        // Inside `foo_bar` at offset 5 → start at 0.
        assert_eq!(find_word_start(&doc, 5), 0);
    }

    #[test]
    fn trailing_whitespace_range_finds_run_when_offset_lands_in_it() {
        let doc = doc_with_trailing_spaces();
        // "hello   \nworld" — the trailing spaces are bytes 5..8.
        // Cursor at byte 6 (in the whitespace) should get the run.
        let range = trailing_whitespace_range(&doc, 6).expect("ws found");
        assert_eq!(range, 5..8);
    }

    #[test]
    fn trailing_whitespace_range_yields_none_when_line_ends_in_word_char() {
        let doc = RopeDocument::from_str("nowhitespace");
        assert!(trailing_whitespace_range(&doc, 5).is_none());
    }

    #[test]
    fn trailing_whitespace_range_yields_none_for_empty_line() {
        let doc = RopeDocument::from_str("\nfoo");
        // Line 0 is empty — function should bail.
        assert!(trailing_whitespace_range(&doc, 0).is_none());
    }
}
