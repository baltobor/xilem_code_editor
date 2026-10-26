//! Breakpoints gutter control (the red dot the user clicks to set/unset).

use masonry::kurbo::Circle;
use masonry::peniko::Brush;
use masonry::peniko::Color;

use super::{GutterControl, GutterPaintContext};

pub struct BreakpointsControl {
    pub breakpoints: Vec<usize>,
}

impl BreakpointsControl {
    /// Build an empty breakpoint control. Lines are pushed in via
    /// `set_breakpoints` whenever the user toggles a breakpoint or
    /// the debugger reports a configuration change.
    pub fn new() -> Self {
        Self {
            breakpoints: Vec::new(),
        }
    }

    /// Replace the entire breakpoint list. Lines are 1-based to
    /// match the gutter rendering.
    pub fn set_breakpoints(&mut self, breakpoints: Vec<usize>) {
        self.breakpoints = breakpoints;
    }
}

impl Default for BreakpointsControl {
    fn default() -> Self {
        Self::new()
    }
}

impl GutterControl for BreakpointsControl {
    fn id(&self) -> &'static str {
        "breakpoints"
    }

    fn paint(&self, gctx: &mut GutterPaintContext<'_, '_, '_>, _brushes: &[Brush]) {
        let breakpoint_color = Color::from_rgb8(255, 80, 80);
        let radius = 5.0;
        // Fixed x position in gutter (no scroll_x offset)
        let x = 10.0;

        for sl in &gctx.screen.lines {
            let line_1based = sl.line + 1;
            if self.breakpoints.contains(&line_1based) {
                let line_height = gctx
                    .lines
                    .get_layout(sl.line)
                    .map(|l| l.line_height)
                    .unwrap_or(gctx.lines.line_height());
                // Center the dot vertically with the *text* glyphs, not
                // the geometric box center. Parley's 1.2 line-height
                // pads the line above the baseline more than below it,
                // so the visual digit center sits at ~0.42 × line_height
                // from the top, not 0.5. Without this nudge the dot
                // appeared a few pixels below the line number text.
                let y = sl.y + line_height * 0.42;

                gctx.painter
                    .fill(Circle::new((x, y), radius), breakpoint_color)
                    .draw();
            }
        }
    }

    fn on_click(&mut self, line: usize, _x: f64, _y: f64) -> bool {
        // Toggle breakpoint
        if let Some(pos) = self.breakpoints.iter().position(|&l| l == line) {
            self.breakpoints.remove(pos);
        } else {
            self.breakpoints.push(line);
        }
        true
    }

    fn z_order(&self) -> i32 {
        20
    }
}
