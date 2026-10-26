//! Interactive editor overlays — paint passes whose output the user
//! can click on.
//!
//! Where `painting.rs` holds the static render passes (text, cursors,
//! gutter), this submodule holds the *interactive* surfaces — the
//! ones that double as click targets:
//!
//! - `diagnostic_bubble`: Xcode-style error/warning bubbles, including
//!   the expanded popover with quick-fix cards.
//! - `completion_popup`: caret-anchored autocomplete balloon.
//!
//! Each pure paint function returns a list of `(target, Rect)`
//! tuples the widget uses to hit-test pointer events. Keeping these
//! interactive surfaces in their own module isolates them from the
//! rest of `painting.rs`, which can stay focused on stateless line
//! rendering.

pub mod completion_popup;
pub mod diagnostic_bubble;
pub mod quick_help_balloon;
pub mod vcs_deletion_balloon;

pub use completion_popup::{paint_completion_block_popup, paint_completion_popup};
pub use diagnostic_bubble::paint_inline_diagnostic_bubbles;
pub use quick_help_balloon::{
    CodeBlockHighlighter, QuickHelpPaintResult, paint_quick_help_balloon,
};
pub use vcs_deletion_balloon::{VcsBalloonAnchor, VcsBalloonColors, paint_vcs_deletion_balloon};
