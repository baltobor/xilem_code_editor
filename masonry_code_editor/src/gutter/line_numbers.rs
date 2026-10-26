//! Line-numbers gutter control.

use masonry::core::{BrushIndex, render_text};
use masonry::kurbo::Affine;
use masonry::parley;
use masonry::peniko::Brush;
use masonry::peniko::Color;

use super::{GutterControl, GutterPaintContext};

pub struct LineNumbersControl {
    pub line_num_color: Brush,
    pub line_count: usize,
}

impl LineNumbersControl {
    /// Build a fresh line-numbers control with the given foreground
    /// colour and total line count.
    pub fn new(color: Color, line_count: usize) -> Self {
        Self {
            line_num_color: Brush::Solid(color),
            line_count,
        }
    }

    /// Update the total line count after a document edit so the
    /// painter can compute the right gutter width (digits).
    pub fn set_line_count(&mut self, count: usize) {
        self.line_count = count;
    }

    /// Re-theme the line-number colour, e.g. after a light/dark mode
    /// switch.
    pub fn set_color(&mut self, color: Color) {
        self.line_num_color = Brush::Solid(color);
    }
}

impl GutterControl for LineNumbersControl {
    fn id(&self) -> &'static str {
        "line_numbers"
    }

    fn paint(&self, gctx: &mut GutterPaintContext<'_, '_, '_>, _brushes: &[Brush]) {
        if gctx.screen.is_empty() {
            return;
        }

        let digits = if self.line_count >= 10000 {
            5
        } else if self.line_count >= 1000 {
            4
        } else if self.line_count >= 100 {
            3
        } else {
            2
        };

        let left_pad = 8.0;
        let right_pad = 12.0;
        let align_width = (gctx.gutter_width - left_pad - right_pad).max(0.0);

        // With internal scrolling and viewport-relative y coords from ScreenLines,
        // gutter is fixed at x=0 (no scroll_x offset needed).
        // sl.y is already in viewport coordinates (0.0 = top of visible area).
        for sl in &gctx.screen.lines {
            let num_text = format!("{:>width$}", sl.line + 1, width = digits);

            let (fctx, lctx) = gctx.ctx.text_contexts();
            let mut builder = lctx.ranged_builder(fctx, &num_text, 1.0, true);
            builder.push_default(parley::StyleProperty::FontSize(gctx.font_size));
            // Match the editor body's typeface so line numbers stay visually
            // consistent with the code when the user switches fonts.
            builder.push_default(parley::StyleProperty::from(
                gctx.lines.font_family().families(),
            ));
            builder.push_default(parley::StyleProperty::LineHeight(
                parley::LineHeight::FontSizeRelative(1.2),
            ));
            builder.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
            let mut num_layout = builder.build(&num_text);
            num_layout.break_all_lines(Some(align_width as f32));
            num_layout.align(
                Some(align_width as f32),
                parley::Alignment::End,
                parley::AlignmentOptions::default(),
            );

            // Gutter at fixed x position (left_pad), viewport-relative y
            render_text(
                gctx.painter,
                Affine::translate((left_pad, sl.y)),
                &num_layout,
                &[self.line_num_color.clone()],
                true,
            );
        }
    }

    fn z_order(&self) -> i32 {
        0
    }
}
