// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::cursor::Cursor;
use crate::document::Document;

use super::TextEdit;

/// Apply a batch of edits (e.g. a quick fix's workspace edit) as **one**
/// undo step. Edits are applied in reverse document order so earlier
/// edits don't shift the offsets of later ones; the caret ends after the
/// first edit in document order. Returns `true` if anything changed.
pub fn apply_text_edits(doc: &mut dyn Document, cursor: &mut Cursor, edits: &[TextEdit]) -> bool {
    let mut ordered: Vec<&TextEdit> = edits.iter().collect();
    ordered.sort_by(|a, b| b.range.start.cmp(&a.range.start));

    doc.begin_edit_group(cursor.selection.regions());
    let mut changed = false;
    for edit in ordered {
        let Some(range) = edit.byte_range(&doc.to_string()) else {
            continue;
        };
        if range.end > doc.len_bytes() {
            continue;
        }
        doc.edit(range.clone(), &edit.new_text);
        cursor
            .selection
            .set_caret(range.start + edit.new_text.len());
        changed = true;
    }
    doc.end_edit_group(cursor.selection.regions());
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::RopeDocument;
    use crate::edit::TextPosition;

    #[test]
    fn batch_is_order_independent_and_one_undo_step() {
        let mut doc = RopeDocument::from_str("a b c");
        let mut cursor = Cursor::new(0);
        let edits = [
            TextEdit::new(TextPosition::new(0, 0)..TextPosition::new(0, 1), "A"),
            TextEdit::new(TextPosition::new(0, 4)..TextPosition::new(0, 5), "C"),
        ];
        assert!(apply_text_edits(&mut doc, &mut cursor, &edits));
        assert_eq!(doc.to_string(), "A b C");
        doc.undo();
        assert_eq!(doc.to_string(), "a b c");
    }
}
