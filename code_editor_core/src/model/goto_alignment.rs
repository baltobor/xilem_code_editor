// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// Where a "go to line" places the target line in the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GotoAlignment {
    /// Scroll only as far as needed to make the line visible.
    Edge,
    /// Center the line (default: shows context above and below).
    #[default]
    Center,
    /// Put the line at the top.
    Top,
}
