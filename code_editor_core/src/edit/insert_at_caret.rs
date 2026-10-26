// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::cursor::Cursor;
use crate::document::Document;

/// Insert `text` at every caret as one undo step. `select` (relative to
/// the primary insertion point) selects part of the inserted text, e.g. a
/// snippet placeholder.
pub fn insert_at_caret(
    doc: &mut dyn Document,
    cursor: &mut Cursor,
    text: &str,
    select: Option<(usize, usize)>,
) {
    let insert_offset = cursor.selection.primary().end;
    doc.begin_edit_group(cursor.selection.regions());
    cursor.insert(doc, text);
    if let Some((start, end)) = select {
        cursor
            .selection
            .set_single(insert_offset + start, insert_offset + end);
    }
    doc.end_edit_group(cursor.selection.regions());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::RopeDocument;

    #[test]
    fn inserts_and_selects_placeholder() {
        let mut doc = RopeDocument::from_str("x");
        let mut cursor = Cursor::new(1);
        insert_at_caret(&mut doc, &mut cursor, "(arg)", Some((1, 4)));
        assert_eq!(doc.to_string(), "x(arg)");
        let r = cursor.selection.primary();
        assert_eq!((r.start, r.end), (2, 5));
    }
}
