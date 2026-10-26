// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Programmatic edits (from the host, completions, quick fixes), each
//! applied as a single undo step.

mod apply_text_edit;
mod apply_text_edits;
mod insert_at_caret;
mod replace_range;
mod text_edit;
mod text_position;

pub use apply_text_edit::apply_text_edit;
pub use apply_text_edits::apply_text_edits;
pub use insert_at_caret::insert_at_caret;
pub use replace_range::replace_range;
pub use text_edit::TextEdit;
pub use text_position::TextPosition;
