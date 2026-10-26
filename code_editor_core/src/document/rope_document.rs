// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! RopeDocument — the default Document implementation backed by `ropey::Rope`.
//!
//! Ropey provides O(log n) edits and line lookups, making it efficient for
//! large files (Cargo.lock, generated code, etc.) where a plain String would
//! cause O(n) copies on every keystroke.

use std::ops::Range;

use ropey::{Rope, RopeSlice};

use crate::document::Document;
use crate::document::UndoManager;
use crate::selection::SelRegion;

/// A document backed by `ropey::Rope` — efficient for files of any size.
///
/// The revision counter starts at 1 (a freshly loaded document has revision 1)
/// and increments on every call to `edit()`. The layout cache uses this to
/// detect when it needs to recompute line layouts.
pub struct RopeDocument {
    rope: Rope,
    revision: u64,
    undo: UndoManager,
}

impl RopeDocument {
    /// Create a new document from a string slice.
    /// This is the normal path for loading a file from disk.
    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            revision: 1,
            undo: UndoManager::new(),
        }
    }

    /// Create an empty document.
    pub fn empty() -> Self {
        Self {
            rope: Rope::new(),
            revision: 1,
            undo: UndoManager::new(),
        }
    }
}

impl Document for RopeDocument {
    fn text(&self) -> &Rope {
        &self.rope
    }

    fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    fn line_content(&self, line: usize) -> Option<RopeSlice<'_>> {
        if line >= self.rope.len_lines() {
            return None;
        }
        let slice = self.rope.line(line);
        // Ropey includes the trailing newline in each line slice.
        // Strip it so callers get just the line content.
        let len = slice.len_chars();
        if len > 0 && slice.char(len - 1) == '\n' {
            // Also strip \r before \n for Windows line endings
            let end = if len > 1 && slice.char(len - 2) == '\r' {
                len - 2
            } else {
                len - 1
            };
            Some(slice.slice(..end))
        } else {
            Some(slice)
        }
    }

    fn offset_of_line(&self, line: usize) -> usize {
        // Ropey works in char offsets; we need byte offsets for Parley.
        let char_offset = self.rope.line_to_char(line);
        self.rope.char_to_byte(char_offset)
    }

    fn line_of_offset(&self, offset: usize) -> usize {
        let clamped = offset.min(self.rope.len_bytes());
        let char_offset = self.rope.byte_to_char(clamped);
        self.rope.char_to_line(char_offset)
    }

    fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    fn edit(&mut self, range: Range<usize>, new_text: &str) {
        // Convert byte offsets to char offsets for ropey
        let start_char = self.rope.byte_to_char(range.start);
        let end_char = self.rope.byte_to_char(range.end);

        // Record old text for undo before mutating
        let old_text = if start_char != end_char {
            self.rope.slice(start_char..end_char).to_string()
        } else {
            String::new()
        };
        self.undo
            .record(range.clone(), old_text, new_text.to_string());

        // Remove the old range, then insert the new text
        if start_char != end_char {
            self.rope.remove(start_char..end_char);
        }
        if !new_text.is_empty() {
            self.rope.insert(start_char, new_text);
        }

        self.revision += 1;
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn to_string(&self) -> String {
        self.rope.to_string()
    }

    fn begin_edit_group(&mut self, cursor: &[SelRegion]) {
        self.undo.begin(cursor);
    }

    fn end_edit_group(&mut self, cursor: &[SelRegion]) {
        self.undo.end(cursor);
    }

    fn undo(&mut self) -> Option<Vec<SelRegion>> {
        self.undo.undo(&mut self.rope, &mut self.revision)
    }

    fn redo(&mut self) -> Option<Vec<SelRegion>> {
        self.undo.redo(&mut self.rope, &mut self.revision)
    }

    fn clear_history(&mut self) {
        self.undo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_edit() {
        let mut doc = RopeDocument::from_str("hello world");
        assert_eq!(doc.line_count(), 1);
        assert_eq!(doc.revision(), 1);

        // Replace "world" with "rust"
        doc.edit(6..11, "rust");
        assert_eq!(doc.to_string(), "hello rust");
        assert_eq!(doc.revision(), 2);
    }

    #[test]
    fn multiline_document() {
        let doc = RopeDocument::from_str("line one\nline two\nline three\n");
        assert_eq!(doc.line_count(), 4); // trailing newline = extra empty line
        assert_eq!(doc.line_content(0).unwrap().to_string(), "line one");
        assert_eq!(doc.line_content(1).unwrap().to_string(), "line two");
        assert_eq!(doc.line_content(2).unwrap().to_string(), "line three");
    }

    #[test]
    fn offset_conversions() {
        let doc = RopeDocument::from_str("abc\ndef\nghi");
        // Line 0 starts at byte 0, line 1 at byte 4, line 2 at byte 8
        assert_eq!(doc.offset_of_line(0), 0);
        assert_eq!(doc.offset_of_line(1), 4);
        assert_eq!(doc.offset_of_line(2), 8);
        assert_eq!(doc.line_of_offset(0), 0);
        assert_eq!(doc.line_of_offset(5), 1); // 'd' in "def"
        assert_eq!(doc.line_of_offset(9), 2); // 'h' in "ghi"
    }

    #[test]
    fn insert_at_start() {
        let mut doc = RopeDocument::from_str("world");
        doc.edit(0..0, "hello ");
        assert_eq!(doc.to_string(), "hello world");
    }

    #[test]
    fn delete_range() {
        let mut doc = RopeDocument::from_str("hello world");
        doc.edit(5..11, "");
        assert_eq!(doc.to_string(), "hello");
    }

    #[test]
    fn empty_document() {
        let doc = RopeDocument::empty();
        assert!(doc.is_empty());
        assert_eq!(doc.line_count(), 1); // ropey: empty rope has 1 line
        assert_eq!(doc.len_bytes(), 0);
    }
}
