// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// A `(line, character)` position as language servers send it.
///
/// `character` is counted in chars (Unicode scalar values) within the
/// line — the same semantics the host models use, so widget and model
/// compute identical byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct TextPosition {
    /// 0-based line.
    pub line: u32,
    /// 0-based char index within the line.
    pub character: u32,
}

impl TextPosition {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }

    /// Byte offset of this position in `text`. Positions past the end of
    /// a line clamp to the line end; past the last line, to `text.len()`.
    pub fn to_byte_offset(self, text: &str) -> usize {
        let target_line = self.line as usize;
        let mut line_start = 0;
        for _ in 0..target_line {
            match text[line_start..].find('\n') {
                Some(nl) => line_start += nl + 1,
                None => return text.len(),
            }
        }
        let line_text = &text[line_start..];
        let line_end = line_text.find('\n').unwrap_or(line_text.len());
        line_text[..line_end]
            .char_indices()
            .nth(self.character as usize)
            .map_or(line_start + line_end, |(byte, _)| line_start + byte)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_positions() {
        let text = "ab\ncde\n";
        assert_eq!(TextPosition::new(0, 0).to_byte_offset(text), 0);
        assert_eq!(TextPosition::new(1, 2).to_byte_offset(text), 5);
    }

    #[test]
    fn multibyte_chars_count_as_one() {
        let text = "äö\nx";
        assert_eq!(TextPosition::new(0, 1).to_byte_offset(text), 2);
        assert_eq!(TextPosition::new(1, 0).to_byte_offset(text), 5);
    }

    #[test]
    fn past_line_end_clamps() {
        let text = "ab\ncd";
        assert_eq!(TextPosition::new(0, 9).to_byte_offset(text), 2);
        assert_eq!(TextPosition::new(7, 0).to_byte_offset(text), 5);
    }
}
