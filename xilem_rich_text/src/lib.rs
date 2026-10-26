// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Rich text for Xilem: a wrapping paragraph with inline styles (bold,
//! italic, code, strikethrough) and clickable links. masonry's `Label` and
//! `Prose` only style the whole text; this lays out styled byte ranges
//! with parley's ranged builder.
//!
//! - `RichParagraph` — the model (text, style ranges, links)
//! - `RichTextStyle` — typography (font stack, size, line height, colors)
//! - `rich_text(paragraph, style, on_link)` — the view
//! - `rich_table(rows, style, colors, on_link)` — a table of rich-text cells
//! - `reading_font_stack()` — a reading serif per OS
//! - `scroll_host(child)` / `scroll_anchor(child, reveal)` — scroll a portal
//!   so a block sits at its top (table-of-contents jumps)

mod reading_font;
mod rich_paragraph;
mod rich_table;
mod rich_text_style;
mod rich_text_view;
mod rich_text_widget;
mod scroll_anchor;

pub use reading_font::reading_font_stack;
pub use rich_paragraph::{LinkSpan, RichParagraph, SpanStyle};
pub use rich_table::{LinkHandler, TableColors, TableRow, rich_table};
pub use rich_text_style::RichTextStyle;
pub use rich_text_view::{RichText, rich_text};
pub use rich_text_widget::{LinkClicked, RichTextWidget};
pub use scroll_anchor::{
    RevealAnchor, ScrollAnchor, ScrollAnchorView, ScrollHost, ScrollHostView, scroll_anchor,
    scroll_host,
};
