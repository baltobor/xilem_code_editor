// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Visual-row geometry. With soft wrap a buffer line occupies several
//! rows; every offset <-> position mapping here asks parley about the
//! actual row (its y), never only about row 0.

use masonry::kurbo::Rect;
use masonry::parley::editing::{Cursor as ParleyCursor, Selection as ParleySelection};
use masonry::parley::layout::Affinity;

use code_editor_core::document::Document;
use code_editor_core::layout::TextMetrics;

use super::{LineLayout, Lines};

/// Caret geometry relative to the top-left of its buffer line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretGeometry {
    pub x: f64,
    /// Top of the visual row within the line.
    pub y: f64,
    pub height: f64,
}

impl Lines {
    /// Caret position of `offset` within its buffer line, including the
    /// visual row it sits on.
    pub fn caret_geometry(&self, doc: &dyn Document, offset: usize) -> CaretGeometry {
        let line = doc.line_of_offset(offset);
        let buffer_col = offset - doc.offset_of_line(line);
        match self.get_layout(line) {
            Some(layout) => {
                let visual_col = layout.phantoms.col_at(buffer_col);
                let cursor =
                    ParleyCursor::from_byte_index(&layout.layout, visual_col, Affinity::Downstream);
                let geom = cursor.geometry(&layout.layout, 1.5);
                CaretGeometry {
                    x: geom.x0,
                    y: geom.y0,
                    height: geom.y1 - geom.y0,
                }
            }
            None => CaretGeometry {
                x: buffer_col as f64 * self.char_width(),
                y: 0.0,
                height: self.line_height(),
            },
        }
    }

    /// Byte offset at (`x`, `y`) where `y` is relative to the top of
    /// buffer line `line` — so a click on the second visual row lands on
    /// the second row.
    pub fn point_to_offset(&self, doc: &dyn Document, line: usize, x: f64, y: f64) -> usize {
        match self.get_layout(line) {
            Some(layout) => doc.offset_of_line(line) + buffer_col_at_point(layout, x, y),
            None => self.x_to_offset(doc, line, x),
        }
    }

    /// Selection rectangles for buffer columns `start_col..end_col` of
    /// `line`: one per visual row, relative to the line's top-left.
    /// Phantom (inlay) text before the selection start is not covered.
    pub fn selection_rects(&self, line: usize, start_col: usize, end_col: usize) -> Vec<Rect> {
        let Some(layout) = self.get_layout(line) else {
            return Vec::new();
        };
        let anchor = layout.phantoms.col_at_for_selection(start_col);
        let focus = layout.phantoms.col_at(end_col);
        let selection = ParleySelection::new(
            ParleyCursor::from_byte_index(&layout.layout, anchor, Affinity::Downstream),
            ParleyCursor::from_byte_index(&layout.layout, focus, Affinity::Upstream),
        );
        selection
            .geometry(&layout.layout)
            .into_iter()
            .map(|(bb, _)| Rect::new(bb.x0, bb.y0, bb.x1, bb.y1))
            .collect()
    }

    /// Byte offset at document point (`x`, `y`): finds the buffer line,
    /// then the visual row within it.
    pub fn offset_at_point(&self, doc: &dyn Document, x: f64, y: f64) -> usize {
        let line = self.line_at_y(y);
        self.point_to_offset(doc, line, x, (y - self.y_of_line(line)).max(0.0))
    }

    /// Height of buffer line `line` including all its visual rows.
    pub fn line_block_height(&self, line: usize) -> f64 {
        self.get_layout(line)
            .map_or(self.line_height(), |l| l.line_height)
    }
}

fn buffer_col_at_point(layout: &LineLayout, x: f64, y: f64) -> usize {
    let cursor = ParleyCursor::from_point(&layout.layout, x as f32, y as f32);
    layout.phantoms.buffer_col_at(cursor.index())
}

impl TextMetrics for Lines {
    fn offset_to_x(&self, doc: &dyn Document, offset: usize) -> f64 {
        self.caret_geometry(doc, offset).x
    }

    fn x_to_offset(&self, doc: &dyn Document, line: usize, x: f64) -> usize {
        Lines::x_to_offset(self, doc, line, x)
    }

    fn hit_test_line(&self, line: usize, x: f64) -> Option<usize> {
        Lines::hit_test_line(self, line, x)
    }

    fn offset_on_row_above(&self, doc: &dyn Document, offset: usize, x: f64) -> Option<usize> {
        let line = doc.line_of_offset(offset);
        let caret = self.caret_geometry(doc, offset);
        if caret.y > 0.5 {
            // Another visual row of the same buffer line lies above.
            return Some(self.point_to_offset(doc, line, x, caret.y - caret.height / 2.0));
        }
        let prev = line.checked_sub(1)?;
        // Last visual row of the previous buffer line.
        let y = self.line_block_height(prev) - self.line_height() / 2.0;
        Some(self.point_to_offset(doc, prev, x, y.max(0.0)))
    }

    fn offset_on_row_below(&self, doc: &dyn Document, offset: usize, x: f64) -> Option<usize> {
        let line = doc.line_of_offset(offset);
        let caret = self.caret_geometry(doc, offset);
        let row_bottom = caret.y + caret.height;
        if row_bottom + 0.5 < self.line_block_height(line) {
            // Another visual row of the same buffer line lies below.
            return Some(self.point_to_offset(doc, line, x, row_bottom + caret.height / 2.0));
        }
        let next = line + 1;
        if next >= doc.line_count() {
            return None;
        }
        Some(self.point_to_offset(doc, next, x, self.line_height() / 2.0))
    }
}

