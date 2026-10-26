// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! The simplest use: a soft-wrapping Markdown editor.
//!
//! `cargo run --release -p xilem_code_editor --example markdown_basic`

use std::sync::Arc;

use code_editor_tree_sitter::{TodoCommentPass, TreeSitterHighlighter};
use xilem::winit::error::EventLoopError;
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};
use xilem_code_editor::code_editor;

struct Notes {
    text: String,
    highlighter: Arc<TreeSitterHighlighter>,
}

fn app_logic(notes: &mut Notes) -> impl WidgetView<Notes> + use<> {
    code_editor(notes.text.clone(), |notes: &mut Notes, text| {
        notes.text = text
    })
    .language("md")
    .highlighter(notes.highlighter.clone())
    .highlight_pass(Arc::new(TodoCommentPass))
    .word_wrap(true)
    .show_line_numbers(true)
}

fn main() -> Result<(), EventLoopError> {
    let notes = Notes {
        text: SAMPLE.to_string(),
        highlighter: Arc::new(TreeSitterHighlighter::new()),
    };
    Xilem::new_simple(notes, app_logic, WindowOptions::new("Markdown editor"))
        .run_in(EventLoop::with_user_event())
}

const SAMPLE: &str = "# Soft wrap\n\nThis paragraph is deliberately long so that it wraps onto several visual rows in a normal window. Select across the wrap with the mouse or with Shift+Down: every visual row of the selection must be highlighted, not only the first one. The caret moves row by row with the arrow keys.\n\n## Code\n\n```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n\n- **bold**, _italic_, `code`\n";
