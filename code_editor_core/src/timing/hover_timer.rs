// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::time::Instant;

use super::PopupTiming;

/// What the hover timer asks the editor to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverEffect {
    /// Ask the hover provider about this byte offset.
    Request(usize),
    /// Close the popover.
    Hide,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Idle,
    /// Waiting to request and/or show the popover for `anchor`.
    Waiting {
        anchor: usize,
        request_at: Option<Instant>,
        show_at: Instant,
        has_content: bool,
        from_keyboard: bool,
    },
    /// Popover visible.
    Shown {
        anchor: usize,
        hide_at: Option<Instant>,
        closest: f64,
        keyboard_grace: bool,
    },
}

/// Hover popover timing, modeled on zed's hover behavior:
///
/// - the provider is asked after half the delay, the popover appears after
///   the full delay (or when the content arrives, if that is later);
/// - resting on the same anchor keeps it open without re-requesting;
/// - leaving starts a grace period that restarts while the pointer moves
///   *closer* to the popover, so it can be reached;
/// - typing, scrolling, selection changes, blur and Esc hide immediately
///   ([`dismiss`](Self::dismiss));
/// - a keyboard-opened hover survives pointer moves until the next click.
#[derive(Debug, Clone)]
pub struct HoverTimer {
    timing: PopupTiming,
    state: State,
}

impl HoverTimer {
    pub fn new(timing: PopupTiming) -> Self {
        Self {
            timing,
            state: State::Idle,
        }
    }

    /// The pointer is over the token at `anchor` (`None`: over nothing
    /// hoverable).
    pub fn hover_at(&mut self, anchor: Option<usize>, now: Instant) -> Option<HoverEffect> {
        match (anchor, self.state) {
            (
                _,
                State::Shown {
                    keyboard_grace: true,
                    ..
                },
            ) => None,
            (Some(a), State::Shown { anchor, .. }) if a == anchor => {
                self.cancel_hide();
                None
            }
            (Some(a), State::Waiting { anchor, .. }) if a == anchor => None,
            (Some(a), previous) => {
                self.start_waiting(a, now);
                matches!(previous, State::Shown { .. }).then_some(HoverEffect::Hide)
            }
            (None, State::Waiting { .. }) => {
                self.state = State::Idle;
                None
            }
            (None, State::Idle)
            | (
                None,
                State::Shown {
                    hide_at: Some(_), ..
                },
            ) => None,
            (None, State::Shown { anchor, .. }) if self.timing.hover_sticky => {
                self.state = State::Shown {
                    anchor,
                    hide_at: Some(now + self.timing.hover_hide_delay),
                    closest: f64::INFINITY,
                    keyboard_grace: false,
                };
                None
            }
            (None, State::Shown { .. }) => self.dismiss(),
        }
    }

    /// The pointer is outside the anchor, `distance` px from the popover.
    /// Moving closer restarts the grace period.
    pub fn approach(&mut self, distance: f64, now: Instant) {
        if let State::Shown {
            anchor,
            hide_at: Some(_),
            closest,
            keyboard_grace,
        } = self.state
        {
            if distance < closest {
                self.state = State::Shown {
                    anchor,
                    hide_at: Some(now + self.timing.hover_hide_delay),
                    closest: distance,
                    keyboard_grace,
                };
            }
        }
    }

    /// The pointer is over the popover itself: keep it open.
    pub fn over_popover(&mut self) {
        self.cancel_hide();
    }

    /// Open immediately for the caret position (keyboard shortcut).
    pub fn open_from_keyboard(&mut self, anchor: usize, now: Instant) -> HoverEffect {
        self.state = State::Waiting {
            anchor,
            request_at: None,
            show_at: now,
            has_content: false,
            from_keyboard: true,
        };
        HoverEffect::Request(anchor)
    }

    /// Mouse button pressed: ends a keyboard hover's grace.
    pub fn pointer_down(&mut self) -> Option<HoverEffect> {
        match self.state {
            State::Shown {
                keyboard_grace: true,
                ..
            } => self.dismiss(),
            _ => None,
        }
    }

    /// Typing, scrolling, selection change, blur or Esc: hide now.
    pub fn dismiss(&mut self) -> Option<HoverEffect> {
        let was_shown = matches!(self.state, State::Shown { .. });
        self.state = State::Idle;
        was_shown.then_some(HoverEffect::Hide)
    }

