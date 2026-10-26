// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::cursor::Cursor;
use crate::document::Document;
use crate::snippet::{SnippetSession, TabStop};

use super::TextEdit;

/// Apply one edit (typically an accepted completion) as one undo step.
///
/// The caret lands at `cursor_offset_in_new_text` (a snippet's `$0`) or
/// at the end of the inserted text. When the snippet has navigable tab
/// stops, a [`SnippetSession`] is returned with its first stop selected.
pub fn apply_text_edit(
    doc: &mut dyn Document,
    cursor: &mut Cursor,
    edit: &TextEdit,
    cursor_offset_in_new_text: Option<usize>,
    tab_stops: &[TabStop],
) -> Option<SnippetSession> {
    let range = edit.byte_range(&doc.to_string())?;
    if range.end > doc.len_bytes() {
        return None;
    }
    doc.begin_edit_group(cursor.selection.regions());
    doc.edit(range.clone(), &edit.new_text);

    let offset_in_new = cursor_offset_in_new_text
        .filter(|off| *off <= edit.new_text.len() && edit.new_text.is_char_boundary(*off))
        .unwrap_or(edit.new_text.len());
    cursor.selection.set_caret(range.start + offset_in_new);

    let session = SnippetSession::start(tab_stops, range.start);
    if let Some(first) = session.as_ref().and_then(SnippetSession::current) {
        cursor
            .selection
            .set_single(first.range.start, first.range.end);
    }
    doc.end_edit_group(cursor.selection.regions());
    session
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::RopeDocument;
    use crate::edit::TextPosition;

    #[test]
    fn caret_at_snippet_zero_offset() {
        let mut doc = RopeDocument::from_str("is_n");
        let mut cursor = Cursor::new(4);
        let edit = TextEdit::new(TextPosition::new(0, 0)..TextPosition::new(0, 4), "is_nan()");
        apply_text_edit(&mut doc, &mut cursor, &edit, Some(7), &[]);
        assert_eq!(doc.to_string(), "is_nan()");
        assert_eq!(cursor.selection.primary().end, 7);
    }
}
