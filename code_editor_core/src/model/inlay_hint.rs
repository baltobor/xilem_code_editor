// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// Kind of an [`InlayHint`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InlayHintKind {
    /// Type annotation, e.g. `: i32`.
    Type,
    /// Parameter name, e.g. `name:`.
    Parameter,
    #[default]
    Other,
}

/// Virtual text shown inside a line without being part of the buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlayHint {
    /// 0-based buffer line.
    pub line: usize,
    /// Byte column the hint is displayed at.
    pub col: usize,
    pub text: String,
    pub kind: InlayHintKind,
}
