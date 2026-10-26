// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use super::{DeletionMark, LineMark};

/// All VCS gutter markers for one file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VcsGutterDiff {
    /// (1-based new-side line, mark) for every added/changed line.
    pub line_marks: Vec<(usize, LineMark)>,
    /// Removed-line markers.
    pub deletions: Vec<DeletionMark>,
}

impl VcsGutterDiff {
    pub fn is_empty(&self) -> bool {
        self.line_marks.is_empty() && self.deletions.is_empty()
    }
}
