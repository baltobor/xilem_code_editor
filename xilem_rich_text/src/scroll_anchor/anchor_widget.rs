// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Wraps a block that can be scrolled to the top of its portal.

use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, ChildrenIds, ComposeCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesRef,
    RegisterCtx, Widget, WidgetMut, WidgetPod,
};
use xilem::masonry::imaging::Painter;
use xilem::masonry::kurbo::{Axis, Point, Size};
use xilem::masonry::layout::{LenReq, Length};

use super::single_child::{layout_child, measure_child};

/// Posted by a `ScrollAnchor` asked to reveal itself; handled by the
/// enclosing `ScrollHost`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RevealAnchor {
    /// The anchor's top-left in window coordinates.
    pub window_top: Point,
    pub height: f64,
}

pub struct ScrollAnchor {
    child: WidgetPod<dyn Widget>,
    reveal_pending: bool,
}

impl ScrollAnchor {
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: child.erased().to_pod(),
            reveal_pending: false,
        }
    }

    /// Reveal as soon as the anchor is laid out.
    pub fn with_reveal(mut self) -> Self {
        self.reveal_pending = true;
        self
    }

    /// Scroll the enclosing portal so this anchor sits at its top.
    pub fn reveal(this: &mut WidgetMut<'_, Self>) {
        tracing::debug!(target: "rich_text", "anchor: reveal requested");
        this.widget.reveal_pending = true;
        this.ctx.request_compose();
    }

    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

impl Widget for ScrollAnchor {
    type Action = RevealAnchor;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        measure_child(ctx, &mut self.child, axis, cross_length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        layout_child(ctx, &mut self.child, size);
    }

    // Runs after layout with current transforms, so the position is final.
    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {
        if !std::mem::take(&mut self.reveal_pending) {
            return;
        }
        let bounds = ctx.content_box();
        tracing::debug!(
            target: "rich_text",
            window_top = ?ctx.to_window(bounds.origin()),
            "anchor: compose submits reveal"
        );
        ctx.submit_action::<RevealAnchor>(RevealAnchor {
            window_top: ctx.to_window(bounds.origin()),
            height: bounds.height(),
        });
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
