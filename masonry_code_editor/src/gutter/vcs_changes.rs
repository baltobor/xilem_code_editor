//! VCS change-markers gutter control.
//!
//! Paints, when enabled, the live git diff of the file against HEAD:
//! - a green vertical bar at the far-left gutter edge on added/changed lines;
//! - a thin beige horizontal marker at each boundary where lines were removed,
//!   drawn thicker when hovered so it reads as a clickable target.
//!
//! This control only *paints*. The owning widget holds the diff and drives
//! hover/click hit-testing (it needs the removed text for the balloon), pushing
//! the current state in each frame via `set_diff` / `set_hovered` / `set_enabled`.

use std::collections::HashSet;

use masonry::kurbo::Rect;
use masonry::peniko::Brush;
use masonry::peniko::Color;

use super::{GutterControl, GutterPaintContext};
use code_editor_core::vcs::VcsGutterDiff;

const BAR_X: f64 = 0.0;
const BAR_WIDTH: f64 = 3.0;
const DEL_THIN: f64 = 1.0;
const DEL_THICK: f64 = 4.0;

pub struct VcsChangesControl {
    enabled: bool,
    /// 1-based new-side lines with an added/changed marker (green bar).
    changed_lines: HashSet<usize>,
    /// `after_new_line` anchors of removed-line markers (beige).
    deletions: Vec<usize>,
    /// `after_new_line` of the currently hovered deletion marker, if any.
    hovered_deletion: Option<usize>,
}

impl VcsChangesControl {
    pub fn new() -> Self {
        Self {
            enabled: false,
            changed_lines: HashSet::new(),
            deletions: Vec::new(),
            hovered_deletion: None,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_diff(&mut self, diff: &VcsGutterDiff) {
        self.changed_lines = diff.line_marks.iter().map(|(n, _)| *n).collect();
        self.deletions = diff.deletions.iter().map(|d| d.after_new_line).collect();
    }

    pub fn clear(&mut self) {
        self.changed_lines.clear();
        self.deletions.clear();
        self.hovered_deletion = None;
    }

    pub fn set_hovered(&mut self, after_new_line: Option<usize>) {
        self.hovered_deletion = after_new_line;
    }
}

impl Default for VcsChangesControl {
    fn default() -> Self {
        Self::new()
    }
}

impl GutterControl for VcsChangesControl {
    fn id(&self) -> &'static str {
        "vcs_changes"
    }

    fn paint(&self, gctx: &mut GutterPaintContext<'_, '_, '_>, _brushes: &[Brush]) {
        if !self.enabled {
            return;
        }
        let green = Color::from_rgb8(80, 170, 90);
        let beige = Color::from_rgb8(214, 188, 140);

        // Green change bars (far-left edge, full line height).
        for sl in &gctx.screen.lines {
            if self.changed_lines.contains(&(sl.line + 1)) {
                let h = gctx
                    .lines
                    .get_layout(sl.line)
                    .map(|l| l.line_height)
                    .unwrap_or(gctx.lines.line_height());
                gctx.painter
                    .fill(Rect::new(BAR_X, sl.y, BAR_X + BAR_WIDTH, sl.y + h), green)
                    .draw();
            }
        }

        // Beige deletion markers: a thin horizontal line at the boundary
        // following `after_new_line` (0-based line index == after_new_line),
        // spanning the gutter width; thicker when hovered.
        for &after in &self.deletions {
            let boundary_y = gctx
                .screen
                .lines
                .iter()
                .find(|sl| sl.line == after)
                .map(|sl| sl.y)
                .or_else(|| {
                    // End-of-file deletion: no line below it on screen — anchor
                    // at the bottom of the preceding line instead.
                    gctx.screen
                        .lines
                        .iter()
                        .find(|sl| sl.line + 1 == after)
                        .map(|sl| {
                            let h = gctx
                                .lines
                                .get_layout(sl.line)
                                .map(|l| l.line_height)
                                .unwrap_or(gctx.lines.line_height());
                            sl.y + h
                        })
                });
            if let Some(y) = boundary_y {
                let thick = if self.hovered_deletion == Some(after) {
                    DEL_THICK
                } else {
                    DEL_THIN
                };
                let top = y - thick / 2.0;
                gctx.painter
                    .fill(Rect::new(BAR_X, top, gctx.gutter_width, top + thick), beige)
                    .draw();
            }
        }
    }

    // Painting only — hover/click is driven by the widget (see pointer_impl),
    // which owns the diff and the removed text needed for the balloon.

    fn z_order(&self) -> i32 {
        // Above line numbers (0), below diagnostics (10) / breakpoints (20) so
        // the breakpoint dot and diagnostic bar stay on top.
        5
    }
}
