//! Execution-line gutter control (the ▶ arrow shown on the line the
//! debugger is currently paused on).

use masonry::peniko::Brush;
use masonry::peniko::Color;

use super::{GutterControl, GutterPaintContext};

pub struct ExecutionLineControl {
    pub current_line: Option<usize>,
}

impl ExecutionLineControl {
    /// Build a control with no execution marker drawn yet.
    pub fn new() -> Self {
        Self { current_line: None }
    }

    /// Update the line the debugger is currently paused on (1-based)
    /// or clear it with `None` when execution resumes.
    pub fn set_line(&mut self, line: Option<usize>) {
        self.current_line = line;
    }
}

impl Default for ExecutionLineControl {
    fn default() -> Self {
        Self::new()
    }
}

impl GutterControl for ExecutionLineControl {
    fn id(&self) -> &'static str {
        "execution_line"
    }

    fn paint(&self, gctx: &mut GutterPaintContext<'_, '_, '_>, _brushes: &[Brush]) {
        use masonry::kurbo::BezPath;
        let Some(target_line) = self.current_line else {
            return;
        };
        // Saturated orange — distinct from both the red breakpoint dot
        // and the yellowish background tones, so the arrow is visible
        // when sitting next to a breakpoint.
        let arrow_color = Color::from_rgb8(255, 140, 0);
        // Equilateral right-pointing triangle: width = height. With a
        // half-height of 5px the equilateral side length is ~5.77px,
        // giving a horizontal extent of about 5px from base to tip.
        let half_h = 5.0;
        let triangle_w = (3.0_f64).sqrt() * half_h; // equilateral
        // Center the triangle horizontally in the gutter's right
        // padding strip — the 12px gap between the right edge of the
        // line-number digits (at gutter_width - 12) and the gutter's
        // right edge. With a triangle width of ~8.66px the remaining
        // 3.34px split evenly into ~1.67px on each side.
        let line_num_right_pad = 12.0;
        let pad_side = (line_num_right_pad - triangle_w) / 2.0;
        let tip_x = gctx.gutter_width - pad_side;
        let base_x = tip_x - triangle_w;
        for sl in &gctx.screen.lines {
            if sl.line + 1 != target_line {
                continue;
            }
            let line_height = gctx
                .lines
                .get_layout(sl.line)
                .map(|l| l.line_height)
                .unwrap_or(gctx.lines.line_height());
            // Match the breakpoint dot's vertical centering — the visual
            // center of the line digits sits above the geometric center
            // of the line box because parley's 1.2 line-height adds more
            // leading above the baseline than below.
            let cy = sl.y + line_height * 0.42;
            let mut path = BezPath::new();
            path.move_to((base_x, cy - half_h));
            path.line_to((tip_x, cy));
            path.line_to((base_x, cy + half_h));
            path.close_path();
            gctx.painter.fill(path, arrow_color).draw();
        }
    }

    fn z_order(&self) -> i32 {
        25
    }
}
