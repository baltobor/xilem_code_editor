// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Neutral data types exchanged with the host and providers, independent
//! of LSP, git or debugger libraries.

mod bracket_mode;
mod code_action;
mod diagnostic;
mod edit_prediction;
mod goto_alignment;
mod hover;
mod hover_info;
mod inlay_hint;
mod section_marker;
mod signature_help;

pub use bracket_mode::BracketMode;
pub use code_action::CodeAction;
pub use diagnostic::{Diagnostic, DiagnosticSeverity};
pub use edit_prediction::EditPrediction;
pub use goto_alignment::GotoAlignment;
pub use hover::Hover;
pub use hover_info::HoverInfo;
pub use inlay_hint::{InlayHint, InlayHintKind};
pub use section_marker::{SectionMarker, parse_section_markers};
pub use signature_help::SignatureHelp;
