//! State for the Quick Help balloon — the context-menu hover
//! documentation popup shown by `CodeEditorWidget`.
//!
//! Extracted from four loose fields on `CodeEditorWidget`
//! (`quick_help_text`/`quick_help_scroll`/`quick_help_rect`/
//! `quick_help_content_height`) as part of the SOLID rework: the
//! widget owned this state, but never needed to reason about it as
//! anything other than "the Quick Help balloon's state" — grouping it
//! here gives that concept a name and a narrow API instead of four
//! fields callers had to keep in sync by hand.

use masonry::kurbo::{Point, Rect};

#[derive(Debug, Clone, Default)]
pub struct QuickHelpState {
    /// Markdown text from the LSP hover response. `None` when the
    /// balloon is hidden.
    text: Option<String>,
    /// Vertical scroll offset (pixels) inside the balloon.
    scroll: f64,
    /// Bounding rect of the last-painted balloon (hit-testing).
    rect: Rect,
    /// Total content height of the last-painted balloon (for scroll
    /// clamping).
    content_height: f64,
}

impl QuickHelpState {
    pub fn is_visible(&self) -> bool {
        self.text.is_some()
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn scroll(&self) -> f64 {
        self.scroll
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn content_height(&self) -> f64 {
        self.content_height
    }

    pub fn contains(&self, pos: Point) -> bool {
        self.rect.contains(pos)
    }

    /// Push new balloon state (text + scroll) from the View layer's
    /// `WidgetMut::set_quick_help`.
    pub fn set(&mut self, text: Option<String>, scroll: f64) {
        self.text = text;
        self.scroll = scroll;
    }

    /// Record the balloon's painted geometry. Called at the end of
    /// each paint pass once the balloon layout is known.
    pub fn set_painted_geometry(&mut self, rect: Rect, content_height: f64) {
        self.rect = rect;
        self.content_height = content_height;
    }

    /// Apply a wheel-scroll delta (already sign-adjusted by the
    /// caller) clamped to the balloon's content height. Returns
    /// `Some(new_scroll)` when the value changed by more than the
    /// existing 0.5px coalescing threshold — the caller uses that to
    /// decide whether to emit `QuickHelpScroll` and request a render
    /// — or `None` when the delta was too small to matter.
    pub fn scroll_by(&mut self, delta_y: f64) -> Option<f64> {
        let balloon_h = self.rect.height();
        let max_scroll = (self.content_height - balloon_h + 20.0).max(0.0);
        let new_scroll = (self.scroll + delta_y).clamp(0.0, max_scroll);
        if (new_scroll - self.scroll).abs() > 0.5 {
            self.scroll = new_scroll;
            Some(new_scroll)
        } else {
            None
        }
    }
}
