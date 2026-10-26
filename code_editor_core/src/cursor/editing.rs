// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Cursor editing operations — delete, insert, word-deletion.
//!
//! These mutate the `Document` (unlike the movement methods, which
//! only walk it). All operations preserve cursor invariants:
//! horizontal-position memory is reset and selection regions
//! converge after the edit.

use super::Cursor;
use super::word_boundaries::{next_word_boundary, prev_word_boundary};
use crate::document::Document;
use crate::selection::SelRegion;

impl Cursor {
    // --- Editing operations ---

    /// Delete the character after the cursor (or delete the selection).
    // MARK: Editing ops — delete + insert (mutate document)

    pub fn delete(&mut self, doc: &mut dyn Document) {
        // Process regions in reverse order so earlier edits don't shift later offsets
        let regions: Vec<_> = self.selection.regions().to_vec();
        for region in regions.iter().rev() {
            if region.is_caret() {
                if region.end < doc.len_bytes() {
                    let text = doc.text();
                    let char_idx = text.byte_to_char(region.end);
                    let end = text.char_to_byte(char_idx + 1);
                    doc.edit(region.end..end, "");
                }
            } else {
                doc.edit(region.min()..region.max(), "");
            }
        }
        // Collapse all regions to carets at their start positions
        for region in self.selection.regions_mut() {
            let pos = region.min();
            region.start = pos;
            region.end = pos;
        }
    }

    /// Delete the character before the cursor (backspace).
    pub fn backdelete(&mut self, doc: &mut dyn Document) {
        let regions: Vec<_> = self.selection.regions().to_vec();
        // Track how much each region will delete (in original positions)
        let mut deleted_lens = Vec::with_capacity(regions.len());
        let mut new_positions = Vec::with_capacity(regions.len());

        // Process in reverse so earlier offsets aren't shifted by later edits
        for region in regions.iter().rev() {
            if region.is_caret() {
                if region.end > 0 {
                    let text = doc.text();
                    let char_idx = text.byte_to_char(region.end);
                    if char_idx > 0 {
                        let start = text.char_to_byte(char_idx - 1);
                        let deleted_len = region.end - start;
                        doc.edit(start..region.end, "");
                        new_positions.push(start);
                        deleted_lens.push(deleted_len);
                    } else {
                        new_positions.push(region.end);
                        deleted_lens.push(0);
                    }
                } else {
                    new_positions.push(0);
                    deleted_lens.push(0);
                }
            } else {
                let min = region.min();
                let deleted_len = region.max() - region.min();
                doc.edit(min..region.max(), "");
                new_positions.push(min);
                deleted_lens.push(deleted_len);
            }
        }

        // Positions were collected in reverse order — flip to match region order
        new_positions.reverse();
        deleted_lens.reverse();

        // Apply cumulative delta adjustment when setting cursor positions.
        // Each deletion shifts all subsequent cursor positions backwards.
        let mut cumulative_delta: isize = 0;
        for (i, pos) in new_positions.into_iter().enumerate() {
            let adjusted_pos = (pos as isize + cumulative_delta) as usize;
            let clamped = adjusted_pos.min(doc.len_bytes());
            self.selection.regions_mut()[i] = SelRegion::caret(clamped);
            cumulative_delta -= deleted_lens[i] as isize;
        }
    }

    /// Delete the word before the cursor.
    pub fn backdelete_word(&mut self, doc: &mut dyn Document) {
        let regions: Vec<_> = self.selection.regions().to_vec();
        let mut new_positions = Vec::with_capacity(regions.len());

        for region in regions.iter().rev() {
            if region.is_caret() {
                let word_start = prev_word_boundary(doc, region.end);
                if word_start < region.end {
                    doc.edit(word_start..region.end, "");
                    new_positions.push(word_start);
                } else {
                    new_positions.push(region.end);
                }
            } else {
                let min = region.min();
                doc.edit(min..region.max(), "");
                new_positions.push(min);
            }
        }

        new_positions.reverse();
        for (i, pos) in new_positions.into_iter().enumerate() {
            let clamped = pos.min(doc.len_bytes());
            self.selection.regions_mut()[i] = SelRegion::caret(clamped);
        }
    }

    /// Delete the word after the cursor.
    pub fn delete_word(&mut self, doc: &mut dyn Document) {
        let regions: Vec<_> = self.selection.regions().to_vec();
        for region in regions.iter().rev() {
            if region.is_caret() {
                let word_end = next_word_boundary(doc, region.end);
                if word_end > region.end {
                    doc.edit(region.end..word_end, "");
                }
            } else {
                doc.edit(region.min()..region.max(), "");
            }
        }
        for region in self.selection.regions_mut() {
            let pos = region.min().min(doc.len_bytes());
            region.start = pos;
            region.end = pos;
        }
    }

    /// Insert text at every cursor position simultaneously.
    /// Replaces any selected text with the insertion.
    pub fn insert(&mut self, doc: &mut dyn Document, text: &str) {
        let insert_len = text.len();
        // Process in reverse so earlier inserts don't invalidate later offsets
        let regions: Vec<_> = self.selection.regions().to_vec();
        // We need to track cumulative offset changes when processing forward
        // for updating cursor positions. Process edits in reverse, positions forward.
        let mut new_positions = vec![0usize; regions.len()];

        for (i, region) in regions.iter().enumerate().rev() {
            let edit_start = region.min();
            let edit_end = region.max();
            doc.edit(edit_start..edit_end, text);
            new_positions[i] = edit_start + insert_len;
        }

        // Now fix up positions accounting for cumulative shifts
        let mut cumulative_delta: isize = 0;
        for (i, region) in regions.iter().enumerate() {
            let old_len = region.max() - region.min();
            let adjusted_pos = (new_positions[i] as isize + cumulative_delta) as usize;
            self.selection.regions_mut()[i] = SelRegion::caret(adjusted_pos);
            cumulative_delta += insert_len as isize - old_len as isize;
        }
    }

    /// Returns the selected text for the primary cursor.
    /// Returns None if the primary cursor is a caret.
    pub fn selected_text<'a>(&self, doc: &'a dyn Document) -> Option<String> {
        let region = self.selection.primary();
        if region.is_caret() {
            return None;
        }
        let text = doc.text();
        let start_char = text.byte_to_char(region.min());
        let end_char = text.byte_to_char(region.max());
        Some(text.slice(start_char..end_char).to_string())
    }

    /// Get the cursor position as (line, col) — both 1-based, for display.
    pub fn position(&self, doc: &dyn Document) -> (usize, usize) {
        crate::document::position_of_offset(doc, self.offset())
    }
}
