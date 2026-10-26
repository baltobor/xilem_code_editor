// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Sits directly inside a portal and scrolls it to revealed anchors.

use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, ActionCtx, ChildrenIds, ErasedAction, LayoutCtx, MeasureCtx, NewWidget, NoAction,
    PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use xilem::masonry::imaging::Painter;
use xilem::masonry::kurbo::{Axis, Rect, Size};
use xilem::masonry::layout::{LenReq, Length};

use super::RevealAnchor;
use super::single_child::{layout_child, measure_child};

/// Space kept above a revealed anchor.
const TOP_MARGIN: f64 = 8.0;
/// Longer than any viewport; see `on_action`.
const FAR: f64 = 1.0e7;

pub struct ScrollHost {
    child: WidgetPod<dyn Widget>,
}

impl ScrollHost {
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: child.erased().to_pod(),
        }
    }

    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

impl Widget for ScrollHost {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn on_action(
        &mut self,
        ctx: &mut ActionCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        action: &ErasedAction,
        _source: WidgetId,
    ) {
        let Some(reveal) = (**action).downcast_ref::<RevealAnchor>() else {
            return;
        };
        ctx.set_handled();
        let top = ctx.to_local(reveal.window_top).y;
        tracing::debug!(target: "rich_text", ?reveal, top, "host: scrolling to anchor");
        // A portal pans as little as possible to show a rect. The first
        // rect ends at the anchor's bottom, so the anchor comes into view
        // (from below or above); the second starts at the anchor and is
        // taller than the viewport, so the portal then aligns its top edge.
        ctx.request_scroll_to(Rect::new(0.0, top - FAR, 1.0, top + reveal.height));
        ctx.request_scroll_to(Rect::new(
            0.0,
            top - TOP_MARGIN,
            1.0,
            top - TOP_MARGIN + FAR,
        ));
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
