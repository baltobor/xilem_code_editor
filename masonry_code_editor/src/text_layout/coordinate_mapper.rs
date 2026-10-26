//! CoordinateMapper — stateless coordinate conversion utilities.
//!
//! Extracted from lines.rs to follow Single Responsibility Principle.
//! Pure functions for converting between different coordinate systems:
//! - Buffer lines (0-indexed line numbers in the document)
//! - Visual lines (VLine - counting wrapped lines)
//! - Relative visual lines (RVLine - line + sub-line index)
//! - Y positions (pixel offset from top)
//! - X positions (pixel offset from gutter)

use crate::text_layout::LayoutCache;
use code_editor_core::layout::{RVLine, VLine};

/// Convert a visual line (VLine) to a relative visual line (RVLine).
///
/// A VLine is the absolute visual line number counting all wrapped lines.
/// An RVLine is a (buffer_line, sub_line_index) pair.
pub fn vline_to_rvline(vline: VLine, cache: &LayoutCache) -> RVLine {
    let mut remaining = vline.0;

    for line_idx in 0..cache.line_count() {
        let visual_count = cache
            .get(line_idx)
            .map(|layout| layout.visual_line_count)
            .unwrap_or(1);

        if remaining < visual_count {
            return RVLine::with_index(line_idx, remaining);
        }
        remaining -= visual_count;
    }

    // Past the end — return last line
    let last = cache.line_count().saturating_sub(1);
    RVLine::new(last)
}

/// Convert a relative visual line (RVLine) to an absolute visual line (VLine).
pub fn rvline_to_vline(rvline: RVLine, cache: &LayoutCache) -> VLine {
    let mut vline = 0;

    for line_idx in 0..cache.line_count() {
        if line_idx == rvline.line {
            return VLine(vline + rvline.line_index);
        }

        let visual_count = cache
            .get(line_idx)
            .map(|layout| layout.visual_line_count)
            .unwrap_or(1);
        vline += visual_count;
    }

    VLine(vline)
}

/// Compute the y pixel position of a given buffer line.
///
/// Sums the heights of all lines before the given line.
pub fn y_of_line(line: usize, cache: &LayoutCache) -> f64 {
    let mut y = 0.0;
    for i in 0..line.min(cache.line_count()) {
        y += cache.height_of(i);
    }
    y
}

/// Find which buffer line is at a given y pixel position.
pub fn line_at_y(y: f64, cache: &LayoutCache) -> usize {
    let mut current_y = 0.0;

    for line in 0..cache.line_count() {
        let h = cache.height_of(line);
        if current_y + h > y {
            return line;
        }
        current_y += h;
    }

    cache.line_count().saturating_sub(1)
}

/// Compute the y pixel position of a visual line.
///
/// Similar to y_of_line but accounts for line wrapping.
pub fn y_of_vline(vline: VLine, cache: &LayoutCache, default_line_height: f64) -> f64 {
    let rvline = vline_to_rvline(vline, cache);

    // Sum heights up to the buffer line
    let base_y = y_of_line(rvline.line, cache);

    // Add sub-line offset within wrapped line
    if rvline.line_index > 0 {
        if let Some(layout) = cache.get(rvline.line) {
            // For wrapped lines, each sub-line has equal height
            let sub_line_height = layout.line_height / layout.visual_line_count as f64;
            return base_y + (rvline.line_index as f64 * sub_line_height);
        }
    }

    base_y + (rvline.line_index as f64 * default_line_height)
}

/// Find which visual line is at a given y pixel position.
pub fn vline_at_y(y: f64, cache: &LayoutCache) -> VLine {
    let line = line_at_y(y, cache);
    let line_y = y_of_line(line, cache);
    let offset_in_line = y - line_y;

    if let Some(layout) = cache.get(line) {
        if layout.visual_line_count > 1 {
            // Find which sub-line within this wrapped line
            let sub_line_height = layout.line_height / layout.visual_line_count as f64;
            let sub_line_idx = (offset_in_line / sub_line_height).floor() as usize;
            let sub_line_idx = sub_line_idx.min(layout.visual_line_count - 1);

            let rvline = RVLine::with_index(line, sub_line_idx);
            return rvline_to_vline(rvline, cache);
        }
    }

    // Convert buffer line to vline (summing visual line counts)
    let mut vline = 0;
    for i in 0..line {
        vline += cache.get(i).map(|l| l.visual_line_count).unwrap_or(1);
    }
    VLine(vline)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_cache(lines: usize, height: f64) -> LayoutCache {
        LayoutCache::new(lines, height)
    }

    #[test]
    fn test_y_of_line() {
        let cache = make_test_cache(10, 20.0);

        assert_eq!(y_of_line(0, &cache), 0.0);
        assert_eq!(y_of_line(1, &cache), 20.0);
        assert_eq!(y_of_line(5, &cache), 100.0);
        assert_eq!(y_of_line(10, &cache), 200.0); // Beyond end
    }

    #[test]
    fn test_line_at_y() {
        let cache = make_test_cache(10, 20.0);

        assert_eq!(line_at_y(0.0, &cache), 0);
        assert_eq!(line_at_y(10.0, &cache), 0);
        assert_eq!(line_at_y(20.0, &cache), 1);
        assert_eq!(line_at_y(25.0, &cache), 1);
        assert_eq!(line_at_y(100.0, &cache), 5);
        assert_eq!(line_at_y(1000.0, &cache), 9); // Clamps to last line
    }

    #[test]
    fn test_vline_to_rvline_no_wrapping() {
        let cache = make_test_cache(5, 20.0);

        // Without any cached layouts (all visual_count = 1)
        assert_eq!(vline_to_rvline(VLine(0), &cache), RVLine::new(0));
        assert_eq!(vline_to_rvline(VLine(1), &cache), RVLine::new(1));
        assert_eq!(vline_to_rvline(VLine(4), &cache), RVLine::new(4));
        assert_eq!(vline_to_rvline(VLine(10), &cache), RVLine::new(4)); // Past end
    }

    #[test]
    fn test_rvline_to_vline_no_wrapping() {
        let cache = make_test_cache(5, 20.0);

        assert_eq!(rvline_to_vline(RVLine::new(0), &cache), VLine(0));
        assert_eq!(rvline_to_vline(RVLine::new(3), &cache), VLine(3));
    }
}
