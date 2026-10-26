// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Cursor — wraps Selection with high-level editing operations.
//!
//! The cursor translates high-level commands (move_left, select_word, insert)
//! into selection region mutations + document edits. It is the bridge between
//! keyboard/mouse input and the document/selection data structures.
//!
use crate::layout::ColPosition;
use crate::selection::Selection;

pub struct Cursor {
    pub selection: Selection,
    /// Preserved horizontal pixel position for vertical movement.
    /// When moving up/down, the cursor tries to stay at this x position
    /// even when passing through shorter lines.
    pub horiz: Option<ColPosition>,
    /// Anchor position for Alt+drag column selection: (line, x_pixel).
    /// Set when column selection starts, cleared when drag ends.
    pub column_anchor: Option<(usize, f64)>,
    /// Click count for tracking double/triple click selection mode.
    /// 2 = word selection, 3 = line selection. Reset on pointer up.
    pub click_count: u8,
    /// Anchor offset for word/line selection drag extension.
    pub word_select_anchor: Option<usize>,
}

impl Cursor {
    /// Create a cursor with a single caret at the given byte offset.
    pub fn new(offset: usize) -> Self {
        Self {
            selection: Selection::caret(offset),
            horiz: None,
            column_anchor: None,
            click_count: 0,
            word_select_anchor: None,
        }
    }

    /// Create a cursor from an existing selection.
    pub fn from_selection(selection: Selection) -> Self {
        Self {
            selection,
            horiz: None,
            column_anchor: None,
            click_count: 0,
            word_select_anchor: None,
        }
    }

    /// Returns the byte offset of the primary cursor (focus point).
    pub fn offset(&self) -> usize {
        self.selection.primary().end
    }

