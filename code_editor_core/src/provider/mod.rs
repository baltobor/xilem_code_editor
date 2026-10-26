// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Attachable data sources. The editor asks; providers answer. Language
//! servers, version control, word lists or AI models implement these
//! traits outside the editor.

mod code_action_provider;
mod completion_provider;
mod edit_prediction_provider;
mod highlight_post_pass;
mod highlight_provider;
mod hover_provider;
mod inlay_hint_provider;
mod provider_future;
mod request;
mod signature_help_provider;

pub use code_action_provider::CodeActionProvider;
pub use completion_provider::{CompletionProvider, CompletionTrigger};
pub use edit_prediction_provider::EditPredictionProvider;
pub use highlight_post_pass::HighlightPostPass;
pub use highlight_provider::HighlightProvider;
pub use hover_provider::HoverProvider;
pub use inlay_hint_provider::InlayHintProvider;
pub use provider_future::ProviderFuture;
pub use request::{DocumentSnapshot, PositionRequest};
pub use signature_help_provider::SignatureHelpProvider;