#[cfg(test)]
mod tests {
    use code_editor_core::document::RopeDocument;
    use masonry::core::BrushIndex;
    use masonry::parley::{FontContext, LayoutContext};

    use super::*;
    use crate::properties::EditorTheme;

    const LONG_LINE: &str =
        "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi";

    /// Lay out `text` wrapped at `width` px (line 0 wrapped, others not).
    fn wrapped(text: &str, width: f32) -> (RopeDocument, Lines) {
        let doc = RopeDocument::from_str(text);
        let mut lines = Lines::new(doc.line_count(), 14.0);
        let mut fcx = FontContext::new();
        let mut lcx = LayoutContext::<BrushIndex>::new();
        let palette = EditorTheme::default().syntax;
        for line in 0..doc.line_count() {
            lines.ensure_layout(line, &doc, &mut fcx, &mut lcx, &palette, Some(width));
        }
        (doc, lines)
    }

    fn rows_of_line0(lines: &Lines) -> usize {
        lines.get_layout(0).map_or(0, |l| l.visual_line_count)
    }

    #[test]
    fn selection_covers_every_visual_row() {
        let (_doc, lines) = wrapped(LONG_LINE, 150.0);
        let rows = rows_of_line0(&lines);
        assert!(rows >= 2, "test needs a wrapped line, got {rows} row(s)");
        let rects = lines.selection_rects(0, 0, LONG_LINE.len());
        assert_eq!(rects.len(), rows, "one rectangle per visual row");
        for pair in rects.windows(2) {
            // Each rectangle is on its own row, below the previous one
            // (parley rounds row boxes, so neighbors may touch by 1 px).
            assert!(
                pair[1].y0 > pair[0].y0 + 1.0,
                "rows stack vertically: {rects:?}"
            );
        }
    }

    #[test]
    fn click_on_second_row_lands_on_second_row() {
        let (doc, lines) = wrapped(LONG_LINE, 150.0);
        let row_height = lines.line_block_height(0) / rows_of_line0(&lines) as f64;
        let first = lines.point_to_offset(&doc, 0, 10.0, row_height * 0.5);
        let second = lines.point_to_offset(&doc, 0, 10.0, row_height * 1.5);
        assert!(
            second > first,
            "second row starts later in the line ({first} vs {second})"
        );
        assert!(lines.caret_geometry(&doc, second).y >= row_height - 0.5);
    }

    #[test]
    fn row_below_stays_inside_the_wrapped_line() {
        let text = format!("{LONG_LINE}\nnext");
        let (doc, lines) = wrapped(&text, 150.0);
        let below = lines
            .offset_on_row_below(&doc, 0, 0.0)
            .expect("a row below");
        assert_eq!(
            doc.line_of_offset(below),
            0,
            "first step down stays on the wrapped line"
        );
        assert!(below > 0);
    }

    #[test]
    fn rewrap_on_width_change() {
        let (doc, mut lines) = wrapped(LONG_LINE, 150.0);
        let narrow_rows = rows_of_line0(&lines);
        let mut fcx = FontContext::new();
        let mut lcx = LayoutContext::<BrushIndex>::new();
        let palette = EditorTheme::default().syntax;
        lines.ensure_layout(0, &doc, &mut fcx, &mut lcx, &palette, Some(2000.0));
        assert!(
            rows_of_line0(&lines) < narrow_rows,
            "a wider viewport re-wraps the cached line"
        );
    }
}

#[cfg(test)]
mod highlight_tests {
    use code_editor_core::document::{Document, RopeDocument};
    use code_editor_core::highlight::{HighlightCategory, HighlightSpan};
    use masonry::core::BrushIndex;
    use masonry::parley::{FontContext, LayoutContext};

    use crate::properties::EditorTheme;
    use crate::text_layout::Lines;

    fn span(range: std::ops::Range<usize>) -> HighlightSpan {
        HighlightSpan {
            byte_range: range,
            category: HighlightCategory::Keyword,
        }
    }

    #[test]
    fn unchanged_highlights_keep_cached_layouts() {
        let doc = RopeDocument::from_str("let a = 1;\nlet b = 2;");
        let mut lines = Lines::new(doc.line_count(), 14.0);
        let (mut fcx, mut lcx) = (FontContext::new(), LayoutContext::<BrushIndex>::new());
        let palette = EditorTheme::default().syntax;
        let spans = vec![vec![span(0..3)], vec![span(0..3)]];
        lines.set_highlights(spans.clone());
        for line in 0..2 {
            lines.ensure_layout(line, &doc, &mut fcx, &mut lcx, &palette, None);
        }

        lines.set_highlights(spans.clone());
        assert!(
            lines.get_layout(0).is_some() && lines.get_layout(1).is_some(),
            "same colors: layouts kept"
        );

        let mut changed = spans;
        changed[1] = vec![span(4..5)];
        lines.set_highlights(changed);
        assert!(lines.get_layout(0).is_some(), "line 0 unchanged: kept");
        assert!(lines.get_layout(1).is_none(), "line 1 recolored: rebuilt");
    }
}
