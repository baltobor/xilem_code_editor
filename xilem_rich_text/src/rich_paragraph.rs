// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! A paragraph of text with inline styles and links — the model the
//! rich-text widget lays out. No UI dependencies.

use std::ops::Range;

/// Inline formatting of a text run. Flags combine (`***x***` is bold and
/// italic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpanStyle {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
}

/// A clickable range and what it points to (interpreted by the caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkSpan {
    pub range: Range<usize>,
    pub target: String,
}

/// Text plus styled byte ranges and links.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RichParagraph {
    pub text: String,
    pub styles: Vec<(Range<usize>, SpanStyle)>,
    pub links: Vec<LinkSpan>,
}

impl RichParagraph {
    /// Append `text` with `style`; `link` makes the run clickable.
    pub fn push(&mut self, text: &str, style: SpanStyle, link: Option<&str>) {
        if text.is_empty() {
            return;
        }
        let range = self.text.len()..self.text.len() + text.len();
        self.text.push_str(text);
        if style != SpanStyle::default() {
            self.styles.push((range.clone(), style));
        }
        if let Some(target) = link {
            match self.links.last_mut() {
                // Consecutive runs of the same link form one clickable range.
                Some(last) if last.target == target && last.range.end == range.start => {
                    last.range.end = range.end
                }
                _ => self.links.push(LinkSpan {
                    range,
                    target: target.to_string(),
                }),
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// The link covering byte `index`, if any.
    pub fn link_at(&self, index: usize) -> Option<&LinkSpan> {
        self.links.iter().find(|link| link.range.contains(&index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_and_links_keep_their_ranges_and_spaces() {
        let mut p = RichParagraph::default();
        p.push("ssasda ", SpanStyle::default(), None);
        p.push("test", SpanStyle::default(), Some("wiki:test"));
        p.push(" and ", SpanStyle::default(), None);
        p.push(
            "bold",
            SpanStyle {
                bold: true,
                ..Default::default()
            },
            None,
        );
        assert_eq!(p.text, "ssasda test and bold");
        assert_eq!(p.links[0].range, 7..11);
        assert_eq!(
            p.styles,
            vec![(
                16..20,
                SpanStyle {
                    bold: true,
                    ..Default::default()
                }
            )]
        );
        assert_eq!(p.link_at(8).map(|l| l.target.as_str()), Some("wiki:test"));
        assert!(p.link_at(5).is_none());
    }

    #[test]
    fn adjacent_runs_of_one_link_merge() {
        let mut p = RichParagraph::default();
        p.push("a ", SpanStyle::default(), Some("url:x"));
        p.push(
            "b",
            SpanStyle {
                bold: true,
                ..Default::default()
            },
            Some("url:x"),
        );
        assert_eq!(p.links.len(), 1);
        assert_eq!(p.links[0].range, 0..3);
    }
}
