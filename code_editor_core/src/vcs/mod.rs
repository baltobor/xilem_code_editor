// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Version-control gutter markers: which lines were added or modified
//! since HEAD, and where lines were removed.

mod deletion_mark;
mod diff_line;
mod from_hunks;
mod line_mark;
mod vcs_gutter_diff;

pub use deletion_mark::DeletionMark;
pub use diff_line::{DiffLine, DiffLineKind};
pub use line_mark::LineMark;
pub use vcs_gutter_diff::VcsGutterDiff;
