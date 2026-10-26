// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! A wrapping paragraph with inline styles (bold, italic, code,
//! strikethrough) and clickable links, laid out with parley's ranged
//! builder. Modeled on masonry's `Label`; `Label`/`Prose` only style the
//! whole text, which is why this exists.

use std::borrow::Cow;

use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, BrushIndex, ChildrenIds, CursorIcon, EventCtx, LayoutCtx, MeasureCtx, PaintCtx,
    PointerButton, PointerButtonEvent, PointerEvent, PropertiesMut, PropertiesRef, QueryCtx,
    RegisterCtx, Widget, WidgetMut, render_text,
};
use xilem::masonry::imaging::Painter;
use xilem::masonry::kurbo::{Affine, Axis, Point, Size};
use xilem::masonry::layout::{AsUnit, LenReq, Length};
use xilem::masonry::parley::editing::Cursor;
use xilem::masonry::parley::{
    FontFamily, FontFamilyName, FontStyle, FontWeight, GenericFamily, Layout, LineHeight,
    OverflowWrap, StyleProperty,
};
use xilem::masonry::{TextAlign, TextAlignOptions};

use crate::{RichParagraph, RichTextStyle, SpanStyle};

const TEXT_BRUSH: BrushIndex = BrushIndex(0);
const LINK_BRUSH: BrushIndex = BrushIndex(1);
const CODE_BRUSH: BrushIndex = BrushIndex(2);

/// Emitted when a link is clicked: the link's target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkClicked(pub String);

pub struct RichTextWidget {
    paragraph: RichParagraph,
    style: RichTextStyle,
    /// Scratch layout for `measure`, which masonry may call for any width
    /// (also without a following `layout`).
    measured: WrappedLayout,
    /// The layout for the widget's actual width; painted and hit-tested.
    placed: WrappedLayout,
}

/// A parley layout plus the wrap width it was built for.
#[derive(Default)]
struct WrappedLayout {
    layout: Layout<BrushIndex>,
    /// `None`: never built (or content changed).
    built_for: Option<Option<f32>>,
}

impl WrappedLayout {
    fn invalidate(&mut self) {
        self.built_for = None;
    }

    fn ensure(
        &mut self,
        ctx_text: (
            &mut xilem::masonry::parley::FontContext,
            &mut xilem::masonry::parley::LayoutContext<BrushIndex>,
        ),
        paragraph: &RichParagraph,
        style: &RichTextStyle,
        max_advance: Option<f32>,
    ) -> &Layout<BrushIndex> {
        if self.built_for != Some(max_advance) {
            build_layout(&mut self.layout, ctx_text, paragraph, style, max_advance);
            self.built_for = Some(max_advance);
        }
        &self.layout
    }
}

impl RichTextWidget {
    pub fn new(paragraph: RichParagraph, style: RichTextStyle) -> Self {
        Self {
            paragraph,
            style,
            measured: WrappedLayout::default(),
            placed: WrappedLayout::default(),
        }
    }

    pub fn set_content(
        this: &mut WidgetMut<'_, Self>,
        paragraph: RichParagraph,
        style: RichTextStyle,
    ) {
        this.widget.paragraph = paragraph;
        this.widget.style = style;
        this.widget.measured.invalidate();
        this.widget.placed.invalidate();
        this.ctx.request_layout();
    }

    /// Number of laid-out lines (for tests and diagnostics).
    pub fn line_count(&self) -> usize {
        self.placed.layout.len()
    }

    /// Index of the link under `pos` (widget coordinates), if any.
    fn link_at(&self, pos: Point) -> Option<&str> {
        if pos.x < 0.0
            || pos.y < 0.0
            || pos.x > self.placed.layout.width() as f64
            || pos.y > self.placed.layout.height() as f64
        {
            return None;
        }
        let index = Cursor::from_point(&self.placed.layout, pos.x as f32, pos.y as f32).index();
        self.paragraph
            .link_at(index)
            .map(|link| link.target.as_str())
    }
}

