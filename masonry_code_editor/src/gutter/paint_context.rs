// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use masonry::core::PaintCtx;
use masonry::imaging::Painter;

use crate::text_layout::{Lines, ScreenLines};

/// Context provided to gutter controls during painting.
pub struct GutterPaintContext<'a, 'b, 'c> {
    pub ctx: &'a mut PaintCtx<'b>,
    pub painter: &'a mut Painter<'c>,
    pub screen: &'a ScreenLines,
    pub lines: &'a Lines,
    pub gutter_width: f64,
    pub scroll_x: f64,
    pub font_size: f32,
    pub height: f64,
}
