// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Cursor movement methods (character, line, word) plus their
//! selection-extending variants.
//!
//! Pure given a `Document` reference (and a `Lines` for vertical
//! movement) — these never touch widget state, so they're trivially
//! testable in isolation. Existing tests in `mod.rs` cover the
//! multibyte UTF-8 boundary cases that triggered issue #33.

use super::Cursor;
use super::word_boundaries::{next_word_boundary, prev_word_boundary};
use crate::document::Document;
use crate::layout::ColPosition;
use crate::layout::TextMetrics;

impl Cursor {
    // MARK: Movement — character + line

    /// Move all cursors left by one character.
    pub fn move_left(&mut self, doc: &dyn Document) {
        self.horiz = None;
        let sel = &mut self.selection;
        for region in sel.regions_mut() {
            if !region.is_caret() {
                // Collapse to the left side of the selection
                let min = region.min();
                region.start = min;
                region.end = min;
            } else if region.end > 0 {
                // Move one char left using the rope for proper UTF-8 handling
                let text = doc.text();
                let char_idx = text.byte_to_char(region.end);
                if char_idx > 0 {
                    let new_offset = text.char_to_byte(char_idx - 1);
                    region.start = new_offset;
                    region.end = new_offset;
                }
            }
            region.horiz = None;
        }
    }

    /// Move all cursors right by one character.
    pub fn move_right(&mut self, doc: &dyn Document) {
        self.horiz = None;
        let sel = &mut self.selection;
        for region in sel.regions_mut() {
            if !region.is_caret() {
                // Collapse to the right side of the selection
                let max = region.max();
                region.start = max;
                region.end = max;
            } else if region.end < doc.len_bytes() {
                let text = doc.text();
                let char_idx = text.byte_to_char(region.end);
                let new_offset = text.char_to_byte(char_idx + 1);
                region.start = new_offset;
                region.end = new_offset;
            }
            region.horiz = None;
        }
    }

    /// Move all cursors up by one visual row (a wrapped line counts as
    /// several rows). From the first row the caret goes to the document start.
    pub fn move_up(&mut self, doc: &dyn Document, lines: &dyn TextMetrics) {
        self.step_rows(doc, lines, RowStep::Up, false);
    }

    /// Move all cursors down by one visual row. From the last row the caret
    /// goes to the document end.
    pub fn move_down(&mut self, doc: &dyn Document, lines: &dyn TextMetrics) {
        self.step_rows(doc, lines, RowStep::Down, false);
    }

    /// Move cursor to the start of the current line.
    pub fn move_to_line_start(&mut self, doc: &dyn Document) {
        self.horiz = None;
        for region in self.selection.regions_mut() {
            let line = doc.line_of_offset(region.end);
            let line_start = doc.offset_of_line(line);
            region.start = line_start;
            region.end = line_start;
            region.horiz = None;
        }
    }

    /// Move cursor to the end of the current line.
    pub fn move_to_line_end(&mut self, doc: &dyn Document) {
        self.horiz = None;
        for region in self.selection.regions_mut() {
            let line = doc.line_of_offset(region.end);
            if let Some(content) = doc.line_content(line) {
                let line_start = doc.offset_of_line(line);
                let line_end = line_start + content.len_bytes();
                region.start = line_end;
                region.end = line_end;
            }
            region.horiz = None;
        }
    }

    /// Move cursor to the start of the document.
    pub fn move_to_start(&mut self) {
        self.horiz = None;
        self.selection.set_caret(0);
    }

    /// Move cursor to the end of the document.
    pub fn move_to_end(&mut self, doc: &dyn Document) {
        self.horiz = None;
        self.selection.set_caret(doc.len_bytes());
    }

    /// Move cursor left by one word boundary.
    // MARK: Movement — word boundaries

    pub fn move_word_left(&mut self, doc: &dyn Document) {
        self.horiz = None;
        for region in self.selection.regions_mut() {
            let new_offset = prev_word_boundary(doc, region.end);
            region.start = new_offset;
            region.end = new_offset;
            region.horiz = None;
        }
    }

    /// Move cursor right by one word boundary.
    pub fn move_word_right(&mut self, doc: &dyn Document) {
        self.horiz = None;
        for region in self.selection.regions_mut() {
            let new_offset = next_word_boundary(doc, region.end);
            region.start = new_offset;
            region.end = new_offset;
            region.horiz = None;
        }
    }

