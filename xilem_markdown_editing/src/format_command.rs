// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use xilem_code_editor::EditorCommand;

use crate::MarkdownFormat;

/// The edit that applies `format` to `text`.
///
/// - Inline formats wrap the selection (`selection`: byte range), or with
///   no selection insert a placeholder and select it, so typing replaces it.
/// - Line formats prefix the caret's line (`cursor_line`, 1-based).
/// - Blocks insert a skeleton at the caret.
pub fn format_command(
    format: MarkdownFormat,
    text: &str,
    selection: Option<(usize, usize)>,
    cursor_line: usize,
) -> EditorCommand {
    use MarkdownFormat as F;
    match format {
        F::Bold => wrap(text, selection, "**", "**", "bold text"),
        F::Italic => wrap(text, selection, "*", "*", "italic text"),
        F::Underline => wrap(text, selection, "<u>", "</u>", "underlined text"),
        F::Strikethrough => wrap(text, selection, "~~", "~~", "strikethrough text"),
        F::InlineCode => wrap(text, selection, "`", "`", "code"),
        F::Heading1 => prefix_line(text, cursor_line, "# "),
        F::Heading2 => prefix_line(text, cursor_line, "## "),
        F::Heading3 => prefix_line(text, cursor_line, "### "),
        F::BulletList => prefix_line(text, cursor_line, "- "),
        F::NumberedList => prefix_line(text, cursor_line, "1. "),
        F::Quote => prefix_line(text, cursor_line, "> "),
        F::Table => EditorCommand::Insert {
            text: "| Header | Header |\n| --- | --- |\n| Cell | Cell |\n".to_string(),
            select: None,
        },
        F::PlainCodeBlock => EditorCommand::Insert {
            text: "```\n\n```\n".to_string(),
            select: Some((4, 4)),
        },
        F::CodeBlock => EditorCommand::Insert {
            text: "```\n\n```\n".to_string(),
            select: Some((3, 3)),
        },
    }
}

fn wrap(
    text: &str,
    selection: Option<(usize, usize)>,
    prefix: &str,
    suffix: &str,
    placeholder: &str,
) -> EditorCommand {
    match selection
        .filter(|(start, end)| end > start)
        .and_then(|(start, end)| Some((start, end, text.get(start..end)?)))
    {
        Some((start, end, selected)) => EditorCommand::Replace {
            range: start..end,
            text: format!("{prefix}{selected}{suffix}"),
        },
        None => EditorCommand::Insert {
            text: format!("{prefix}{placeholder}{suffix}"),
            select: Some((prefix.len(), prefix.len() + placeholder.len())),
        },
    }
}

fn prefix_line(text: &str, cursor_line: usize, prefix: &str) -> EditorCommand {
    let start = line_start(text, cursor_line);
    EditorCommand::Replace {
        range: start..start,
        text: prefix.to_string(),
    }
}

/// Byte offset of 1-based `line` in `text` (clamped to the end).
fn line_start(text: &str, line: usize) -> usize {
    if line <= 1 {
        return 0;
    }
    text.match_indices('\n')
        .nth(line - 2)
        .map_or(text.len(), |(i, _)| i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_the_selection() {
        let cmd = format_command(MarkdownFormat::Bold, "say hello", Some((4, 9)), 1);
        assert_eq!(
            cmd,
            EditorCommand::Replace {
                range: 4..9,
                text: "**hello**".into()
            }
        );
    }

    #[test]
    fn inserts_and_selects_a_placeholder_without_selection() {
        let cmd = format_command(MarkdownFormat::Italic, "x", None, 1);
        assert_eq!(
            cmd,
            EditorCommand::Insert {
                text: "*italic text*".into(),
                select: Some((1, 12))
            }
        );
    }

    #[test]
    fn prefixes_the_caret_line() {
        let cmd = format_command(MarkdownFormat::Heading2, "a\nbb\nccc", None, 3);
        assert_eq!(
            cmd,
            EditorCommand::Replace {
                range: 5..5,
                text: "## ".into()
            }
        );
    }

    #[test]
    fn line_start_clamps() {
        assert_eq!(line_start("one\ntwo\n", 99), 8);
        assert_eq!(line_start("one", 1), 0);
    }
}
