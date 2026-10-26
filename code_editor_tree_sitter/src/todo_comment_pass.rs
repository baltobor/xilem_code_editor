// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use code_editor_core::highlight::{HighlightCategory, HighlightSpan};
use code_editor_core::provider::HighlightPostPass;

/// Re-classifies `// TODO:`, `FIXME`, `XXX`, `HACK` and `BUG` line
/// comments as [`HighlightCategory::TodoComment`], so the theme can paint
/// them differently from ordinary comments.
#[derive(Debug, Default, Clone, Copy)]
pub struct TodoCommentPass;

impl HighlightPostPass for TodoCommentPass {
    fn apply(&self, text: &str, _language: &str, spans: &mut Vec<HighlightSpan>) {
        for span in spans.iter_mut() {
            if span.category == HighlightCategory::Comment
                && text
                    .get(span.byte_range.clone())
                    .is_some_and(is_task_comment)
            {
                span.category = HighlightCategory::TodoComment;
            }
        }
    }
}

/// `true` for a `//` comment starting with a task marker (case-insensitive,
/// whole word: `todoist` does not count).
pub fn is_task_comment(text: &str) -> bool {
    let trimmed = text.trim_start();
    let Some(rest) = trimmed.strip_prefix("//") else {
        return false;
    };
    let rest = rest.trim_start();
    // Compare on bytes, not `&rest[..n]`: a non-ASCII leading char like `→`
    // (3 bytes) would make a `str` slice land mid-codepoint and panic. Byte
    // slicing of a `&[u8]` is always safe.
    let bytes = rest.as_bytes();
    // Each marker must be followed by a non-word char (or end-of-string), so
    // `todoist` / `fixmeup` don't false-match.
    const MARKERS: [&[u8]; 5] = [b"todo", b"fixme", b"xxx", b"hack", b"bug"];
    MARKERS.iter().any(|marker| {
        bytes.len() >= marker.len()
            && bytes[..marker.len()].eq_ignore_ascii_case(marker)
            && match bytes.get(marker.len()) {
                None => true,
                Some(c) => !(c.is_ascii_alphanumeric() || *c == b'_'),
            }
    })
}
