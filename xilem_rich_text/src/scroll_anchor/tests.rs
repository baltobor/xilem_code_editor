// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{NewWidget, WidgetTag};
use masonry::kurbo::Point;
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::theme::default_property_set;
use masonry::widgets::{Flex, Portal, SizedBox};
use masonry_testing::TestHarness;

use super::{ScrollAnchor, ScrollHost};

const BLOCK: f64 = 100.0;
const VIEWPORT: f64 = 300.0;

fn block() -> NewWidget<SizedBox> {
    NewWidget::new(SizedBox::empty().height(Length::px(BLOCK)))
}

/// A portal over 30 blocks of 100 px; blocks 5 and 20 are anchors.
fn harness(
    early: WidgetTag<ScrollAnchor>,
    late: WidgetTag<ScrollAnchor>,
) -> TestHarness<Portal<ScrollHost>> {
    let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    for i in 0..30 {
        column = match i {
            5 => column.with_fixed(NewWidget::new(ScrollAnchor::new(block())).with_tag(early)),
            20 => column.with_fixed(NewWidget::new(ScrollAnchor::new(block())).with_tag(late)),
            _ => column.with_fixed(block()),
        };
    }
    let host = NewWidget::new(ScrollHost::new(NewWidget::new(column)));
    let root = NewWidget::new(Portal::new(host).constrain_horizontal(true));
    TestHarness::create_with_size(default_property_set(), root, (200, VIEWPORT as u32))
}

/// The anchor's top edge within the window (the portal fills the window).
fn top_of(harness: &TestHarness<Portal<ScrollHost>>, anchor: WidgetTag<ScrollAnchor>) -> f64 {
    harness.get_widget(anchor).ctx().to_window(Point::ORIGIN).y
}

#[test]
fn reveal_puts_the_anchor_at_the_top_both_ways() {
    let early = WidgetTag::named("early");
    let late = WidgetTag::named("late");
    let mut harness = harness(early, late);
    let margin = 8.0;

    harness.edit_widget(late, |mut anchor| ScrollAnchor::reveal(&mut anchor));
    assert!(
        harness.root_widget().inner().get_viewport_pos().y > 0.0,
        "scrolled down"
    );
    assert_eq!(top_of(&harness, late), margin, "down to the late anchor");

    harness.edit_widget(early, |mut anchor| ScrollAnchor::reveal(&mut anchor));
    assert_eq!(
        top_of(&harness, early),
        margin,
        "back up to the early anchor"
    );
}
