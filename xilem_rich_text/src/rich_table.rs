// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! A table of rich-text cells. Grid lines are the background showing
//! through 1 px gaps between rows and cells (plus a 1 px frame), so no
//! line can be covered by a cell that measures taller than expected.

use std::sync::Arc;

use xilem::masonry::layout::Length;
use xilem::masonry::properties::Padding;
use xilem::style::Style as _;
use xilem::view::{AnyFlexChild, CrossAxisAlignment, FlexExt, flex_col, flex_row};
use xilem::{AnyWidgetView, Color, WidgetView};

use crate::{RichParagraph, RichTextStyle, rich_text};

/// Called with a clicked link's target.
pub type LinkHandler<State> = Arc<dyn Fn(&mut State, &str) + Send + Sync>;

/// One table row: whether it is a header row, and its cells.
pub type TableRow = (bool, Vec<RichParagraph>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableColors {
    pub grid: Color,
    pub header_background: Color,
    pub cell_background: Color,
}

/// `rows` as a table; columns share the width equally, short rows are
/// padded with empty cells, header rows are bold.
pub fn rich_table<State: 'static>(
    rows: Vec<TableRow>,
    style: RichTextStyle,
    colors: TableColors,
    on_link: LinkHandler<State>,
) -> impl WidgetView<State> + use<State> {
    let columns = rows.iter().map(|(_, cells)| cells.len()).max().unwrap_or(0);
    let rows: Vec<Box<AnyWidgetView<State>>> = rows
        .into_iter()
        .map(|(is_header, mut cells)| {
            cells.resize_with(columns, RichParagraph::default);
            let cell_style = RichTextStyle {
                bold_base: is_header,
                ..style.clone()
            };
            let background = if is_header {
                colors.header_background
            } else {
                colors.cell_background
            };
            let cells: Vec<AnyFlexChild<State>> = cells
                .into_iter()
                .map(|cell| {
                    table_cell(cell, cell_style.clone(), background, on_link.clone())
                        .flex(1.0)
                        .into_any_flex()
                })
                .collect();
            flex_row(cells)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(Length::px(1.0))
                .boxed()
        })
        .collect();
    flex_col(rows)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(Length::px(1.0))
        .padding(Length::px(1.0))
        .background_color(colors.grid)
}

fn table_cell<State: 'static>(
    cell: RichParagraph,
    style: RichTextStyle,
    background: Color,
    on_link: LinkHandler<State>,
) -> impl WidgetView<State> + use<State> {
    flex_col((rich_text(
        cell,
        style,
        move |state: &mut State, target: &str| on_link(state, target),
    ),))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .padding(Padding {
        top: Length::px(5.0),
        bottom: Length::px(5.0),
        left: Length::px(8.0),
        right: Length::px(8.0),
    })
    .background_color(background)
}
