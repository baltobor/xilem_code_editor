//! Scrollbar geometry + paint — vertical and horizontal scrollbars
//! with the same coordinate system the pointer hit-testing uses.

use masonry::imaging::Painter;
use masonry::kurbo::{Rect, RoundedRect};
use masonry::peniko::Color;

/// Width (cross-axis) of the scrollbar track and thumb.
pub const SCROLLBAR_WIDTH: f64 = 8.0;
/// Margin between the scrollbar and the viewport edges (and between the two scrollbars).
pub const SCROLLBAR_MARGIN: f64 = 2.0;
/// Minimum thumb length so very long content still produces a draggable thumb.
pub const SCROLLBAR_MIN_THUMB: f64 = 20.0;

/// Geometry for one scrollbar (vertical or horizontal).
///
/// Returned by [`vertical_scrollbar_layout`] / [`horizontal_scrollbar_layout`] when
/// the content overflows the viewport along the relevant axis. Used by both painting
/// and pointer hit-testing so the two stay in sync.
// MARK: Scrollbars — layout + paint

pub struct ScrollbarLayout {
    pub track_rect: Rect,
    pub thumb_rect: Rect,
    /// Track length along the scroll axis (height for vertical, width for horizontal).
    pub track_extent: f64,
    /// Thumb length along the scroll axis.
    pub thumb_extent: f64,
}

/// Compute the vertical scrollbar layout, or `None` if content fits in the viewport.
pub fn vertical_scrollbar_layout(
    viewport_size: (f64, f64),
    content_h: f64,
    scroll_y: f64,
) -> Option<ScrollbarLayout> {
    let (viewport_w, viewport_h) = viewport_size;
    if content_h <= viewport_h {
        return None;
    }
    let track_x = viewport_w - SCROLLBAR_WIDTH - SCROLLBAR_MARGIN;
    let track_h = viewport_h - SCROLLBAR_MARGIN * 2.0;
    let track_rect = Rect::new(
        track_x,
        SCROLLBAR_MARGIN,
        track_x + SCROLLBAR_WIDTH,
        SCROLLBAR_MARGIN + track_h,
    );
    let thumb_ratio = viewport_h / content_h;
    let thumb_h = (track_h * thumb_ratio).max(SCROLLBAR_MIN_THUMB);
    let scroll_ratio = scroll_y / (content_h - viewport_h).max(1.0);
    let thumb_y = SCROLLBAR_MARGIN + scroll_ratio * (track_h - thumb_h);
    let thumb_rect = Rect::new(
        track_x,
        thumb_y,
        track_x + SCROLLBAR_WIDTH,
        thumb_y + thumb_h,
    );
    Some(ScrollbarLayout {
        track_rect,
        thumb_rect,
        track_extent: track_h,
        thumb_extent: thumb_h,
    })
}

/// Compute the horizontal scrollbar layout, or `None` if content fits in the viewport.
pub fn horizontal_scrollbar_layout(
    viewport_size: (f64, f64),
    content_w: f64,
    scroll_x: f64,
) -> Option<ScrollbarLayout> {
    let (viewport_w, viewport_h) = viewport_size;
    if content_w <= viewport_w {
        return None;
    }
    let track_y = viewport_h - SCROLLBAR_WIDTH - SCROLLBAR_MARGIN;
    let track_w = viewport_w - SCROLLBAR_MARGIN * 2.0;
    let track_rect = Rect::new(
        SCROLLBAR_MARGIN,
        track_y,
        SCROLLBAR_MARGIN + track_w,
        track_y + SCROLLBAR_WIDTH,
    );
    let thumb_ratio = viewport_w / content_w;
    let thumb_w = (track_w * thumb_ratio).max(SCROLLBAR_MIN_THUMB);
    let scroll_ratio = scroll_x / (content_w - viewport_w).max(1.0);
    let thumb_x = SCROLLBAR_MARGIN + scroll_ratio * (track_w - thumb_w);
    let thumb_rect = Rect::new(
        thumb_x,
        track_y,
        thumb_x + thumb_w,
        track_y + SCROLLBAR_WIDTH,
    );
    Some(ScrollbarLayout {
        track_rect,
        thumb_rect,
        track_extent: track_w,
        thumb_extent: thumb_w,
    })
}

/// Paint scroll bars (vertical and horizontal).
///
/// The scroll bars are painted as rounded rectangles that indicate the current
/// viewport position within the content.
pub fn paint_scrollbars(
    painter: &mut Painter<'_>,
    viewport_size: (f64, f64),
    content_size: (f64, f64),
    scroll_pos: (f64, f64),
    track_color: Color,
    thumb_color: Color,
) {
    let corner_radius = SCROLLBAR_WIDTH / 2.0;

    if let Some(layout) = vertical_scrollbar_layout(viewport_size, content_size.1, scroll_pos.1) {
        painter
            .fill(
                RoundedRect::from_rect(layout.track_rect, corner_radius),
                track_color,
            )
            .draw();
        painter
            .fill(
                RoundedRect::from_rect(layout.thumb_rect, corner_radius),
                thumb_color,
            )
            .draw();
    }

    if let Some(layout) = horizontal_scrollbar_layout(viewport_size, content_size.0, scroll_pos.0) {
        painter
            .fill(
                RoundedRect::from_rect(layout.track_rect, corner_radius),
                track_color,
            )
            .draw();
        painter
            .fill(
                RoundedRect::from_rect(layout.thumb_rect, corner_radius),
                thumb_color,
            )
            .draw();
    }
}
