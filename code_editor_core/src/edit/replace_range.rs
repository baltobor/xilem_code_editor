// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use crate::cursor::Cursor;
use crate::document::Document;

/// Replace `range` with `new_text` as one undo step and put the caret
/// after the inserted text. Returns `false` (no change) for an invalid
/// range.
pub fn replace_range(
    doc: &mut dyn Document,
    cursor: &mut Cursor,
    range: Range<usize>,
    new_text: &str,
) -> bool {
    if range.start > range.end || range.end > doc.len_bytes() {
        return false;
    }
    doc.begin_edit_group(cursor.selection.regions());
    doc.edit(range.clone(), new_text);
    cursor.selection.set_caret(range.start + new_text.len());
    doc.end_edit_group(cursor.selection.regions());
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::RopeDocument;

    #[test]
    fn replaces_and_places_caret_after() {
        let mut doc = RopeDocument::from_str("hello world");
        let mut cursor = Cursor::new(0);
        assert!(replace_range(&mut doc, &mut cursor, 6..11, "there"));
        assert_eq!(doc.to_string(), "hello there");
        assert_eq!(cursor.selection.primary().end, 11);
    }

    #[test]
    fn one_undo_step() {
        let mut doc = RopeDocument::from_str("abc");
        let mut cursor = Cursor::new(0);
        replace_range(&mut doc, &mut cursor, 0..3, "xyz");
        doc.undo();
        assert_eq!(doc.to_string(), "abc");
    }

    #[test]
    fn rejects_invalid_range() {
        let mut doc = RopeDocument::from_str("abc");
        let mut cursor = Cursor::new(0);
        assert!(!replace_range(&mut doc, &mut cursor, 2..9, "x"));
        assert_eq!(doc.to_string(), "abc");
    }
}
