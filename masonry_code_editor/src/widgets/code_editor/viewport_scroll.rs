//! Scroll-to-rect + cursor-geometry helpers for `CodeEditorWidget`.
//!
//! `scroll_to_show_rect` is the one-shot "make this rect visible"
//! call that goto-line / search / cursor-follow funnel through — it
//! stays here (rather than on `ScrollState`) because it also reads
//! `pending_scroll_alignment` and `is_search_highlight`, which belong
//! to a different field cluster. The lower-level scroll-position
//! machinery (clamping, wheel-scroll panning, scrollbar drag) lives
//! on `ScrollState` itself; see `src/editor/scroll_state.rs`.

use masonry::kurbo::Rect;

use super::CodeEditorWidget;

impl CodeEditorWidget {
    // MARK: Viewport / scrolling

    /// Scroll the internal viewport to ensure the given rect is visible.
    /// Returns true if scroll was applied, false if viewport size isn't valid yet.
    ///
    /// When `is_search_highlight` is true, centers the rect in the viewport
    /// for better visibility of search results. Otherwise scrolls minimally.
    ///
    /// TODO: The editor widget now handles scrolling internally via viewport_pos instead of
    /// using ctx.request_scroll_to(rect). Maybe later we can mimic the ctx.request_scroll_to(rect)
    /// syntax for a cleaner API.
    pub(super) fn scroll_to_show_rect(&mut self, rect: Rect) -> bool {
        let viewport_pos = self.scroll.pos();
        let viewport_size = self.scroll.viewport_size();
        // Skip if viewport size isn't set yet (before first layout)
        if viewport_size.width < 1.0 || viewport_size.height < 1.0 {
            return false;
        }

        let mut new_pos = viewport_pos;
        // Pick the scroll strategy. The `is_search_highlight` flag forces
        // Center even when the alignment was left at Edge (search results
        // always center). External navigation may also override.
        // Marker: Structure Click Appearance: Click shows the function at
        // the top of the text editor — `Top` lands the line right at the
        // viewport's top edge with the body fully visible below.
        use code_editor_core::model::GotoAlignment;
        let mut alignment = self.pending_scroll_alignment;
        if alignment == GotoAlignment::Edge && self.is_search_highlight {
            alignment = GotoAlignment::Center;
        }
        if alignment == GotoAlignment::Center {
            // Center the target in the viewport for better visibility
            let rect_center_y = (rect.y0 + rect.y1) / 2.0;
            let viewport_center_y = viewport_size.height / 2.0;
            new_pos.y = (rect_center_y - viewport_center_y).max(0.0);

            // Also center horizontally if needed
            let rect_center_x = (rect.x0 + rect.x1) / 2.0;
            let viewport_center_x = viewport_size.width / 2.0;
            // Only center horizontally if rect is far outside viewport
            if rect.x0 < viewport_pos.x || rect.x1 > viewport_pos.x + viewport_size.width {
                new_pos.x = (rect_center_x - viewport_center_x).max(0.0);
            }
        } else if alignment == GotoAlignment::Top {
            // Place the rect at the top of the viewport (a few pixels of
            // padding so it doesn't kiss the very top edge).
            let top_padding = 8.0_f64;
            new_pos.y = (rect.y0 - top_padding).max(0.0);
            // Horizontal: only scroll if needed (don't force-recentre).
            if rect.x0 < viewport_pos.x {
                new_pos.x = rect.x0;
            } else if rect.x1 > viewport_pos.x + viewport_size.width {
                new_pos.x = rect.x1 - viewport_size.width;
            }
        } else {
            // Normal scrolling: just make the rect visible
            let viewport = Rect::from_origin_size(viewport_pos, viewport_size);

            // Vertical scrolling
            if rect.y0 < viewport.y0 {
                new_pos.y = rect.y0;
            } else if rect.y1 > viewport.y1 {
                new_pos.y = rect.y1 - viewport_size.height;
            }

            // Horizontal scrolling
            if rect.x0 < viewport.x0 {
                new_pos.x = rect.x0;
            } else if rect.x1 > viewport.x1 {
                new_pos.x = rect.x1 - viewport_size.width;
            }
        }

        self.scroll.set_viewport_pos(new_pos);
        // Consume the one-shot alignment so subsequent typing/arrow scrolls
        // don't keep re-applying the same strategy.
        self.pending_scroll_alignment = code_editor_core::model::GotoAlignment::Edge;
        true
    }

    // MARK: Cursor geometry helpers

    /// Get the cursor position as (line, col) — both 1-based.
    pub(super) fn cursor_position(&self) -> (usize, usize) {
        self.cursor.position(self.document.as_ref())
    }

    /// Compute the primary cursor's bounding rect in widget-local coordinates.
    /// Returns `None` if the cursor line has no layout cached yet.
    pub(super) fn cursor_rect(&self) -> Option<Rect> {
        let offset = self.cursor.offset();
        let line = self.document.line_of_offset(offset);
        let x = self.lines.offset_to_x(self.document.as_ref(), offset) + self.config.gutter_width;
        let y = self.lines.y_of_line(line);
        let line_height = self
            .lines
            .get_layout(line)
            .map(|l| l.line_height)
            .unwrap_or(self.lines.line_height());
        // Include some horizontal padding so the cursor isn't flush against the edge.
        Some(Rect::new(x - 4.0, y, x + 20.0, y + line_height))
    }
}
