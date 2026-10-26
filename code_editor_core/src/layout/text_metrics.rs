// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::document::Document;

/// Horizontal geometry the model needs for vertical movement and column
/// selection, implemented by whatever lays the text out (the masonry
/// widget's parley layout, or a fixed-width stub in tests).
///
/// Positions are expressed per **visual row**: with soft wrap, one
/// buffer line can occupy several rows, and moving up/down must step
/// through them rather than jump a whole buffer line.
pub trait TextMetrics {
    /// X position (content coordinates, before gutter/scroll) of `offset`
    /// within its visual row.
    fn offset_to_x(&self, doc: &dyn Document, offset: usize) -> f64;

    /// The byte offset closest to `x` on buffer line `line`'s first
    /// visual row.
    fn x_to_offset(&self, doc: &dyn Document, line: usize, x: f64) -> usize;

    /// Byte offset at `x` on buffer line `line`, or `None` when `x` lies
    /// past the end of the line (used by column selection).
    fn hit_test_line(&self, line: usize, x: f64) -> Option<usize>;

    /// The offset at `x` on the visual row above the one containing
    /// `offset`, or `None` at the first row of the document.
    fn offset_on_row_above(&self, doc: &dyn Document, offset: usize, x: f64) -> Option<usize>;

    /// The offset at `x` on the visual row below the one containing
    /// `offset`, or `None` at the last row of the document.
    fn offset_on_row_below(&self, doc: &dyn Document, offset: usize, x: f64) -> Option<usize>;
}
