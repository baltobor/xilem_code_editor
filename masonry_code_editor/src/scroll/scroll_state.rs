//! Scroll/viewport geometry state for `CodeEditorWidget`: the current
//! scroll position, the last-laid-out viewport and content sizes,
//! and an in-progress scrollbar drag.
//!
//! Extracted from five loose fields (`viewport_pos`/`viewport_size`/
//! `content_size`/`scrollbar_drag`/`last_emitted_scroll_y`) as part
//! of the CodeEditorWidget SOLID rework. The editor handles its own
//! scrolling instead of being wrapped in `Portal`, so this type also
//! owns the geometry that used to live as free-standing methods in
//! `widget/viewport_scroll.rs`: position clamping, wheel-scroll
//! panning, and scrollbar hit-testing/dragging all only ever touched
//! these five fields, so they move here wholesale rather than staying
//! as external functions poking at public fields.
//!
//! `scroll_to_show_rect` (goto-line / search / cursor-follow) stays
//! on `CodeEditorWidget` — it also depends on `pending_scroll_alignment`
//! and `is_search_highlight`, which belong to a different cluster.

use masonry::core::EventCtx;
use masonry::kurbo::{Point, Size, Vec2};

/// Active scrollbar drag, recording which scrollbar is being dragged and
/// the offset of the original click within the thumb (so the thumb tracks
/// the cursor without snapping its top to the pointer).
#[derive(Debug, Clone, Copy)]
enum ScrollbarDrag {
    Vertical { thumb_offset_y: f64 },
    Horizontal { thumb_offset_x: f64 },
}

#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    pos: Point,
    viewport_size: Size,
    content_size: Size,
    drag: Option<ScrollbarDrag>,
    /// Last vertical scroll position emitted as a `ScrollChanged`
    /// action. See `take_scroll_change`.
    last_emitted_y: f64,
}

impl ScrollState {
    pub fn pos(&self) -> Point {
        self.pos
    }

    pub fn viewport_size(&self) -> Size {
        self.viewport_size
    }

