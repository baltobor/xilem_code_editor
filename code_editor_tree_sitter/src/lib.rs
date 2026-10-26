// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Syntax highlighting for `code_editor_core` via tree-sitter.
//!
//! Enable the languages you need with `lang-*` features
//! (`lang-markdown`, `lang-rust`, …, or `all-languages`); a Markdown-only
//! app pulls in just the Markdown grammars.

mod capture_category;
pub mod languages;
#[cfg(feature = "lang-pem")]
pub mod pem;
pub mod registry;
mod syntax_highlighter;
mod todo_comment_pass;
mod tree_sitter_highlighter;

#[cfg(test)]
mod tests;

pub use capture_category::{HIGHLIGHT_NAMES, capture_to_category};
pub use registry::{LanguageConfig, LanguageRegistry, registry};
pub use syntax_highlighter::SyntaxHighlighter;
pub use todo_comment_pass::{TodoCommentPass, is_task_comment};
pub use tree_sitter_highlighter::TreeSitterHighlighter;
