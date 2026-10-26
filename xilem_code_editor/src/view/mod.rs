// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! The `CodeEditor` view.

mod builder;
mod code_editor;
mod lifecycle;
mod props;

pub use code_editor::{CodeEditor, code_editor};
pub use lifecycle::CodeEditorViewState;
pub use props::GutterControlFactory;
