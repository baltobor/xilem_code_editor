//! ScreenLines — a snapshot of which lines are currently visible in the viewport.
//!
//! Computed once per frame from the scroll position and viewport height.
//! The widget uses this to decide:
//! - Which lines to ensure have layouts computed
//! - What to paint (only visible lines)
//! - Where to paint each line (y position relative to viewport)

use masonry::core::BrushIndex;
use masonry::parley::{FontContext, LayoutContext};

use crate::properties::SyntaxPalette;
use crate::text_layout::Lines;
use code_editor_core::document::Document;
use code_editor_core::layout::RVLine;

/// A single visible line in the viewport.
#[allow(dead_code)]
pub struct ScreenLine {
    /// Which buffer line (and wrap segment) this represents.
    pub rvline: RVLine,
    /// Y position relative to the viewport top (pixels).
    /// 0.0 = top of the visible area.
    pub y: f64,
    /// Buffer line number (convenience — same as rvline.line).
    pub line: usize,
}

/// A snapshot of all lines visible in the current viewport.
///
/// Created fresh each frame during `layout()`. The `paint()` method
/// iterates over these lines to render only what's visible.
#[allow(dead_code)]
pub struct ScreenLines {
    /// The visible lines, in order from top to bottom.
    pub lines: Vec<ScreenLine>,
    /// The first visible buffer line number.
    pub first_line: usize,
    /// The last visible buffer line number.
    pub last_line: usize,
    /// Pixel offset of the first visible line's top edge.
    /// Negative when the first line is partially scrolled off the top.
    pub y_offset: f64,
}

impl ScreenLines {
    /// Compute which lines are visible for the current scroll position and viewport.
    ///
    /// This is the core of viewport virtualization. It:
    /// 1. Finds which buffer line is at the scroll position
    /// 2. Ensures layouts are computed for visible lines (and only those)
    /// 3. Collects them into a list with y positions
    ///
    /// # Arguments
    /// - `lines`: The layout manager (will compute layouts on demand)
    /// - `document`: The text document (for line content)
    /// - `font_ctx` / `layout_ctx`: Parley text contexts
    /// - `scroll_y`: Current vertical scroll position in pixels
    /// - `viewport_height`: Height of the visible area in pixels
    /// - `max_width`: Maximum line width for word wrapping (None = no wrap)
    pub fn compute(
        lines: &mut Lines,
        document: &dyn Document,
        font_ctx: &mut FontContext,
        layout_ctx: &mut LayoutContext<BrushIndex>,
        syntax_palette: &SyntaxPalette,
        scroll_y: f64,
        viewport_height: f64,
        max_width: Option<f32>,
    ) -> Self {
        let line_count = document.line_count();
        if line_count == 0 {
            return Self {
                lines: vec![],
                first_line: 0,
                last_line: 0,
                y_offset: 0.0,
            };
        }

        // Ensure the layout cache matches the document size
        lines.resize(line_count);

        // Find the first visible line by walking down from the top.
        let first_line = lines.line_at_y(scroll_y);

        // Pre-compute a buffer zone above the viewport so that lines just
        // scrolled into view already have correct heights. This avoids the
        // "empty area" artifact where estimated heights differ from actual.
        // Use a larger buffer for backward scrolling (50 lines above, 30 below).
        let buffer_lines_above = 50;
        let buffer_lines_below = 30;
        let prefetch_start = first_line.saturating_sub(buffer_lines_above);
        for pre in prefetch_start..first_line {
            lines.ensure_layout(
                pre,
                document,
                font_ctx,
                layout_ctx,
                syntax_palette,
                max_width,
            );
        }

        // Recompute first_line_y after prefetching — actual heights may differ
        // from estimates, shifting the y position of the first visible line.
        // Start a few lines earlier to handle boundary cases during fast backward scroll.
        let actual_first = first_line.saturating_sub(3);
        let first_line_y = lines.y_of_line(actual_first);
        let y_offset = first_line_y - scroll_y;

        let mut visible = Vec::with_capacity(64); // typical screen fits ~50 lines
        // =========================================================================
        // VIEWPORT-RELATIVE Y POSITIONS
        // =========================================================================
        // We store y positions relative to the viewport top (0.0 = top of visible area).
        // This is used for internal scroll handling where we don't have Portal's
        // compose() transform. The widget paints at these viewport coordinates directly.
        //
        // Document y: first_line_y (where line starts in document space)
        // Viewport y: first_line_y - scroll_y (where line appears on screen)
        let mut doc_y = first_line_y;
        let mut buf_line = actual_first;
        let end_y = scroll_y + viewport_height;

        while doc_y < end_y && buf_line < line_count {
            // Ensure this line has a layout computed
            lines.ensure_layout(
                buf_line,
                document,
                font_ctx,
                layout_ctx,
                syntax_palette,
                max_width,
            );

            let line_height = lines
                .get_layout(buf_line)
                .map(|l| l.line_height)
                .unwrap_or(lines.line_height());

            // Convert document y to viewport y
            let viewport_y = doc_y - scroll_y;

            visible.push(ScreenLine {
                rvline: RVLine::new(buf_line),
                y: viewport_y,
                line: buf_line,
            });

            doc_y += line_height;
            buf_line += 1;
        }

        // Also prefetch a buffer zone below the viewport
        let post_end = (buf_line + buffer_lines_below).min(line_count);
        for post in buf_line..post_end {
            lines.ensure_layout(
                post,
                document,
                font_ctx,
                layout_ctx,
                syntax_palette,
                max_width,
            );
        }

        let last_line = if buf_line > 0 { buf_line - 1 } else { 0 };

        Self {
            lines: visible,
            first_line,
            last_line,
            y_offset,
        }
    }

    /// Returns true if no lines are visible.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Create an empty ScreenLines (no visible lines).
    pub fn empty() -> Self {
        Self {
            lines: vec![],
            first_line: 0,
            last_line: 0,
            y_offset: 0.0,
        }
    }
}
