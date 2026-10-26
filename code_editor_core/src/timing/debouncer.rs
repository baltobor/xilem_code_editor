// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, Instant};

/// Fires once `delay` after the most recent `poke`.
#[derive(Debug, Clone)]
pub struct Debouncer {
    delay: Duration,
    due: Option<Instant>,
}

impl Debouncer {
    pub fn new(delay: Duration) -> Self {
        Self { delay, due: None }
    }

    /// Restart the countdown.
    pub fn poke(&mut self, now: Instant) {
        self.due = Some(now + self.delay);
    }

    pub fn cancel(&mut self) {
        self.due = None;
    }

    /// `true` exactly once when the countdown has elapsed.
    pub fn fire(&mut self, now: Instant) -> bool {
        match self.due {
            Some(due) if now >= due => {
                self.due = None;
                true
            }
            _ => false,
        }
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_after_quiet_period_only() {
        let t0 = Instant::now();
        let ms = Duration::from_millis;
        let mut d = Debouncer::new(ms(75));
        d.poke(t0);
        d.poke(t0 + ms(50));
        assert!(!d.fire(t0 + ms(100)));
        assert!(d.fire(t0 + ms(125)));
        assert!(!d.fire(t0 + ms(200)));
    }
}