    /// Returns true if the selection is just a caret (no text selected).
    pub fn is_caret(&self) -> bool {
        self.selection.is_caret()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::{find_matching_bracket, find_nearby_bracket, is_bracket};
    use crate::document::RopeDocument;

    #[test]
    fn test_select_to_line_end() {
        let doc = RopeDocument::from_str("hello world\nsecond line");
        let mut cursor = Cursor::new(6); // After "hello "
        cursor.select_to_line_end(&doc);

        let region = cursor.selection.primary();
        assert_eq!(region.start, 6);
        assert_eq!(region.end, 11); // End of "hello world"
    }

    #[test]
    fn test_select_to_line_start() {
        let doc = RopeDocument::from_str("hello world\nsecond line");
        let mut cursor = Cursor::new(6); // After "hello "
        cursor.select_to_line_start(&doc);

        let region = cursor.selection.primary();
        assert_eq!(region.start, 6);
        assert_eq!(region.end, 0); // Start of line
    }

    #[test]
    fn test_move_to_line_end() {
        let doc = RopeDocument::from_str("hello world\nsecond line");
        let mut cursor = Cursor::new(6);
        cursor.move_to_line_end(&doc);

        assert_eq!(cursor.offset(), 11);
    }

    #[test]
    fn test_move_to_line_start() {
        let doc = RopeDocument::from_str("hello world\nsecond line");
        let mut cursor = Cursor::new(6);
        cursor.move_to_line_start(&doc);

        assert_eq!(cursor.offset(), 0);
    }

    // MARK: tests — UTF-8 horizontal movement
    //
    // Issue #33 was a parley char-boundary panic on paste of multibyte text.
    // These tests pin `move_left` / `move_right` to *characters*, not bytes,
    // so any future refactor that accidentally indexes by byte trips a test.
    //
    // Each test uses a tiny named helper for the document so the test body
    // reads as a single scenario.

    fn doc_with_two_byte_char() -> RopeDocument {
        // 'á' is two UTF-8 bytes (0xC3 0xA1).
        RopeDocument::from_str("aábc")
    }

    fn doc_with_three_byte_char() -> RopeDocument {
        // '日' is three UTF-8 bytes (0xE6 0x97 0xA5).
        RopeDocument::from_str("a日b")
    }

    fn doc_with_emoji() -> RopeDocument {
        // '😀' (U+1F600) is four UTF-8 bytes.
        RopeDocument::from_str("a😀b")
    }

    #[test]
    fn move_left_steps_one_codepoint_across_two_byte_char() {
        let doc = doc_with_two_byte_char();
        let mut cursor = Cursor::new(3); // After 'á' (a=1, á=2 bytes => offset 3)
        cursor.move_left(&doc);
        // Should land on the byte-boundary *before* 'á', which is offset 1.
        assert_eq!(cursor.offset(), 1);
    }

    #[test]
    fn move_right_steps_one_codepoint_across_three_byte_char() {
        let doc = doc_with_three_byte_char();
        let mut cursor = Cursor::new(1); // After 'a', before '日'
        cursor.move_right(&doc);
        // Should skip all three bytes of '日' in a single step.
        assert_eq!(cursor.offset(), 4);
    }

    #[test]
    fn move_right_steps_one_codepoint_across_four_byte_emoji() {
        let doc = doc_with_emoji();
        let mut cursor = Cursor::new(1);
        cursor.move_right(&doc);
        assert_eq!(cursor.offset(), 5); // 1 + 4 bytes of emoji
    }

    #[test]
    fn move_left_at_start_of_document_is_idempotent() {
        let doc = RopeDocument::from_str("abc");
        let mut cursor = Cursor::new(0);
        cursor.move_left(&doc);
        assert_eq!(cursor.offset(), 0);
    }

    #[test]
    fn move_right_at_end_of_document_is_idempotent() {
        let doc = RopeDocument::from_str("abc");
        let mut cursor = Cursor::new(3);
        cursor.move_right(&doc);
        assert_eq!(cursor.offset(), 3);
    }

    // MARK: tests — word boundaries

    fn doc_with_words_and_spaces() -> RopeDocument {
        // Three words separated by single spaces: positions are
        //   foo   bar   baz
        //   0..3  4..7  8..11
        RopeDocument::from_str("foo bar baz")
    }

    #[test]
    fn move_word_right_skips_word_and_trailing_whitespace() {
        // From the start of "foo", one word-right hop should land at
        // the start of "bar" (offset 4) — the function skips the word,
        // then any trailing whitespace.
        let doc = doc_with_words_and_spaces();
        let mut cursor = Cursor::new(0);
        cursor.move_word_right(&doc);
        assert_eq!(cursor.offset(), 4);
    }

    #[test]
    fn move_word_right_chained_walks_across_each_word() {
        // Three hops from offset 0 should walk foo → bar → baz → end.
        let doc = doc_with_words_and_spaces();
        let mut cursor = Cursor::new(0);
        cursor.move_word_right(&doc);
        assert_eq!(cursor.offset(), 4);
        cursor.move_word_right(&doc);
        assert_eq!(cursor.offset(), 8);
        cursor.move_word_right(&doc);
        assert_eq!(cursor.offset(), 11);
    }

    #[test]
    fn move_word_left_walks_back_to_previous_word_start() {
        // From end of "baz" (offset 11), one word-left hop should
        // land at the start of "baz" (offset 8).
        let doc = doc_with_words_and_spaces();
        let mut cursor = Cursor::new(11);
        cursor.move_word_left(&doc);
        assert_eq!(cursor.offset(), 8);
    }

    // MARK: tests — bracket pairing

    fn doc_with_nested_brackets() -> RopeDocument {
        RopeDocument::from_str("a(b{c}d)e")
    }

    #[test]
    fn find_matching_bracket_pairs_outer_parens() {
        let doc = doc_with_nested_brackets();
        // '(' at offset 1 should match ')' at offset 7.
        assert_eq!(find_matching_bracket(&doc, 1), Some(7));
    }

    #[test]
    fn find_matching_bracket_pairs_inner_braces() {
        let doc = doc_with_nested_brackets();
        // '{' at offset 3 should match '}' at offset 5.
        assert_eq!(find_matching_bracket(&doc, 3), Some(5));
    }

    #[test]
    fn find_matching_bracket_works_for_closing_brackets_too() {
        let doc = doc_with_nested_brackets();
        assert_eq!(find_matching_bracket(&doc, 7), Some(1));
        assert_eq!(find_matching_bracket(&doc, 5), Some(3));
    }

    #[test]
    fn find_matching_bracket_returns_none_when_unbalanced() {
        let doc = RopeDocument::from_str("(((");
        assert_eq!(find_matching_bracket(&doc, 0), None);
    }

    #[test]
    fn find_matching_bracket_returns_none_when_not_a_bracket() {
        let doc = RopeDocument::from_str("hello");
        assert_eq!(find_matching_bracket(&doc, 2), None);
    }

    #[test]
    fn is_bracket_recognises_all_four_pair_kinds() {
        assert!(is_bracket('('));
        assert!(is_bracket(')'));
        assert!(is_bracket('['));
        assert!(is_bracket(']'));
        assert!(is_bracket('{'));
        assert!(is_bracket('}'));
        assert!(is_bracket('<'));
        assert!(is_bracket('>'));
        assert!(!is_bracket('a'));
        assert!(!is_bracket(' '));
    }

    #[test]
    fn find_matching_bracket_offset_past_end_returns_none() {
        let doc = doc_with_nested_brackets();
        // Offset beyond the document must not panic / wrap.
        assert_eq!(find_matching_bracket(&doc, 99), None);
    }

    #[test]
    fn find_nearby_bracket_finds_at_caret() {
        let doc = doc_with_nested_brackets();
        // Caret sits right on '(' at offset 1.
        assert_eq!(find_nearby_bracket(&doc, 1), Some((1, '(')));
    }

    #[test]
    fn find_nearby_bracket_finds_when_caret_is_just_past() {
        // "a()" — caret at offset 2 lands on the ')' character.
        let doc = RopeDocument::from_str("a()");
        assert_eq!(find_nearby_bracket(&doc, 2), Some((2, ')')));
    }

    #[test]
    fn find_nearby_bracket_falls_back_to_neighbour() {
        // "a(x" — caret on 'x' (offset 2). The current and right
        // positions are not brackets; the helper falls back to
        // the left position '(' at offset 1.
        let doc = RopeDocument::from_str("a(x");
        assert_eq!(find_nearby_bracket(&doc, 2), Some((1, '(')));
    }

    #[test]
    fn find_nearby_bracket_in_empty_doc_returns_none() {
        let doc = RopeDocument::from_str("");
        assert_eq!(find_nearby_bracket(&doc, 0), None);
    }

    #[test]
    fn find_nearby_bracket_with_no_brackets_returns_none() {
        let doc = RopeDocument::from_str("hello world");
        assert_eq!(find_nearby_bracket(&doc, 5), None);
    }
}
