// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// A formatting action of the Markdown toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkdownFormat {
    Bold,
    Italic,
    /// Raw HTML `<u>`: Markdown has no native underline.
    Underline,
    Strikethrough,
    InlineCode,
    Heading1,
    Heading2,
    Heading3,
    BulletList,
    NumberedList,
    Quote,
    Table,
    /// A plain fenced block, caret inside.
    PlainCodeBlock,
    /// A fenced block with the caret after the fence, to type the language.
    CodeBlock,
}
