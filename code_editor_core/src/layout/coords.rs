// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0
// Adapted from Lapce (Apache-2.0) and Floem (MIT); see README.md.

//! Visual coordinate types for the code editor.
//!
//! The editor uses three coordinate systems:
//! - Buffer offsets (bytes) — the ground truth in the Rope
//! - Buffer lines (0-based) — which line in the Rope
//! - Visual lines — what the user sees (accounts for word wrapping)
//!
//! These types make conversions explicit and prevent mixing up coordinate spaces.

/// Absolute visual line index (0-based).
///
/// This counts every visual line from the top of the document:
/// - Unwrapped lines each contribute 1 visual line
/// - A wrapped buffer line contributes N visual lines (one per wrap segment)
///
/// Used for: y_position = vline.0 * line_height
/// Unstable across edits — inserting a line shifts all subsequent VLines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(dead_code)]
pub struct VLine(pub usize);

/// Relative visual line — stable across edits to other lines.
///
/// Identifies a specific wrap segment within a specific buffer line.
/// Even if lines above are inserted or deleted, an RVLine pointing at
/// buffer line 42, segment 0 still refers to the same text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RVLine {
    /// Buffer line number (0-based).
    pub line: usize,
    /// Which wrap segment within that buffer line.
    /// 0 = first (or only) segment, 1+ = wrapped continuations.
    pub line_index: usize,
}

impl RVLine {
    /// Create an RVLine pointing to the first segment of a buffer line.
    pub fn new(line: usize) -> Self {
        Self {
            line,
            line_index: 0,
        }
    }

    /// Create an RVLine pointing to a specific wrap segment.
    pub fn with_index(line: usize, line_index: usize) -> Self {
        Self { line, line_index }
    }
}

/// Horizontal column position in pixels.
///
/// Preserved across vertical cursor movement (up/down arrows) so the cursor
/// returns to its original column even after passing through shorter lines.
/// This matches the behavior users expect from every text editor.
#[derive(Debug, Clone, Copy)]
pub struct ColPosition(pub f64);