    pub fn content_size(&self) -> Size {
        self.content_size
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub fn take_drag(&mut self) -> bool {
        self.drag.take().is_some()
    }

    /// Store the viewport size from a fresh layout pass.
    pub fn set_viewport_size(&mut self, size: Size) {
        self.viewport_size = size;
    }

    /// Store the content size from a fresh layout pass.
    pub fn set_content_size(&mut self, size: Size) {
        self.content_size = size;
    }

    /// Set the viewport scroll position (clamped to valid range).
    /// Returns true if position changed.
    pub fn set_viewport_pos(&mut self, pos: Point) -> bool {
        let max_scroll = (self.content_size - self.viewport_size).max(Size::ZERO);
        let clamped = Point::new(
            pos.x.clamp(0.0, max_scroll.width),
            pos.y.clamp(0.0, max_scroll.height),
        );
        if (clamped.x - self.pos.x).abs() > 0.5 || (clamped.y - self.pos.y).abs() > 0.5 {
            self.pos = clamped;
            true
        } else {
            false
        }
    }

    /// Pan the viewport by a delta (used for scroll events).
    /// Returns true if position changed.
    pub fn pan_by(&mut self, delta: Vec2) -> bool {
        self.set_viewport_pos(self.pos + delta)
    }

    /// Scroll the viewport so the last line is visible at the bottom
    /// edge while leaving horizontal scroll untouched. Returns `true`
    /// if the viewport actually moved.
    ///
    /// This exists so the compiler-output follow-tail logic can keep
    /// autoscrolling on new output without dragging the user back to
    /// column 1 every frame (which is what `perform_goto_line` does
    /// as a side effect of moving the caret).
    pub fn scroll_to_bottom_keep_horizontal(&mut self) -> bool {
        if self.viewport_size.height < 1.0 {
            // Layout hasn't run yet. Caller will retry next rebuild.
            return false;
        }
        let target_y = (self.content_size.height - self.viewport_size.height).max(0.0);
        let new_pos = Point::new(self.pos.x, target_y);
        self.set_viewport_pos(new_pos)
    }

    /// Hit-test the scrollbars and start a drag or page-jump on a primary
    /// `PointerEvent::Down`. Returns `true` if the event was consumed by the
    /// scrollbar (caller should `return` and skip text-selection handling).
    pub fn handle_scrollbar_down(&mut self, ctx: &mut EventCtx<'_>, pos: Point) -> bool {
        let viewport = (self.viewport_size.width, self.viewport_size.height);

        if let Some(layout) =
            crate::paint::vertical_scrollbar_layout(viewport, self.content_size.height, self.pos.y)
        {
            if layout.thumb_rect.contains(pos) {
                self.drag = Some(ScrollbarDrag::Vertical {
                    thumb_offset_y: pos.y - layout.thumb_rect.y0,
                });
                ctx.capture_pointer();
                ctx.set_handled();
                return true;
            }
            if layout.track_rect.contains(pos) {
                let direction = if pos.y < layout.thumb_rect.y0 {
                    -1.0
                } else {
                    1.0
                };
                let delta = Vec2::new(0.0, direction * self.viewport_size.height);
                if self.pan_by(delta) {
                    ctx.request_render();
                }
                ctx.set_handled();
                return true;
            }
        }

        if let Some(layout) =
            crate::paint::horizontal_scrollbar_layout(viewport, self.content_size.width, self.pos.x)
        {
            if layout.thumb_rect.contains(pos) {
                self.drag = Some(ScrollbarDrag::Horizontal {
                    thumb_offset_x: pos.x - layout.thumb_rect.x0,
                });
                ctx.capture_pointer();
                ctx.set_handled();
                return true;
            }
            if layout.track_rect.contains(pos) {
                let direction = if pos.x < layout.thumb_rect.x0 {
                    -1.0
                } else {
                    1.0
                };
                let delta = Vec2::new(direction * self.viewport_size.width, 0.0);
                if self.pan_by(delta) {
                    ctx.request_render();
                }
                ctx.set_handled();
                return true;
            }
        }

        false
    }

    /// Update the scroll position while a scrollbar drag is active. Inverts
    /// the thumb-position formula in `crate::paint::*_scrollbar_layout` so the
    /// thumb tracks the cursor with the original click offset preserved.
    pub fn handle_scrollbar_drag(&mut self, ctx: &mut EventCtx<'_>, pos: Point) {
        let Some(drag) = self.drag else { return };
        let viewport = (self.viewport_size.width, self.viewport_size.height);

        let new_pos = match drag {
            ScrollbarDrag::Vertical { thumb_offset_y } => {
                let Some(layout) = crate::paint::vertical_scrollbar_layout(
                    viewport,
                    self.content_size.height,
                    self.pos.y,
                ) else {
                    return;
                };
                let track_remaining = layout.track_extent - layout.thumb_extent;
                if track_remaining <= 0.0 {
                    return;
                }
                let thumb_top = pos.y - thumb_offset_y;
                let scroll_ratio = ((thumb_top - crate::paint::SCROLLBAR_MARGIN) / track_remaining)
                    .clamp(0.0, 1.0);
                let max_scroll_y = (self.content_size.height - self.viewport_size.height).max(0.0);
                Point::new(self.pos.x, scroll_ratio * max_scroll_y)
            }
            ScrollbarDrag::Horizontal { thumb_offset_x } => {
                let Some(layout) = crate::paint::horizontal_scrollbar_layout(
                    viewport,
                    self.content_size.width,
                    self.pos.x,
                ) else {
                    return;
                };
                let track_remaining = layout.track_extent - layout.thumb_extent;
                if track_remaining <= 0.0 {
                    return;
                }
                let thumb_left = pos.x - thumb_offset_x;
                let scroll_ratio = ((thumb_left - crate::paint::SCROLLBAR_MARGIN)
                    / track_remaining)
                    .clamp(0.0, 1.0);
                let max_scroll_x = (self.content_size.width - self.viewport_size.width).max(0.0);
                Point::new(scroll_ratio * max_scroll_x, self.pos.y)
            }
        };

        if self.set_viewport_pos(new_pos) {
            ctx.request_render();
        }
        ctx.set_handled();
    }

    /// Coalesced scroll-change detection for the `ScrollChanged` action:
    /// listeners (e.g. compile-output autoscroll) only care about
    /// movements over 0.5px. Returns the new Y position — and updates
    /// the internal high-water mark — only when the threshold is
    /// crossed; `None` otherwise.
    pub fn take_scroll_change(&mut self) -> Option<f64> {
        if (self.pos.y - self.last_emitted_y).abs() > 0.5 {
            self.last_emitted_y = self.pos.y;
            Some(self.pos.y)
        } else {
            None
        }
    }
}