    /// The provider answered for `anchor` (`has_content: false`: nothing
    /// to show).
    pub fn content_arrived(&mut self, anchor: usize, has_content: bool, now: Instant) {
        let State::Waiting {
            anchor: a,
            request_at,
            show_at,
            from_keyboard,
            ..
        } = self.state
        else {
            return;
        };
        if a != anchor {
            return;
        }
        if !has_content {
            self.state = State::Idle;
            return;
        }
        self.state = State::Waiting {
            anchor,
            request_at,
            show_at,
            has_content: true,
            from_keyboard,
        };
        self.promote_if_due(now);
    }

    /// Advance time. Call whenever [`next_deadline`](Self::next_deadline)
    /// has passed.
    pub fn tick(&mut self, now: Instant) -> Option<HoverEffect> {
        match self.state {
            State::Waiting {
                anchor,
                request_at: Some(at),
                show_at,
                has_content,
                from_keyboard,
            } if now >= at => {
                self.state = State::Waiting {
                    anchor,
                    request_at: None,
                    show_at,
                    has_content,
                    from_keyboard,
                };
                Some(HoverEffect::Request(anchor))
            }
            State::Waiting { .. } => {
                self.promote_if_due(now);
                None
            }
            State::Shown {
                hide_at: Some(at), ..
            } if now >= at => self.dismiss(),
            _ => None,
        }
    }

    /// The anchor whose popover is visible.
    pub fn visible_anchor(&self) -> Option<usize> {
        match self.state {
            State::Shown { anchor, .. } => Some(anchor),
            _ => None,
        }
    }

    /// When `tick` next needs to run, if anything is pending.
    pub fn next_deadline(&self) -> Option<Instant> {
        match self.state {
            State::Idle => None,
            State::Waiting {
                request_at,
                show_at,
                ..
            } => Some(request_at.map_or(show_at, |r| r.min(show_at))),
            State::Shown { hide_at, .. } => hide_at,
        }
    }

    fn start_waiting(&mut self, anchor: usize, now: Instant) {
        let lead = self.timing.hover_delay / 2;
        self.state = State::Waiting {
            anchor,
            request_at: Some(now + (self.timing.hover_delay - lead)),
            show_at: now + self.timing.hover_delay,
            has_content: false,
            from_keyboard: false,
        };
    }

    fn promote_if_due(&mut self, now: Instant) {
        if let State::Waiting {
            anchor,
            show_at,
            has_content: true,
            from_keyboard,
            ..
        } = self.state
        {
            if now >= show_at {
                self.state = State::Shown {
                    anchor,
                    hide_at: None,
                    closest: f64::INFINITY,
                    keyboard_grace: from_keyboard,
                };
            }
        }
    }

