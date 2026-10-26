// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! The code editor as a [masonry] widget: text layout, painting, gutter,
//! popups and input handling, on top of `code_editor_core`'s model.

pub mod action;
pub mod gutter;
pub mod input;
pub mod paint;
pub mod popup;
pub mod properties;
pub mod scroll;
pub mod state;
pub mod text_layout;
pub mod widgets;

pub use action::CodeEditorAction;
pub use properties::{EditorFontFamily, EditorTheme};
pub use widgets::CodeEditorWidget;