/// Lay out `paragraph` into `target`, wrapped at `max_advance`.
fn build_layout(
    target: &mut Layout<BrushIndex>,
    ctx_text: (
        &mut xilem::masonry::parley::FontContext,
        &mut xilem::masonry::parley::LayoutContext<BrushIndex>,
    ),
    paragraph: &RichParagraph,
    style: &RichTextStyle,
    max_advance: Option<f32>,
) {
    let (font_ctx, layout_ctx) = ctx_text;
    let text = &paragraph.text;
    let mut builder = layout_ctx.ranged_builder(font_ctx, text, 1.0, true);
    builder.push_default(StyleProperty::FontFamily(FontFamily::List(Cow::Borrowed(
        style.font_stack,
    ))));
    builder.push_default(StyleProperty::FontSize(style.font_size));
    builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
        style.line_height,
    )));
    builder.push_default(StyleProperty::Brush(TEXT_BRUSH));
    // Long URLs break mid-word when nothing else fits, so they don't set a
    // huge minimum width (which would widen the whole page).
    builder.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
    if style.bold_base {
        builder.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
    }
    for (range, span) in &paragraph.styles {
        for property in span_properties(*span, style.font_size) {
            builder.push(property, range.clone());
        }
    }
    for link in &paragraph.links {
        builder.push(StyleProperty::Brush(LINK_BRUSH), link.range.clone());
        builder.push(StyleProperty::Underline(true), link.range.clone());
    }
    builder.build_into(target, text);
    target.break_all_lines(max_advance);
    target.align(max_advance, TextAlign::Start, TextAlignOptions::default());
}

/// The first characters of `text`, to identify a paragraph in logs.
fn preview_text(text: &str) -> String {
    text.chars().take(40).collect()
}

/// Parley properties for one inline span.
fn span_properties(span: SpanStyle, font_size: f32) -> Vec<StyleProperty<'static, BrushIndex>> {
    let mut properties = Vec::new();
    if span.bold {
        properties.push(StyleProperty::FontWeight(FontWeight::BOLD));
    }
    if span.italic {
        properties.push(StyleProperty::FontStyle(FontStyle::Italic));
    }
    if span.strikethrough {
        properties.push(StyleProperty::Strikethrough(true));
    }
    if span.code {
        properties.push(StyleProperty::FontFamily(FontFamily::Single(
            FontFamilyName::Generic(GenericFamily::Monospace),
        )));
        properties.push(StyleProperty::FontSize(font_size * 0.88));
        properties.push(StyleProperty::Brush(CODE_BRUSH));
    }
    properties
}

impl Widget for RichTextWidget {
    type Action = LinkClicked;