    fn cancel_hide(&mut self) {
        if let State::Shown {
            anchor,
            keyboard_grace,
            ..
        } = self.state
        {
            self.state = State::Shown {
                anchor,
                hide_at: None,
                closest: f64::INFINITY,
                keyboard_grace,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn shown_at(timer: &mut HoverTimer, t0: Instant, anchor: usize) {
        timer.hover_at(Some(anchor), t0);
        assert_eq!(timer.tick(t0 + ms(150)), Some(HoverEffect::Request(anchor)));
        timer.content_arrived(anchor, true, t0 + ms(200));
        timer.tick(t0 + ms(300));
        assert_eq!(timer.visible_anchor(), Some(anchor));
    }

    #[test]
    fn requests_at_half_delay_and_shows_at_full_delay() {
        let t0 = Instant::now();
        let mut timer = HoverTimer::new(PopupTiming::default());
        timer.hover_at(Some(7), t0);
        assert_eq!(timer.tick(t0 + ms(149)), None);
        assert_eq!(timer.tick(t0 + ms(150)), Some(HoverEffect::Request(7)));
        timer.content_arrived(7, true, t0 + ms(200));
        assert_eq!(
            timer.visible_anchor(),
            None,
            "content early, still waits for 300 ms"
        );
        timer.tick(t0 + ms(300));
        assert_eq!(timer.visible_anchor(), Some(7));
    }

    #[test]
    fn late_content_shows_on_arrival() {
        let t0 = Instant::now();
        let mut timer = HoverTimer::new(PopupTiming::default());
        timer.hover_at(Some(1), t0);
        timer.tick(t0 + ms(150));
        timer.tick(t0 + ms(400));
        assert_eq!(timer.visible_anchor(), None);
        timer.content_arrived(1, true, t0 + ms(450));
        assert_eq!(timer.visible_anchor(), Some(1));
    }

    #[test]
    fn moving_to_another_token_restarts_the_delay() {
        let t0 = Instant::now();
        let mut timer = HoverTimer::new(PopupTiming::default());
        timer.hover_at(Some(1), t0);
        timer.hover_at(Some(2), t0 + ms(100));
        assert_eq!(
            timer.tick(t0 + ms(150)),
            None,
            "the first request was cancelled"
        );
        assert_eq!(timer.tick(t0 + ms(250)), Some(HoverEffect::Request(2)));
    }

    #[test]
    fn leaving_hides_after_grace_period() {
        let t0 = Instant::now();
        let mut timer = HoverTimer::new(PopupTiming::default());
        shown_at(&mut timer, t0, 3);
        timer.hover_at(None, t0 + ms(1000));
        assert_eq!(timer.tick(t0 + ms(1299)), None);
        assert_eq!(timer.tick(t0 + ms(1300)), Some(HoverEffect::Hide));
    }

    #[test]
    fn approaching_the_popover_extends_the_grace_period() {
        let t0 = Instant::now();
        let mut timer = HoverTimer::new(PopupTiming::default());
        shown_at(&mut timer, t0, 3);
        timer.hover_at(None, t0 + ms(1000));
        timer.approach(40.0, t0 + ms(1200));
        timer.approach(20.0, t0 + ms(1400));
        assert_eq!(timer.tick(t0 + ms(1600)), None);
        timer.approach(30.0, t0 + ms(1650)); // moving away: no restart
        assert_eq!(timer.tick(t0 + ms(1700)), Some(HoverEffect::Hide));
    }

    #[test]
    fn reaching_the_popover_keeps_it_open() {
        let t0 = Instant::now();
        let mut timer = HoverTimer::new(PopupTiming::default());
        shown_at(&mut timer, t0, 3);
        timer.hover_at(None, t0 + ms(1000));
        timer.over_popover();
        assert_eq!(timer.tick(t0 + ms(5000)), None);
        assert_eq!(timer.visible_anchor(), Some(3));
    }

    #[test]
    fn typing_dismisses_immediately() {
        let t0 = Instant::now();
        let mut timer = HoverTimer::new(PopupTiming::default());
        shown_at(&mut timer, t0, 3);
        assert_eq!(timer.dismiss(), Some(HoverEffect::Hide));
        assert_eq!(timer.visible_anchor(), None);
    }

    #[test]
    fn non_sticky_hides_immediately_on_leave() {
        let t0 = Instant::now();
        let mut timer = HoverTimer::new(PopupTiming {
            hover_sticky: false,
            ..Default::default()
        });
        shown_at(&mut timer, t0, 3);
        assert_eq!(timer.hover_at(None, t0 + ms(1000)), Some(HoverEffect::Hide));
    }

    #[test]
    fn keyboard_hover_survives_pointer_until_click() {
        let t0 = Instant::now();
        let mut timer = HoverTimer::new(PopupTiming::default());
        assert_eq!(timer.open_from_keyboard(9, t0), HoverEffect::Request(9));
        timer.content_arrived(9, true, t0 + ms(20));
        assert_eq!(timer.visible_anchor(), Some(9));
        assert_eq!(timer.hover_at(None, t0 + ms(100)), None);
        assert_eq!(timer.tick(t0 + ms(2000)), None);
        assert_eq!(timer.pointer_down(), Some(HoverEffect::Hide));
    }

    #[test]
    fn empty_answer_shows_nothing() {
        let t0 = Instant::now();
        let mut timer = HoverTimer::new(PopupTiming::default());
        timer.hover_at(Some(5), t0);
        timer.tick(t0 + ms(150));
        timer.content_arrived(5, false, t0 + ms(160));
        timer.tick(t0 + ms(300));
        assert_eq!(timer.visible_anchor(), None);
        assert_eq!(timer.next_deadline(), None);
    }
}
