// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Layout shared by the single-child wrappers: the child takes the
//! wrapper's whole box.

use xilem::masonry::core::{LayoutCtx, MeasureCtx, Widget, WidgetPod};
use xilem::masonry::kurbo::{Axis, Point, Size};
use xilem::masonry::layout::Length;

pub(super) fn measure_child(
    ctx: &mut MeasureCtx<'_>,
    child: &mut WidgetPod<dyn Widget>,
    axis: Axis,
    cross_length: Option<Length>,
) -> Length {
    ctx.redirect_measurement(child, axis, cross_length)
}

pub(super) fn layout_child(ctx: &mut LayoutCtx<'_>, child: &mut WidgetPod<dyn Widget>, size: Size) {
    ctx.run_layout(child, size);
    ctx.place_child(child, Point::ORIGIN);
    ctx.derive_baselines(child);
}
