//! ViewportState — manages viewport synchronization and scroll state.
//!
//! Extracted from widget.rs to follow Single Responsibility Principle.
//! Tracks when the viewport needs updates due to content or dimension changes.

/// Viewport synchronization state.
///
/// Tracks various conditions that require re-layout or scroll adjustments:
/// - Document revision changes
/// - Content width changes (for horizontal scrolling)
/// - Cursor navigation requiring scroll-to-view
#[derive(Debug, Clone)]
pub struct ViewportState {
    /// Document revision when we last synced to the view layer.
    /// Used to detect when we need to emit ContentChanged.
    pub last_synced_revision: u64,

    /// The last width we laid out with (for detecting resize).
    pub last_available_width: Option<f32>,

    /// Maximum content width from cached line layouts.
    /// Used in layout() to report the correct size so Portal enables horizontal scrolling.
    pub max_content_width: f64,

    /// Set to true in paint() when newly-built layouts produce a wider max width
    /// than what layout() last reported. Checked in on_anim_frame() to trigger re-layout.
    pub needs_width_update: bool,

    /// Set to true when cursor moves from external navigation (goto_line).
    /// Checked in on_anim_frame() to trigger scroll to cursor.
    pub needs_scroll_to_cursor: bool,

    /// Last known total document height. Used to detect when newly-computed
    /// line layouts change the height and Portal needs a re-layout.
    pub last_total_height: f64,

    /// Last known scroll Y position. Used to detect scroll changes
    /// and emit ScrollChanged action for external gutter sync.
    pub last_scroll_y: f64,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewportState {
    /// Create a new viewport state with default values.
    pub fn new() -> Self {
        Self {
            last_synced_revision: 1,
            last_available_width: None,
            max_content_width: 0.0,
            needs_width_update: false,
            needs_scroll_to_cursor: false,
            last_total_height: 0.0,
            last_scroll_y: 0.0,
        }
    }

    /// Check if a re-layout is needed due to width changes.
    ///
    /// Returns true and clears the flag if an update is needed.
    pub fn check_and_clear_width_update(&mut self) -> bool {
        if self.needs_width_update {
            self.needs_width_update = false;
            true
        } else {
            false
        }
    }

    /// Check if we need to scroll to cursor position.
    ///
    /// Returns true and clears the flag if scrolling is needed.
    pub fn check_and_clear_scroll_to_cursor(&mut self) -> bool {
        if self.needs_scroll_to_cursor {
            self.needs_scroll_to_cursor = false;
            true
        } else {
            false
        }
    }

    /// Mark that content has changed and sync the revision.
    pub fn sync_revision(&mut self, revision: u64) {
        self.last_synced_revision = revision;
    }

    /// Check if the document revision has changed since last sync.
    pub fn is_revision_stale(&self, current_revision: u64) -> bool {
        self.last_synced_revision != current_revision
    }

    /// Update the maximum content width and flag for re-layout if it increased.
    ///
    /// Returns true if the width increased significantly (> 1 pixel).
    pub fn update_max_width(&mut self, new_width: f64) -> bool {
        if new_width > self.max_content_width + 1.0 {
            self.needs_width_update = true;
            true
        } else {
            false
        }
    }

    /// Set the maximum content width directly (used during layout pass).
    pub fn set_max_content_width(&mut self, width: f64) {
        self.max_content_width = width;
    }

    /// Update the total height and flag for re-layout if it changed.
    ///
    /// Returns true if the height changed significantly (> 0.5 pixels).
    pub fn update_total_height(&mut self, new_height: f64) -> bool {
        let new_rounded = (new_height * 10.0).round() / 10.0;
        let old_rounded = (self.last_total_height * 10.0).round() / 10.0;

        if (new_rounded - old_rounded).abs() > 0.5 {
            self.last_total_height = new_height;
            self.needs_width_update = true; // Piggyback on existing re-layout flag
            true
        } else {
            false
        }
    }

    /// Store the available width from layout.
    pub fn set_available_width(&mut self, width: f32) {
        self.last_available_width = Some(width);
    }

    /// Request scrolling to cursor position.
    pub fn request_scroll_to_cursor(&mut self) {
        self.needs_scroll_to_cursor = true;
    }

    /// Reset content-related state (used when document is replaced).
    pub fn reset_content(&mut self) {
        self.max_content_width = 0.0;
        self.needs_width_update = false;
        self.last_total_height = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let state = ViewportState::new();
        assert_eq!(state.last_synced_revision, 1);
        assert_eq!(state.max_content_width, 0.0);
        assert!(!state.needs_width_update);
        assert!(!state.needs_scroll_to_cursor);
    }

    #[test]
    fn test_width_update_detection() {
        let mut state = ViewportState::new();
        state.max_content_width = 100.0;

        // Small change should not trigger update
        assert!(!state.update_max_width(100.5));
        assert!(!state.needs_width_update);

        // Larger change should trigger update
        assert!(state.update_max_width(102.0));
        assert!(state.needs_width_update);
    }

    #[test]
    fn test_check_and_clear() {
        let mut state = ViewportState::new();
        state.needs_width_update = true;
        state.needs_scroll_to_cursor = true;

        assert!(state.check_and_clear_width_update());
        assert!(!state.needs_width_update);

        assert!(state.check_and_clear_scroll_to_cursor());
        assert!(!state.needs_scroll_to_cursor);

        // Second call should return false
        assert!(!state.check_and_clear_width_update());
        assert!(!state.check_and_clear_scroll_to_cursor());
    }

    #[test]
    fn test_revision_tracking() {
        let mut state = ViewportState::new();
        assert!(!state.is_revision_stale(1));
        assert!(state.is_revision_stale(2));

        state.sync_revision(5);
        assert!(!state.is_revision_stale(5));
        assert!(state.is_revision_stale(6));
    }
}
