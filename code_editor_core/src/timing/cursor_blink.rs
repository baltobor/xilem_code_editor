// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! CursorAnimator — manages cursor blink animation state.
//!
//! Extracted from widget.rs to follow Single Responsibility Principle.
//! Handles cursor visibility timing independently of the editor widget.

/// Cursor blink animation state.
///
/// The cursor blinks at a fixed interval, with a timeout after which
/// it stays visible (to avoid distracting blinking when idle).
#[derive(Debug, Clone)]
pub struct CursorAnimator {
    /// Whether the cursor is currently visible in the blink cycle.
    visible: bool,
    /// Accumulated time in the current blink interval (ms).
    prev_interval: u64,
    /// Total elapsed time since last cursor movement (ms).
    elapsed: u64,
}

impl Default for CursorAnimator {
    fn default() -> Self {
        Self::new()
    }
}

impl CursorAnimator {
    /// Blink interval in milliseconds (cursor toggles visibility at half this).
    const BLINK_TIME: u64 = 1000;
    /// Stop blinking after this many milliseconds of no cursor movement.
    const BLINK_TIMEOUT: u64 = 10_000;

    /// Create a new cursor animator with the cursor initially visible.
    pub fn new() -> Self {
        Self {
            visible: true,
            prev_interval: 0,
            elapsed: 0,
        }
    }

    /// Returns whether the cursor should be rendered.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Reset the blink animation (call after cursor movement or text input).
    ///
    /// This makes the cursor immediately visible and restarts the blink timer.
    pub fn reset(&mut self) {
        self.visible = true;
        self.prev_interval = 0;
        self.elapsed = 0;
    }

    /// Advance the animation by the given interval (in nanoseconds).
    ///
    /// Returns `true` if the visibility state changed (caller should repaint).
    /// Also returns whether to request another animation frame.
    pub fn tick(&mut self, interval_ns: u64, is_focused: bool) -> AnimationResult {
        if !is_focused {
            // Ensure cursor is visible when not focused (no blinking)
            let changed = !self.visible;
            self.visible = true;
            return AnimationResult {
                visibility_changed: changed,
                request_frame: false,
            };
        }

        if self.elapsed >= Self::BLINK_TIMEOUT {
            // After timeout, cursor stays visible
            let changed = !self.visible;
            self.visible = true;
            return AnimationResult {
                visibility_changed: changed,
                request_frame: false,
            };
        }

        let interval_ms = interval_ns / 1_000_000;
        self.prev_interval += interval_ms;
        self.elapsed += interval_ms;

        // Wrap around blink interval
        if self.prev_interval >= Self::BLINK_TIME {
            self.prev_interval = self.prev_interval.rem_euclid(Self::BLINK_TIME);
        }

        // Determine visibility: visible in first half of blink cycle
        let should_be_visible = self.prev_interval < Self::BLINK_TIME / 2;
        let changed = self.visible != should_be_visible;
        self.visible = should_be_visible;

        AnimationResult {
            visibility_changed: changed,
            request_frame: true,
        }
    }
}

/// Result of a tick operation.
#[derive(Debug, Clone, Copy)]
pub struct AnimationResult {
    /// True if the cursor visibility changed (caller should repaint).
    pub visibility_changed: bool,
    /// True if another animation frame should be requested.
    pub request_frame: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let animator = CursorAnimator::new();
        assert!(animator.is_visible());
    }

    #[test]
    fn test_reset() {
        let mut animator = CursorAnimator::new();
        animator.visible = false;
        animator.elapsed = 5000;
        animator.reset();
        assert!(animator.is_visible());
        assert_eq!(animator.elapsed, 0);
    }

    #[test]
    fn test_blink_cycle() {
        let mut animator = CursorAnimator::new();

        // Initially visible
        assert!(animator.is_visible());

        // After 400ms, still visible (first half of 1000ms cycle)
        animator.tick(400_000_000, true);
        assert!(animator.is_visible());

        // After another 200ms (total 600ms), should be invisible
        let result = animator.tick(200_000_000, true);
        assert!(!animator.is_visible());
        assert!(result.visibility_changed);

        // After another 500ms (total 1100ms -> 100ms in new cycle), visible again
        let result = animator.tick(500_000_000, true);
        assert!(animator.is_visible());
        assert!(result.visibility_changed);
    }

    #[test]
    fn test_timeout_stops_blinking() {
        let mut animator = CursorAnimator::new();

        // Advance past timeout
        animator.elapsed = CursorAnimator::BLINK_TIMEOUT + 1;
        let result = animator.tick(100_000_000, true);

        // Should stay visible and not request more frames
        assert!(animator.is_visible());
        assert!(!result.request_frame);
    }
}
