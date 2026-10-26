// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use xilem::masonry::layout::Length;
use xilem::masonry::properties::Padding;
use xilem::style::Style as _;
use xilem::view::{button, flex_row, label};
use xilem::{AnyWidgetView, Color, FontWeight, WidgetView};
use xilem_material_icons::{FONT_FAMILY, icons};

use crate::{MarkdownFormat, ToolbarStyle};

/// What a toolbar button shows.
#[derive(Debug, Clone, Copy)]
enum Glyph {
    Icon(&'static str),
    Text(&'static str),
}

/// The buttons, in display order.
const ITEMS: [(MarkdownFormat, Glyph); 14] = [
    (MarkdownFormat::Bold, Glyph::Icon(icons::FORMAT_BOLD)),
    (MarkdownFormat::Italic, Glyph::Icon(icons::FORMAT_ITALIC)),
    (
        MarkdownFormat::Underline,
        Glyph::Icon(icons::FORMAT_UNDERLINED),
    ),
    (
        MarkdownFormat::Strikethrough,
        Glyph::Icon(icons::FORMAT_STRIKETHROUGH),
    ),
    (MarkdownFormat::InlineCode, Glyph::Icon(icons::CODE)),
    (MarkdownFormat::Heading1, Glyph::Text("H1")),
    (MarkdownFormat::Heading2, Glyph::Text("H2")),
    (MarkdownFormat::Heading3, Glyph::Text("H3")),
    (
        MarkdownFormat::BulletList,
        Glyph::Icon(icons::FORMAT_LIST_BULLETED),
    ),
    (
        MarkdownFormat::NumberedList,
        Glyph::Icon(icons::FORMAT_LIST_NUMBERED),
    ),
    (MarkdownFormat::Quote, Glyph::Icon(icons::FORMAT_QUOTE)),
    (MarkdownFormat::Table, Glyph::Icon(icons::TABLE_CHART)),
    (MarkdownFormat::PlainCodeBlock, Glyph::Text("pre")),
    (MarkdownFormat::CodeBlock, Glyph::Icon(icons::CODE_BLOCKS)),
];

/// The Markdown formatting buttons (bold … code block). `extra` buttons
/// (an app's image upload, help, …) follow them in the same row.
pub fn markdown_format_toolbar<State, F>(
    style: ToolbarStyle,
    on_format: F,
    extra: Vec<Box<AnyWidgetView<State>>>,
) -> impl WidgetView<State> + use<State, F>
where
    State: 'static,
    F: Fn(&mut State, MarkdownFormat) + Clone + Send + Sync + 'static,
{
    let mut buttons: Vec<Box<AnyWidgetView<State>>> = ITEMS
        .iter()
        .map(|&(format, glyph)| {
            let on_format = on_format.clone();
            toolbar_button(
                glyph,
                style,
                Color::TRANSPARENT,
                move |state: &mut State| on_format(state, format),
            )
        })
        .collect();
    buttons.extend(extra);
    flex_row(buttons).gap(Length::px(2.0))
}

/// An icon button styled like the toolbar's own (for `extra` buttons).
pub fn toolbar_icon_button<State: 'static>(
    icon: &'static str,
    style: ToolbarStyle,
    on_press: impl Fn(&mut State) + Send + Sync + 'static,
) -> Box<AnyWidgetView<State>> {
    toolbar_button(Glyph::Icon(icon), style, Color::TRANSPARENT, on_press)
}

pub(crate) fn toolbar_toggle<State: 'static>(
    icon: &'static str,
    style: ToolbarStyle,
    active: bool,
    on_press: impl Fn(&mut State) + Send + Sync + 'static,
) -> Box<AnyWidgetView<State>> {
    let background = if active {
        style.active_background
    } else {
        Color::TRANSPARENT
    };
    toolbar_button(Glyph::Icon(icon), style, background, on_press)
}

fn toolbar_button<State: 'static>(
    glyph: Glyph,
    style: ToolbarStyle,
    background: Color,
    on_press: impl Fn(&mut State) + Send + Sync + 'static,
) -> Box<AnyWidgetView<State>> {
    let content = match glyph {
        Glyph::Icon(icon) => label(icon)
            .font(FONT_FAMILY)
            .text_size(style.icon_size)
            .color(style.icon_color)
            .boxed(),
        Glyph::Text(text) => label(text)
            .text_size(style.text_size)
            .weight(FontWeight::BOLD)
            .color(style.icon_color)
            .boxed(),
    };
    button(content, on_press)
        .background_color(background)
        .border(Color::TRANSPARENT, Length::px(0.0))
        .padding(Padding {
            top: Length::px(3.0),
            bottom: Length::px(3.0),
            left: Length::px(4.0),
            right: Length::px(4.0),
        })
        .boxed()
}
