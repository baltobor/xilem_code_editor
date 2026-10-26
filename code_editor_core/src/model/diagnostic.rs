// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// Severity of a [`Diagnostic`]. Colors are chosen by the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// A compiler / language-server message attached to a line range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// 0-based buffer line.
    pub line: usize,
    /// Start byte column within the line (squiggle start).
    pub col: usize,
    /// End byte column within the line (squiggle end).
    pub end_col: usize,
    pub message: String,
    pub severity: DiagnosticSeverity,
}
