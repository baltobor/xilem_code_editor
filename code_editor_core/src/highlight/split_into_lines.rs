// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use super::HighlightSpan;

/// Distribute document-wide spans into per-line spans (byte ranges
/// relative to each line). Spans crossing lines are split; boundaries are
/// snapped forward to char boundaries so a span can never cut a
/// multi-byte glyph (parley would panic on the next layout).
pub fn split_into_lines(source: &str, flat: &[HighlightSpan]) -> Vec<Vec<HighlightSpan>> {
    // Compute line start offsets
    let mut line_starts: Vec<usize> = vec![0];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }
    let line_count = line_starts.len();
    let mut per_line: Vec<Vec<HighlightSpan>> = vec![Vec::new(); line_count];

    let bytes = source.as_bytes();
    // Move `idx` forward (or stay) so it lands on the next char
    // boundary in `bytes`. UTF-8 continuation bytes have the
    // pattern `10xx_xxxx` (`0x80..=0xBF`); leading bytes start
    // a new char. Defensive against tree-sitter spans that
    // truncate inside a multi-byte glyph (`→` etc.) — without
    // this, parley panics on the next layout pass.
    let snap_to_boundary = |idx: usize| -> usize {
        let mut i = idx.min(bytes.len());
        while i < bytes.len() && (bytes[i] & 0b1100_0000) == 0b1000_0000 {
            i += 1;
        }
        i
    };

    for span in flat {
        // Find which lines this span covers
        let start_line = line_starts
            .partition_point(|&o| o <= span.byte_range.start)
            .saturating_sub(1);
        let end_line = line_starts
            .partition_point(|&o| o < span.byte_range.end)
            .saturating_sub(1);

        for line in start_line..=end_line.min(line_count - 1) {
            let line_start = line_starts[line];
            let line_end = if line + 1 < line_starts.len() {
                line_starts[line + 1]
            } else {
                source.len()
            };

            let raw_start = span.byte_range.start.max(line_start);
            let raw_end = span.byte_range.end.min(line_end);
            let snapped_start = snap_to_boundary(raw_start);
            let snapped_end = snap_to_boundary(raw_end);
            let span_start_in_line = snapped_start - line_start;
            let span_end_in_line = snapped_end - line_start;

            if span_start_in_line < span_end_in_line {
                per_line[line].push(HighlightSpan {
                    byte_range: span_start_in_line..span_end_in_line,
                    category: span.category,
                });
            }
        }
    }

    per_line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlight::HighlightCategory;

    #[test]
    fn splits_multi_line_span() {
        let src = "ab\ncd\n";
        let flat = [HighlightSpan {
            byte_range: 1..5,
            category: HighlightCategory::String,
        }];
        let lines = split_into_lines(src, &flat);
        assert_eq!(lines[0][0].byte_range, 1..3);
        assert_eq!(lines[1][0].byte_range, 0..2);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn snaps_to_char_boundary() {
        let src = "a→b";
        let flat = [HighlightSpan {
            byte_range: 0..2,
            category: HighlightCategory::String,
        }];
        assert_eq!(split_into_lines(src, &flat)[0][0].byte_range, 0..4);
    }
}