    // MARK: Selection — extend variants of every move

    /// Extend selection left by one character.
    pub fn select_left(&mut self, doc: &dyn Document) {
        self.horiz = None;
        for region in self.selection.regions_mut() {
            if region.end > 0 {
                let text = doc.text();
                let char_idx = text.byte_to_char(region.end);
                if char_idx > 0 {
                    region.end = text.char_to_byte(char_idx - 1);
                }
            }
            region.horiz = None;
        }
    }

    /// Extend selection right by one character.
    pub fn select_right(&mut self, doc: &dyn Document) {
        self.horiz = None;
        for region in self.selection.regions_mut() {
            if region.end < doc.len_bytes() {
                let text = doc.text();
                let char_idx = text.byte_to_char(region.end);
                region.end = text.char_to_byte(char_idx + 1);
            }
            region.horiz = None;
        }
    }

    /// Extend selection up by one visual row.
    pub fn select_up(&mut self, doc: &dyn Document, lines: &dyn TextMetrics) {
        self.step_rows(doc, lines, RowStep::Up, true);
    }

    /// Extend selection down by one visual row.
    pub fn select_down(&mut self, doc: &dyn Document, lines: &dyn TextMetrics) {
        self.step_rows(doc, lines, RowStep::Down, true);
    }

    /// Shared body of the four vertical moves. Keeps each region's preferred
    /// x (`horiz`) so passing through shorter rows doesn't drift the column.
    fn step_rows(
        &mut self,
        doc: &dyn Document,
        lines: &dyn TextMetrics,
        step: RowStep,
        extend: bool,
    ) {
        for region in self.selection.regions_mut() {
            let horiz = region
                .horiz
                .unwrap_or_else(|| ColPosition(lines.offset_to_x(doc, region.end)));
            let target = match step {
                RowStep::Up => lines
                    .offset_on_row_above(doc, region.end, horiz.0)
                    .unwrap_or(0),
                RowStep::Down => lines
                    .offset_on_row_below(doc, region.end, horiz.0)
                    .unwrap_or_else(|| doc.len_bytes()),
            };
            region.end = target;
            if !extend {
                region.start = target;
            }
            region.horiz = Some(horiz);
        }
    }

    /// Extend selection to word boundary left.
    pub fn select_word_left(&mut self, doc: &dyn Document) {
        self.horiz = None;
        for region in self.selection.regions_mut() {
            region.end = prev_word_boundary(doc, region.end);
            region.horiz = None;
        }
    }

    /// Extend selection to word boundary right.
    pub fn select_word_right(&mut self, doc: &dyn Document) {
        self.horiz = None;
        for region in self.selection.regions_mut() {
            region.end = next_word_boundary(doc, region.end);
            region.horiz = None;
        }
    }

    /// Extend selection to line start.
    pub fn select_to_line_start(&mut self, doc: &dyn Document) {
        self.horiz = None;
        for region in self.selection.regions_mut() {
            let line = doc.line_of_offset(region.end);
            region.end = doc.offset_of_line(line);
            region.horiz = None;
        }
    }

    /// Extend selection to line end.
    pub fn select_to_line_end(&mut self, doc: &dyn Document) {
        self.horiz = None;
        for region in self.selection.regions_mut() {
            let line = doc.line_of_offset(region.end);
            if let Some(content) = doc.line_content(line) {
                region.end = doc.offset_of_line(line) + content.len_bytes();
            }
            region.horiz = None;
        }
    }

    /// Extend selection to document start.
    pub fn select_to_start(&mut self) {
        self.horiz = None;
        self.selection.primary_mut().end = 0;
    }

    /// Extend selection to document end.
    pub fn select_to_end(&mut self, doc: &dyn Document) {
        self.horiz = None;
        self.selection.primary_mut().end = doc.len_bytes();
    }

    /// Select the entire document.
    pub fn select_all(&mut self, doc: &dyn Document) {
        self.horiz = None;
        self.selection.set_single(0, doc.len_bytes());
    }

    /// Collapse selection to a caret (deselect).
    pub fn collapse_selection(&mut self) {
        for region in self.selection.regions_mut() {
            region.start = region.end;
        }
    }
}

/// Direction of a vertical cursor move.
#[derive(Clone, Copy)]
enum RowStep {
    Up,
    Down,
}
