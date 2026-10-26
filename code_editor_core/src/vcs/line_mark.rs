// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// A changed new-side line. Added and Modified both render green; the
/// distinction is kept for a possible future colour split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineMark {
    /// Line is new since HEAD (pure insertion).
    Added,
    /// Line replaced a removed line (the hunk deleted as well as added).
    Modified,
}
