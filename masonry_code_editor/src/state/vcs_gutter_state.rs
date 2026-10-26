//! State for the VCS gutter feature on `CodeEditorWidget`: the live
//! HEAD-vs-buffer diff, whether the feature is enabled, the hovered
//! deletion marker, and the removed-text balloon.
//!
//! Extracted from six loose fields (`vcs_diff`/`vcs_gutter_enabled`/
//! `vcs_hovered_deletion`/`vcs_balloon`/`vcs_balloon_rect`/
//! `vcs_insert_btn_rect`) as part of the CodeEditorWidget SOLID
//! rework. Two invariants that used to live as scattered side effects
//! at each field-mutation call site now live once, on the type that
//! owns them:
//! - disabling the gutter clears the hover + balloon (a stale hover
//!   or an open balloon over a marker that's about to disappear)
//! - pushing a new diff clears the hover (the marker it pointed at
//!   may no longer exist at that line)

use masonry::kurbo::Rect;

use code_editor_core::vcs::VcsGutterDiff;

#[derive(Debug, Clone)]
pub struct VcsGutterState {
    /// Live diff (HEAD vs buffer). `None` when there's nothing to show.
    diff: Option<VcsGutterDiff>,
    /// Whether the VCS gutter feature is enabled (Version Control menu toggle).
    enabled: bool,
    /// `after_new_line` of the deletion marker currently under the pointer.
    hovered_deletion: Option<usize>,
    /// Active removed-text balloon: `(after_new_line, removed_text)`.
    balloon: Option<(usize, String)>,
    /// Bounding rect of the last-painted deletion balloon (hit-testing).
    balloon_rect: Rect,
    /// Bounding rect of the balloon's "Insert" button (hit-testing).
    insert_btn_rect: Rect,
}

impl Default for VcsGutterState {
    /// Enabled by default — matches `CodeEditorWidget::new`'s prior
    /// `vcs_gutter_enabled: true` field initializer.
    fn default() -> Self {
        Self {
            diff: None,
            enabled: true,
            hovered_deletion: None,
            balloon: None,
            balloon_rect: Rect::ZERO,
            insert_btn_rect: Rect::ZERO,
        }
    }
}

impl VcsGutterState {
    pub fn diff(&self) -> Option<&VcsGutterDiff> {
        self.diff.as_ref()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn hovered_deletion(&self) -> Option<usize> {
        self.hovered_deletion
    }

    pub fn balloon(&self) -> Option<&(usize, String)> {
        self.balloon.as_ref()
    }

    pub fn balloon_rect(&self) -> Rect {
        self.balloon_rect
    }

    pub fn insert_btn_rect(&self) -> Rect {
        self.insert_btn_rect
    }

    /// Push a fresh diff from the View layer. Clears the hovered
    /// marker — it may no longer exist at that line in the new diff.
    pub fn set_diff(&mut self, diff: Option<VcsGutterDiff>) {
        self.diff = diff;
        self.hovered_deletion = None;
    }

    /// Enable/disable the whole feature. Disabling clears the balloon
    /// and hover state so nothing stale reappears if it's re-enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.balloon = None;
            self.hovered_deletion = None;
        }
    }

    /// Update the hovered marker on pointer move. Returns `true` when
    /// it actually changed, so the caller knows whether to repaint.
    pub fn set_hovered_deletion(&mut self, hovered: Option<usize>) -> bool {
        if hovered != self.hovered_deletion {
            self.hovered_deletion = hovered;
            true
        } else {
            false
        }
    }

    pub fn open_balloon(&mut self, after_new_line: usize, removed_text: String) {
        self.balloon = Some((after_new_line, removed_text));
    }

    pub fn close_balloon(&mut self) {
        self.balloon = None;
    }

    /// Record the balloon's + Insert button's painted geometry. Called
    /// once per paint pass; `Rect::ZERO` for both when nothing painted
    /// (feature disabled or no balloon open) so a stale hit-test rect
    /// from a previous frame never lingers.
    pub fn set_painted_geometry(&mut self, balloon_rect: Rect, insert_btn_rect: Rect) {
        self.balloon_rect = balloon_rect;
        self.insert_btn_rect = insert_btn_rect;
    }
}
