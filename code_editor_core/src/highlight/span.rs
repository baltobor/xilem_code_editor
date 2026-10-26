// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use super::HighlightCategory;

/// A byte range within one line, with its highlight category.
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightSpan {
    pub byte_range: Range<usize>,
    pub category: HighlightCategory,
}
