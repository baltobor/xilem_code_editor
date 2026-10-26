//! Diagnostic-markers gutter control (the ✕ / ⚠ icons in the gutter).

use masonry::kurbo::{Rect, RoundedRect};
use masonry::peniko::Brush;
use masonry::peniko::Color;

use code_editor_core::model::DiagnosticSeverity;

use super::{GutterControl, GutterPaintContext};

pub struct DiagnosticSpan {
    /// Start line (0-based, inclusive).
    pub start_line: usize,
    /// End line (0-based, inclusive).
    pub end_line: usize,
    /// Severity level.
    pub severity: DiagnosticSeverity,
}

impl DiagnosticSpan {
    /// Create a single-line span.
    pub fn single(line: usize, severity: DiagnosticSeverity) -> Self {
        Self {
            start_line: line,
            end_line: line,
            severity,
        }
    }

    /// Create a multi-line span.
    pub fn range(start_line: usize, end_line: usize, severity: DiagnosticSeverity) -> Self {
        Self {
            start_line,
            end_line,
            severity,
        }
    }
}

/// Built-in: Diagnostic markers gutter control.
///
/// Displays error/warning bars in the gutter. Each span is rendered as one
/// continuous rounded bar, allowing multi-line diagnostics to be visually grouped.
pub struct DiagnosticMarkersControl {
    pub spans: Vec<DiagnosticSpan>,
}

impl DiagnosticMarkersControl {
    /// Build an empty diagnostic-markers control. Spans are pushed
    /// in via `set_spans` / `set_markers` whenever the LSP publishes
    /// a fresh diagnostic batch.
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Set diagnostic spans directly.
    pub fn set_spans(&mut self, spans: Vec<DiagnosticSpan>) {
        self.spans = spans;
    }

    /// Set markers from legacy (line, severity) format - each becomes a single-line span.
    pub fn set_markers(&mut self, markers: Vec<(usize, DiagnosticSeverity)>) {
        self.spans = markers
            .into_iter()
            .map(|(line, severity)| DiagnosticSpan::single(line, severity))
            .collect();
    }
}

impl Default for DiagnosticMarkersControl {
    fn default() -> Self {
        Self::new()
    }
}

impl GutterControl for DiagnosticMarkersControl {
    fn id(&self) -> &'static str {
        "diagnostic_markers"
    }

    fn paint(&self, gctx: &mut GutterPaintContext<'_, '_, '_>, _brushes: &[Brush]) {
        let bar_width = 3.0;
        let corner_radius = bar_width / 2.0; // Fully rounded ends
        // Fixed x position at right edge of gutter (no scroll_x offset)
        let x = gctx.gutter_width - bar_width - 2.0;

        for span in &self.spans {
            let color = match span.severity {
                DiagnosticSeverity::Error => Color::from_rgb8(255, 80, 80),
                DiagnosticSeverity::Warning => Color::from_rgb8(255, 165, 0),
                _ => continue,
            };

            // Find viewport y positions for the span's start and end lines
            let mut y_start: Option<f64> = None;
            let mut y_end: Option<f64> = None;

            for sl in &gctx.screen.lines {
                if sl.line >= span.start_line && sl.line <= span.end_line {
                    let line_height = gctx
                        .lines
                        .get_layout(sl.line)
                        .map(|l| l.line_height)
                        .unwrap_or(gctx.lines.line_height());

                    if y_start.is_none() {
                        y_start = Some(sl.y);
                    }
                    // Keep updating y_end to get the bottom of the last visible line in span
                    y_end = Some(sl.y + line_height);
                }
            }

            // Only paint if at least part of the span is visible
            if let (Some(top), Some(bottom)) = (y_start, y_end) {
                let rect = Rect::new(x, top, x + bar_width, bottom);
                let rounded = RoundedRect::from_rect(rect, corner_radius);
                gctx.painter.fill(rounded, color).draw();
            }
        }
    }

    fn z_order(&self) -> i32 {
        10
    }
}
