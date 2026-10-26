// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

/// A predicted edit shown as ghost text and accepted with Tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPrediction {
    /// Byte range the prediction replaces (empty for a pure insertion).
    pub range: Range<usize>,
    pub text: String,
}
