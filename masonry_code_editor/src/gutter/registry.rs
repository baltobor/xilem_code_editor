// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::any::Any;

use code_editor_core::document::Document;
use masonry::peniko::Brush;

use super::{GutterControl, GutterPaintContext};

/// The registered gutter controls, painted in z-order.
pub struct GutterRegistry {
    controls: Vec<Box<dyn GutterControl>>,
    sorted: bool,
}

impl GutterRegistry {
    /// Build an empty registry. The widget pushes a default set of
    /// controls (line numbers, diagnostic markers, breakpoints,
    /// execution-line marker) during construction.
    pub fn new() -> Self {
        Self {
            controls: Vec::new(),
            sorted: true,
        }
    }

    /// Register a new gutter control.
    pub fn register(&mut self, control: Box<dyn GutterControl>) {
        self.controls.push(control);
        self.sorted = false;
    }

    /// The control registered under `id`, if it is a `T`.
    pub fn get_mut<T: GutterControl>(&mut self, id: &str) -> Option<&mut T> {
        self.controls
            .iter_mut()
            .find(|c| c.id() == id)
            .and_then(|c| (c.as_mut() as &mut dyn Any).downcast_mut::<T>())
    }

    /// Paint all controls in z-order.
    pub fn paint_all(&mut self, ctx: &mut GutterPaintContext<'_, '_, '_>, brushes: &[Brush]) {
        self.sort();
        for control in &self.controls {
            control.paint(ctx, brushes);
        }
    }

    /// Offer a click to the controls, topmost first. Returns the id of the
    /// control that consumed it.
    pub fn on_click(&mut self, line: usize, x: f64, y: f64) -> Option<&'static str> {
        self.sort();
        self.controls
            .iter_mut()
            .rev()
            .find_map(|control| control.on_click(line, x, y).then(|| control.id()))
    }

    /// Notify every control that the document changed.
    pub fn document_changed(&mut self, document: &dyn Document) {
        for control in &mut self.controls {
            control.document_changed(document);
        }
    }

    fn sort(&mut self) {
        if !self.sorted {
            self.controls.sort_by_key(|c| c.z_order());
            self.sorted = true;
        }
    }
}

impl Default for GutterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
