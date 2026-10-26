// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::any::Any;

use code_editor_core::document::Document;
use masonry::peniko::Brush;

use super::GutterPaintContext;

/// A gutter element (line numbers, breakpoints, VCS bars, a "run" button,
/// …). Built-in controls cover the common cases; apps register their own
/// for anything else.
pub trait GutterControl: Any + Send + Sync {
    /// Unique identifier, reported back in
    /// [`CodeEditorAction::GutterControlClicked`](crate::CodeEditorAction::GutterControlClicked).
    fn id(&self) -> &'static str;

    /// Paint this control. Called in z-order (lowest first).
    fn paint(&self, ctx: &mut GutterPaintContext<'_, '_, '_>, brushes: &[Brush]);

    /// A click on 1-based `line` at gutter position (`x`, `y`). Return
    /// `true` to consume it; unconsumed clicks become
    /// [`CodeEditorAction::GutterClicked`](crate::CodeEditorAction::GutterClicked).
    fn on_click(&mut self, _line: usize, _x: f64, _y: f64) -> bool {
        false
    }

    /// The document changed; controls that derive their markers from the
    /// text (e.g. a "run" button on certain lines) update here.
    fn document_changed(&mut self, _document: &dyn Document) {}

    /// Z-order for painting (higher = on top).
    fn z_order(&self) -> i32 {
        0
    }
}