    fn accepts_pointer_interaction(&self) -> bool {
        true
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if let PointerEvent::Down(PointerButtonEvent {
            button: None | Some(PointerButton::Primary),
            state,
            ..
        }) = event
        {
            let pos = ctx.local_position(state.position);
            if let Some(target) = self.link_at(pos) {
                ctx.submit_action::<LinkClicked>(LinkClicked(target.to_string()));
                ctx.set_handled();
            }
        }
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, pos: Point) -> CursorIcon {
        if self.link_at(pos).is_some() {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        // Same word-wrap rules as masonry's Label.
        let max_advance = match (axis, len_req) {
            (Axis::Horizontal, LenReq::MinContent) => Some(0.0),
            (Axis::Horizontal, LenReq::MaxContent) => None,
            (Axis::Horizontal, LenReq::FitContent(space)) => Some(space.get() as f32),
            (Axis::Vertical, LenReq::MinContent) => {
                Some(cross_length.map_or(0.0, |l| l.get() as f32))
            }
            (Axis::Vertical, _) => cross_length.map(|l| l.get() as f32),
        };
        let layout = self.measured.ensure(
            ctx.text_contexts(),
            &self.paragraph,
            &self.style,
            max_advance,
        );
        let length = if axis == Axis::Horizontal {
            layout.width()
        } else {
            layout.height()
        };
        length.px()
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let layout = self.placed.ensure(
            ctx.text_contexts(),
            &self.paragraph,
            &self.style,
            Some(size.width as f32),
        );
        match (
            layout.get(0),
            layout.len().checked_sub(1).and_then(|i| layout.get(i)),
        ) {
            (Some(first), Some(last)) => {
                ctx.set_baselines(
                    first.metrics().baseline as f64,
                    last.metrics().baseline as f64,
                );
            }
            _ => ctx.clear_baselines(),
        }
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let brushes = [
            self.style.text_color.into(),
            self.style.link_color.into(),
            self.style.code_color.into(),
        ];
        let box_width = ctx.content_box().width();
        if f64::from(self.placed.layout.width()) > box_width + 0.5 {
            tracing::warn!(
                target: "rich_text",
                text = %preview_text(&self.paragraph.text),
                box_width,
                layout_width = self.placed.layout.width(),
                built_for = ?self.placed.built_for,
                "painted text is wider than its box"
            );
        }
        render_text(
            painter,
            Affine::IDENTITY,
            &self.placed.layout,
            &brushes,
            false,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::Paragraph
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_value(self.paragraph.text.clone());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

#[cfg(test)]
mod tests {
    use masonry::core::{NewWidget, WidgetTag};
    use masonry::properties::types::CrossAxisAlignment;
    use masonry::theme::default_property_set;
    use masonry::widgets::{Flex, Portal};
    use masonry_testing::TestHarness;
    use xilem::Color;

    use super::*;
    use crate::reading_font_stack;

    fn style() -> RichTextStyle {
        RichTextStyle {
            font_stack: reading_font_stack(),
            font_size: 15.0,
            line_height: 1.6,
            bold_base: false,
            text_color: Color::BLACK,
            link_color: Color::BLACK,
            code_color: Color::BLACK,
        }
    }

    /// The Markdown preview's setup: a stretching column in a horizontally
    /// constrained portal. A long paragraph must wrap to the window width.
    #[test]
    fn wraps_inside_a_constrained_portal() {
        let tag = WidgetTag::named("paragraph");
        let mut paragraph = RichParagraph::default();
        paragraph.push(
            &"lorem ipsum dolor sit amet ".repeat(20),
            SpanStyle::default(),
            None,
        );
        let text = NewWidget::new(RichTextWidget::new(paragraph, style())).with_tag(tag);
        let column = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(text);
        let root = NewWidget::new(Portal::new(NewWidget::new(column)).constrain_horizontal(true));
        let harness = TestHarness::create_with_size(default_property_set(), root, (300, 400));
        let widget = harness.get_widget(tag);
        assert!(
            widget.inner().line_count() > 3,
            "wrapped into {} line(s)",
            widget.inner().line_count()
        );
        let width = widget.ctx().border_box().width();
        assert!(width <= 300.5, "width {width}");
    }

    /// sophia's real nesting: portal → padded column → document column →
    /// paragraph.
    #[test]
    fn wraps_in_nested_columns() {
        let tag = WidgetTag::named("nested");
        let mut paragraph = RichParagraph::default();
        paragraph.push(
            &"lorem ipsum dolor sit amet ".repeat(20),
            SpanStyle::default(),
            None,
        );
        let text = NewWidget::new(RichTextWidget::new(paragraph, style())).with_tag(tag);
        let document = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(text);
        let outer = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(NewWidget::new(document));
        let root = NewWidget::new(Portal::new(NewWidget::new(outer)).constrain_horizontal(true));
        let harness = TestHarness::create_with_size(default_property_set(), root, (300, 400));
        let widget = harness.get_widget(tag);
        let width = widget.ctx().border_box().width();
        assert!(
            widget.inner().line_count() > 3 && width <= 300.5,
            "{} line(s), width {width}",
            widget.inner().line_count()
        );
    }

    /// A list item: a row of a fixed bullet and the flexed paragraph,
    /// inside a stretching column.
    #[test]
    fn wraps_as_flexed_child_of_a_row() {
        use masonry::layout::Length;
        use masonry::widgets::SizedBox;
        let tag = WidgetTag::named("item");
        let mut paragraph = RichParagraph::default();
        paragraph.push(
            &"lorem ipsum dolor sit amet ".repeat(20),
            SpanStyle::default(),
            None,
        );
        let text = NewWidget::new(RichTextWidget::new(paragraph, style())).with_tag(tag);
        let bullet = NewWidget::new(SizedBox::empty().width(Length::px(24.0)));
        let row = Flex::row()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_fixed(bullet)
            .with(text, 1.0);
        let column = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(NewWidget::new(row));
        let root = NewWidget::new(Portal::new(NewWidget::new(column)).constrain_horizontal(true));
        let harness = TestHarness::create_with_size(default_property_set(), root, (300, 400));
        let widget = harness.get_widget(tag);
        let width = widget.ctx().border_box().width();
        assert!(
            widget.inner().line_count() > 3 && width <= 276.5,
            "{} line(s), width {width}",
            widget.inner().line_count()
        );
    }

    /// A long URL must not force the page wider than the window.
    #[test]
    fn long_url_wraps_mid_word() {
        let tag = WidgetTag::named("url");
        let mut paragraph = RichParagraph::default();
        let url = format!("https://example.com/{}", "segment".repeat(60));
        paragraph.push(&url, SpanStyle::default(), None);
        let text = NewWidget::new(RichTextWidget::new(paragraph, style())).with_tag(tag);
        let column = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(text);
        let root = NewWidget::new(Portal::new(NewWidget::new(column)).constrain_horizontal(true));
        let harness = TestHarness::create_with_size(default_property_set(), root, (300, 400));
        let widget = harness.get_widget(tag);
        let width = widget.ctx().border_box().width();
        assert!(
            widget.inner().line_count() > 3 && width <= 300.5,
            "{} line(s), width {width}",
            widget.inner().line_count()
        );
    }
}
