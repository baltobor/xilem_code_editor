// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use masonry_code_editor::CodeEditorWidget;
use masonry_code_editor::gutter::GutterControl;

/// Creates the app's own gutter controls for a new editor widget.
pub type GutterControlFactory = Arc<dyn Fn() -> Vec<Box<dyn GutterControl>> + Send + Sync>;

/// App-specific gutter controls; created once, when the widget is built.
#[derive(Clone, Default)]
pub(crate) struct GutterControlsProps {
    pub factory: Option<GutterControlFactory>,
}

impl GutterControlsProps {
    pub(crate) fn build(&self, widget: &mut CodeEditorWidget) {
        for control in self.factory.iter().flat_map(|factory| factory()) {
            widget.register_gutter_control_direct(control);
        }
    }
}
