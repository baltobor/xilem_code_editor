// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0
// Adapted from Lapce (Apache-2.0) and Floem (MIT); see README.md.

//! Multi-region selection model for the code editor.
//!
//! Supports multiple cursors and column/block selection.
//! Adapted from Lapce's proven selection model, simplified for our needs.
//!
//! Key invariants:
//! - Regions are always sorted by start offset
//! - Overlapping regions are merged automatically
//! - A zero-width region (start == end) represents a caret

use crate::document::Document;
use crate::layout::ColPosition;
use crate::layout::TextMetrics;

/// A single contiguous selection region defined by two byte offsets.
///
/// `start` is the anchor (where the selection began) and `end` is the
/// active/focus point (where the cursor is). When start == end, this
/// is a caret (zero-width cursor).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelRegion {
    /// Anchor byte offset in the document.
    pub start: usize,
    /// Active/focus byte offset in the document (where the cursor renders).
    pub end: usize,
    /// Preserved horizontal pixel position for vertical movement.
    /// Set when moving up/down, cleared on horizontal movement.
    pub horiz: Option<ColPosition>,
}

// Manual PartialEq/Eq for ColPosition (f64 doesn't impl Eq)
impl PartialEq for ColPosition {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}
impl Eq for ColPosition {}

impl SelRegion {
    /// Create a caret (zero-width cursor) at the given byte offset.
    pub fn caret(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
            horiz: None,
        }
    }

    /// Create a selection region spanning from `start` to `end`.
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            horiz: None,
        }
    }

    /// Returns true if this region is a caret (zero-width).
    pub fn is_caret(&self) -> bool {
        self.start == self.end
    }

    /// Returns the leftmost byte offset (min of start and end).
    pub fn min(&self) -> usize {
        self.start.min(self.end)
    }

    /// Returns the rightmost byte offset (max of start and end).
    pub fn max(&self) -> usize {
        self.start.max(self.end)
    }
}

/// An ordered, non-overlapping collection of selection regions.
///
/// Supports multiple cursors and column/block selection. Regions are always
/// kept sorted by their start offset, and overlapping regions are merged
/// when added.
#[derive(Debug, Clone)]
pub struct Selection {
    regions: Vec<SelRegion>,
}

impl Selection {
    /// Create a selection with a single caret at the given offset.
    pub fn caret(offset: usize) -> Self {
        Self {
            regions: vec![SelRegion::caret(offset)],
        }
    }

    /// Create a selection from a vector of regions.
    /// If empty, defaults to a caret at position 0.
    pub fn from_regions(regions: Vec<SelRegion>) -> Self {
        if regions.is_empty() {
            Self::caret(0)
        } else {
            Self { regions }
        }
    }

    /// Create a selection with a single region.
    pub fn region(start: usize, end: usize) -> Self {
        Self {
            regions: vec![SelRegion::new(start, end)],
        }
    }

    /// Returns true when all regions are zero-width carets.
    pub fn is_caret(&self) -> bool {
        self.regions.iter().all(|r| r.is_caret())
    }

    /// Returns the number of regions.
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    /// Returns true if there are no regions.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Returns the primary (most recently added) region.
    /// This is always the last region in the sorted list.
    pub fn primary(&self) -> &SelRegion {
        self.regions
            .last()
            .expect("Selection must have at least one region")
    }

    /// Returns a mutable reference to the primary region.
    pub fn primary_mut(&mut self) -> &mut SelRegion {
        self.regions
            .last_mut()
            .expect("Selection must have at least one region")
    }

    /// Iterate over all regions.
    pub fn regions(&self) -> &[SelRegion] {
        &self.regions
    }

    /// Mutable access to all regions.
    pub fn regions_mut(&mut self) -> &mut Vec<SelRegion> {
        &mut self.regions
    }

    /// Add a region, maintaining sort order and merging overlaps.
    pub fn add_region(&mut self, region: SelRegion) {
        // Find insertion position (sorted by min offset)
        let pos = self.regions.partition_point(|r| r.min() < region.min());
        self.regions.insert(pos, region);
        self.merge_overlapping();
    }

    /// Replace all regions with a single caret.
    pub fn set_caret(&mut self, offset: usize) {
        self.regions.clear();
        self.regions.push(SelRegion::caret(offset));
    }

    /// Replace all regions with a single selection.
    pub fn set_single(&mut self, start: usize, end: usize) {
        self.regions.clear();
        self.regions.push(SelRegion::new(start, end));
    }

