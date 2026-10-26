// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use xilem::WidgetView;
use xilem::masonry::layout::Length;
use xilem::style::Style as _;
use xilem::view::{FlexSpacer, flex_row};
use xilem_material_icons::icons;

use crate::ToolbarStyle;
use crate::format_toolbar::toolbar_toggle;

/// How a Markdown document is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MarkdownViewMode {
    /// Source only.
    #[default]
    Edit,
    /// Rendered only.
    Preview,
    /// Source and rendered side by side.
    Split,
}

/// Edit / Preview, Split and word-wrap toggles (right side of the bar).
pub fn view_mode_buttons<State, M, W>(
    mode: MarkdownViewMode,
    word_wrap: bool,
    style: ToolbarStyle,
    on_mode: M,
    on_toggle_wrap: W,
) -> impl WidgetView<State> + use<State, M, W>
where
    State: 'static,
    M: Fn(&mut State, MarkdownViewMode) + Clone + Send + Sync + 'static,
    W: Fn(&mut State) + Send + Sync + 'static,
{
    let mode_button = |icon, target: MarkdownViewMode| {
        let on_mode = on_mode.clone();
        toolbar_toggle(icon, style, mode == target, move |state: &mut State| {
            on_mode(state, target)
        })
    };
    flex_row((
        mode_button(icons::EDIT_NOTE, MarkdownViewMode::Edit),
        mode_button(icons::PREVIEW, MarkdownViewMode::Preview),
        FlexSpacer::Fixed(Length::px(8.0)),
        mode_button(icons::VERTICAL_SPLIT, MarkdownViewMode::Split),
        toolbar_toggle(icons::WRAP_TEXT, style, word_wrap, on_toggle_wrap),
    ))
    .gap(Length::px(6.0))
}
