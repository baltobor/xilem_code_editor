// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// A run of removed lines, anchored at the new-side line it follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletionMark {
    /// 1-based new-side line the removal sits *after* (0 = top of file). The
    /// gutter draws the beige marker at the top of `after_new_line + 1`.
    pub after_new_line: usize,
    /// The removed (HEAD) text, shown in the balloon on click.
    pub removed_text: String,
}
