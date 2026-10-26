// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// Kind of a line in a unified diff hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    /// Unchanged context line.
    Context,
    /// Added line.
    Addition,
    /// Removed line.
    Deletion,
}

/// One line of a diff hunk, as the gutter needs it. Apps map their own
/// diff representation (git2, `git diff` output, …) onto this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    /// Line text including its trailing newline.
    pub content: String,
    /// 1-based line number on the new (working-copy) side, for context
    /// and added lines.
    pub new_line: Option<usize>,
}
