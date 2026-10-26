// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! The gutter: line numbers, markers and buttons left of the text.
//!
//! Every element is a [`GutterControl`] in a [`GutterRegistry`]; the
//! built-in controls below are configured instances, and apps register
//! their own.

mod breakpoints;
mod diagnostic_markers;
mod execution_line;
mod gutter_control;
mod line_numbers;
mod paint_context;
mod registry;
mod vcs_changes;

pub use breakpoints::BreakpointsControl;
pub use diagnostic_markers::{DiagnosticMarkersControl, DiagnosticSpan};
pub use execution_line::ExecutionLineControl;
pub use gutter_control::GutterControl;
pub use line_numbers::LineNumbersControl;
pub use paint_context::GutterPaintContext;
pub use registry::GutterRegistry;
pub use vcs_changes::VcsChangesControl;
