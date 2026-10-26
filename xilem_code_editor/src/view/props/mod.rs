// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! The view's data, grouped by feature. Each group applies itself when the
//! widget is built and sends only what changed on rebuild.

mod completion;
mod display;
mod gutter;
mod gutter_controls;
mod highlighting;
mod hover;
mod language_data;
mod quick_help;

pub(crate) use completion::CompletionProps;
pub(crate) use display::DisplayProps;
pub(crate) use gutter::GutterProps;
pub use gutter_controls::GutterControlFactory;
pub(crate) use gutter_controls::GutterControlsProps;
pub(crate) use highlighting::HighlightingProps;
pub(crate) use hover::HoverProps;
pub(crate) use language_data::LanguageDataProps;
pub(crate) use quick_help::QuickHelpProps;
