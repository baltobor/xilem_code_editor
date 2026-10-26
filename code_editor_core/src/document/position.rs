// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use super::Document;

/// 1-based (line, column) of a byte offset; the column counts chars.
/// This is the convention of every line/col the editor reports.
pub fn position_of_offset(doc: &dyn Document, offset: usize) -> (usize, usize) {
    let text = doc.text();
    let char_offset = text.byte_to_char(offset.min(doc.len_bytes()));
    let line = text.char_to_line(char_offset);
    (line + 1, char_offset - text.line_to_char(line) + 1)
}

/// Byte offset of a 1-based (line, column) position, clamped to the line
/// (and the document).
pub fn offset_of_position(doc: &dyn Document, line: usize, col: usize) -> usize {
    let text = doc.text();
    let line0 = line
        .saturating_sub(1)
        .min(text.len_lines().saturating_sub(1));
    let line_start = text.line_to_char(line0);
    let line_chars = text.line(line0).len_chars();
    let char_offset = line_start + col.saturating_sub(1).min(line_chars);
    text.char_to_byte(char_offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::RopeDocument;

    #[test]
    fn round_trip_with_multibyte_chars() {
        let doc = RopeDocument::from_str("äb\nxyz");
        for offset in [0, 2, 3, 4, 5, 7] {
            let (line, col) = position_of_offset(&doc, offset);
            assert_eq!(
                offset_of_position(&doc, line, col),
                offset,
                "offset {offset}"
            );
        }
        assert_eq!(position_of_offset(&doc, 5), (2, 2));
    }

    #[test]
    fn clamps_out_of_range() {
        let doc = RopeDocument::from_str("ab\ncd");
        assert_eq!(offset_of_position(&doc, 9, 9), 5);
        assert_eq!(offset_of_position(&doc, 1, 9), 3); // end of line 1 incl. newline
    }
}
