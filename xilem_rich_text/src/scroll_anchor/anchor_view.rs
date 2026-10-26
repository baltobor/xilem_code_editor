// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Xilem view for [`ScrollAnchor`].

use std::marker::PhantomData;

use xilem::core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};
use xilem::{Pod, ViewCtx, WidgetView};

use super::ScrollAnchor;

const CHILD_ID: ViewId = ViewId::new(0x5c_a7_c4_01);

/// Wraps `child` so a [`scroll_host`](super::scroll_host) can scroll it to
/// the top. Each new `Some(token)` (one per jump request) reveals it once.
pub fn scroll_anchor<State, Action, V>(
    child: V,
    reveal: Option<u64>,
) -> ScrollAnchorView<V, State, Action>
where
    V: WidgetView<State, Action>,
    State: 'static,
{
    ScrollAnchorView {
        child,
        reveal,
        phantom: PhantomData,
    }
}

#[must_use = "View values do nothing unless provided to Xilem."]
pub struct ScrollAnchorView<V, State, Action = ()> {
    child: V,
    reveal: Option<u64>,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<V, State, Action> ViewMarker for ScrollAnchorView<V, State, Action> {}

impl<V, State, Action> View<State, Action, ViewCtx> for ScrollAnchorView<V, State, Action>
where
    V: WidgetView<State, Action>,
    State: 'static,
    Action: 'static,
{
    type Element = Pod<ScrollAnchor>;
    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx, state: &mut State) -> (Self::Element, Self::ViewState) {
        let (child, child_state) = ctx.with_id(CHILD_ID, |ctx| self.child.build(ctx, state));
        let widget = ScrollAnchor::new(child.new_widget);
        let widget = if self.reveal.is_some() {
            widget.with_reveal()
        } else {
            widget
        };
        (ctx.create_pod(widget), child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut State,
    ) {
        if self.reveal.is_some() && self.reveal != prev.reveal {
            ScrollAnchor::reveal(&mut element);
        }
        ctx.with_id(CHILD_ID, |ctx| {
            self.child.rebuild(
                &prev.child,
                view_state,
                ctx,
                ScrollAnchor::child_mut(&mut element).downcast(),
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
                ScrollAnchor::child_mut(&mut element).downcast(),
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
                ScrollAnchor::child_mut(&mut element).downcast(),
                state,
            ),
            _ => MessageResult::Stale,
        }
    }
}