    /// Adjust all region offsets after a buffer edit.
    ///
    /// When text is inserted or deleted, all regions after the edit point
    /// need their offsets shifted. Regions that overlap the edit are collapsed.
    pub fn apply_edit(&mut self, edit_start: usize, old_len: usize, new_len: usize) {
        let edit_end = edit_start + old_len;
        let delta = new_len as isize - old_len as isize;

        for region in &mut self.regions {
            // Adjust start
            region.start = adjust_offset(region.start, edit_start, edit_end, delta);
            // Adjust end
            region.end = adjust_offset(region.end, edit_start, edit_end, delta);
        }
    }

    /// Create a column (rectangular) selection between two points.
    ///
    /// Produces one SelRegion per line between start_line and end_line,
    /// each spanning from col_start to col_end in pixel coordinates.
    pub fn column_select(
        document: &dyn Document,
        lines: &dyn TextMetrics,
        start_line: usize,
        end_line: usize,
        col_start: f64,
        col_end: f64,
    ) -> Self {
        let (first, last) = if start_line <= end_line {
            (start_line, end_line)
        } else {
            (end_line, start_line)
        };

        let mut regions = Vec::with_capacity(last - first + 1);

        for buf_line in first..=last {
            // Convert pixel x positions to byte offsets within this line
            let line_start_offset = document.offset_of_line(buf_line);
            let start_offset = lines
                .hit_test_line(buf_line, col_start)
                .map(|off| line_start_offset + off)
                .unwrap_or(line_start_offset);
            let end_offset = lines
                .hit_test_line(buf_line, col_end)
                .map(|off| line_start_offset + off)
                .unwrap_or(line_start_offset);

            regions.push(SelRegion::new(start_offset, end_offset));
        }

        Self { regions }
    }

    /// Merge overlapping or adjacent regions.
    fn merge_overlapping(&mut self) {
        if self.regions.len() < 2 {
            return;
        }

        let mut merged = Vec::with_capacity(self.regions.len());
        let mut current = self.regions[0].clone();

        for region in &self.regions[1..] {
            if region.min() <= current.max() {
                // Overlapping — extend the current region
                if region.max() > current.max() {
                    // Keep the anchor of the earlier region, extend the end
                    current.end = region.end;
                }
            } else {
                merged.push(current);
                current = region.clone();
            }
        }
        merged.push(current);
        self.regions = merged;
    }
}

/// Adjust a single byte offset after an edit operation.
#[allow(dead_code)]
fn adjust_offset(offset: usize, edit_start: usize, edit_end: usize, delta: isize) -> usize {
    if offset <= edit_start {
        // Before the edit — unchanged
        offset
    } else if offset >= edit_end {
        // After the edit — shift by delta
        (offset as isize + delta).max(0) as usize
    } else {
        // Inside the deleted range — collapse to edit start
        edit_start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caret_is_zero_width() {
        let sel = Selection::caret(10);
        assert!(sel.is_caret());
        assert_eq!(sel.primary().start, 10);
        assert_eq!(sel.primary().end, 10);
    }

    #[test]
    fn region_min_max() {
        let r = SelRegion::new(5, 3); // backwards selection
        assert_eq!(r.min(), 3);
        assert_eq!(r.max(), 5);
    }

    #[test]
    fn merge_overlapping_regions() {
        let mut sel = Selection::caret(0);
        sel.add_region(SelRegion::new(0, 10));
        sel.add_region(SelRegion::new(5, 15));
        // Should merge into one region spanning 0..15
        assert_eq!(sel.len(), 1);
        assert_eq!(sel.primary().min(), 0);
        assert_eq!(sel.primary().max(), 15);
    }

    #[test]
    fn apply_edit_shifts_offsets() {
        let mut sel = Selection::caret(20);
        // Insert 5 bytes at position 10
        sel.apply_edit(10, 0, 5);
        assert_eq!(sel.primary().end, 25);
    }

    #[test]
    fn apply_edit_shifts_after_delete() {
        let mut sel = Selection::region(5, 15);
        // Delete bytes 8..12 (4 bytes removed)
        sel.apply_edit(8, 4, 0);
        assert_eq!(sel.primary().start, 5);
        // end=15 was after the deleted range (8..12), so it shifts by -4
        assert_eq!(sel.primary().end, 11);
    }

    #[test]
    fn apply_edit_collapses_inside_delete() {
        let mut sel = Selection::region(5, 10);
        // Delete bytes 8..12 — end=10 is inside the deleted range
        sel.apply_edit(8, 4, 0);
        assert_eq!(sel.primary().start, 5);
        assert_eq!(sel.primary().end, 8); // collapsed to edit start
    }
}
