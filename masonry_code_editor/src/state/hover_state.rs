// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Pointer-hover popup state: zed's timing (`HoverTimer`) plus the content
//! the host sent and where the popup was painted.

use std::time::Instant;

use code_editor_core::timing::{HoverEffect, HoverTimer, PopupTiming};
use masonry::kurbo::{Point, Rect};

pub struct HoverState {
    timer: HoverTimer,
    /// (anchor offset, markdown) from the host.
    content: Option<(usize, String)>,
    /// Painted popup, for hit-testing; `Rect::ZERO` when not shown.
    rect: Rect,
}

impl Default for HoverState {
    fn default() -> Self {
        Self {
            timer: HoverTimer::new(PopupTiming::default()),
            content: None,
            rect: Rect::ZERO,
        }
    }
}

impl HoverState {
    /// The pointer moved; `anchor` is the word start under it (`None`:
    /// nothing hoverable). Over the popup itself it stays open; elsewhere
    /// moving closer to it extends the grace period.
    pub fn pointer_moved(
        &mut self,
        anchor: Option<usize>,
        pos: Point,
        now: Instant,
    ) -> Option<HoverEffect> {
        if self.rect.contains(pos) {
            self.timer.over_popover();
            return None;
        }
        let effect = self.timer.hover_at(anchor, now);
        if anchor.is_none() && self.rect != Rect::ZERO {
            self.timer.approach(distance(self.rect, pos), now);
        }
        self.apply(effect)
    }

    /// Typing, scrolling, a click, focus loss or Esc: hide immediately.
    /// Returns whether a popup was visible.
    pub fn dismiss(&mut self) -> bool {
        let effect = self.timer.dismiss();
        self.apply(effect).is_some()
    }

    pub fn tick(&mut self, now: Instant) -> Option<HoverEffect> {
        let effect = self.timer.tick(now);
        self.apply(effect)
    }

    /// The host answered for the word at `anchor`.
    pub fn set_content(&mut self, anchor: usize, markdown: Option<String>, now: Instant) {
        self.timer.content_arrived(anchor, markdown.is_some(), now);
        self.content = markdown.map(|text| (anchor, text));
    }

    /// (anchor, markdown) while the popup is visible.
    pub fn visible(&self) -> Option<(usize, &str)> {
        let anchor = self.timer.visible_anchor()?;
        match &self.content {
            Some((a, text)) if *a == anchor => Some((anchor, text.as_str())),
            _ => None,
        }
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.timer.next_deadline()
    }

    pub fn set_painted_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    fn apply(&mut self, effect: Option<HoverEffect>) -> Option<HoverEffect> {
        if effect == Some(HoverEffect::Hide) {
            self.content = None;
            self.rect = Rect::ZERO;
        }
        effect
    }
}

/// Distance from `pos` to the nearest point of `rect` (0 inside).
fn distance(rect: Rect, pos: Point) -> f64 {
    let dx = (rect.x0 - pos.x).max(pos.x - rect.x1).max(0.0);
    let dy = (rect.y0 - pos.y).max(pos.y - rect.y1).max(0.0);
    dx.hypot(dy)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn shows_answer_only_for_the_rested_word() {
        let t0 = Instant::now();
        let mut hover = HoverState::default();
        hover.pointer_moved(Some(10), Point::new(5.0, 5.0), t0);
        assert_eq!(
            hover.tick(t0 + Duration::from_millis(150)),
            Some(HoverEffect::Request(10))
        );
        hover.set_content(10, Some("docs".into()), t0 + Duration::from_millis(200));
        hover.tick(t0 + Duration::from_millis(300));
        assert_eq!(hover.visible(), Some((10, "docs")));
        assert!(hover.dismiss());
        assert_eq!(hover.visible(), None);
    }

    #[test]
    fn distance_to_rect() {
        let rect = Rect::new(10.0, 10.0, 20.0, 20.0);
        assert_eq!(distance(rect, Point::new(15.0, 15.0)), 0.0);
        assert_eq!(distance(rect, Point::new(25.0, 15.0)), 5.0);
    }
}
