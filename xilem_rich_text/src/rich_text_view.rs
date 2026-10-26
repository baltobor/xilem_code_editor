// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Xilem view for [`RichTextWidget`].

use xilem::core::{MessageCtx, MessageResult, Mut, View, ViewMarker};
use xilem::{Pod, ViewCtx};

use crate::rich_text_widget::{LinkClicked, RichTextWidget};
use crate::{RichParagraph, RichTextStyle};

type LinkHandler<State> = Box<dyn Fn(&mut State, &str) + Send + Sync + 'static>;

/// A wrapping paragraph with inline styles; `on_link` receives the
/// target of a clicked link.
pub fn rich_text<State, F>(
    paragraph: RichParagraph,
    style: RichTextStyle,
    on_link: F,
) -> RichText<State>
where
    F: Fn(&mut State, &str) + Send + Sync + 'static,
{
    RichText {
        paragraph,
        style,
        on_link: Box::new(on_link),
    }
}

pub struct RichText<State> {
    paragraph: RichParagraph,
    style: RichTextStyle,
    on_link: LinkHandler<State>,
}

impl<State> ViewMarker for RichText<State> {}

impl<State: 'static> View<State, (), ViewCtx> for RichText<State> {
    type Element = Pod<RichTextWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let widget = RichTextWidget::new(self.paragraph.clone(), self.style.clone());
        (ctx.with_action_widget(|ctx| ctx.create_pod(widget)), ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        _: &mut (),
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        if self.paragraph != prev.paragraph || self.style != prev.style {
            RichTextWidget::set_content(&mut element, self.paragraph.clone(), self.style.clone());
        }
    }

    fn teardown(&self, _: &mut (), _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        _: &mut (),
        message: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<()> {
        match message.take_message::<LinkClicked>() {
            Some(link) => {
                (self.on_link)(app_state, &link.0);
                MessageResult::Action(())
            }
            None => MessageResult::Stale,
        }
    }
}
