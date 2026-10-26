// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Xilem view for [`ScrollHost`].

use std::marker::PhantomData;

use xilem::core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};
use xilem::{Pod, ViewCtx, WidgetView};

use super::ScrollHost;

const CHILD_ID: ViewId = ViewId::new(0x5c_a7_c4_02);

/// Put directly inside a portal: scrolls it to the top of any
/// [`scroll_anchor`](super::scroll_anchor) in `child` that reveals itself.
pub fn scroll_host<State, Action, V>(child: V) -> ScrollHostView<V, State, Action>
where
    V: WidgetView<State, Action>,
    State: 'static,
{
    ScrollHostView {
        child,
        phantom: PhantomData,
    }
}

#[must_use = "View values do nothing unless provided to Xilem."]
pub struct ScrollHostView<V, State, Action = ()> {
    child: V,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<V, State, Action> ViewMarker for ScrollHostView<V, State, Action> {}

impl<V, State, Action> View<State, Action, ViewCtx> for ScrollHostView<V, State, Action>
where
    V: WidgetView<State, Action>,
    State: 'static,
    Action: 'static,
{
    type Element = Pod<ScrollHost>;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx, state: &mut State) -> (Self::Element, Self::ViewState) {
        let (child, child_state) = ctx.with_id(CHILD_ID, |ctx| self.child.build(ctx, state));
        (
            ctx.create_pod(ScrollHost::new(child.new_widget)),
            child_state,
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut State,
    ) {
        ctx.with_id(CHILD_ID, |ctx| {
            self.child.rebuild(
                &prev.child,
                view_state,
                ctx,
                ScrollHost::child_mut(&mut element).downcast(),
                state,
            );
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(CHILD_ID, |ctx| {
            self.child.teardown(
                view_state,
                ctx,
                ScrollHost::child_mut(&mut element).downcast(),
            );
        });
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut State,
    ) -> MessageResult<Action> {
        match message.take_first() {
            Some(CHILD_ID) => self.child.message(
                view_state,
                message,
                ScrollHost::child_mut(&mut element).downcast(),
                state,
            ),
            _ => MessageResult::Stale,
        }
    }
}
