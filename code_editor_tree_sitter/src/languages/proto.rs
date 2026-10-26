// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Protobuf language configuration.
//!
//! Highlighting is provided by `tree-sitter-proto`. The crate
//! ships its highlights query as a `.scm` file in its repo but
//! does not re-export it as a Rust constant, so we vendor the
//! contents in `proto_highlights.scm` and `include_str!` it —
//! the same pattern `just.rs` uses.

use crate::capture_category::HIGHLIGHT_NAMES;
use crate::registry::LanguageConfig;
use tree_sitter_highlight::HighlightConfiguration;

pub static PROTO: ProtoLanguage = ProtoLanguage;

pub struct ProtoLanguage;

const HIGHLIGHTS_QUERY: &str = include_str!("proto_highlights.scm");

impl LanguageConfig for ProtoLanguage {
    fn language_id(&self) -> &'static str {
        "proto"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["proto"]
    }

    fn highlight_config(&self) -> Option<HighlightConfiguration> {
        let mut cfg = HighlightConfiguration::new(
            tree_sitter_proto::LANGUAGE.into(),
            "proto",
            HIGHLIGHTS_QUERY,
            "",
            "",
        )
        .ok()?;
        cfg.configure(HIGHLIGHT_NAMES);
        Some(cfg)
    }
}
