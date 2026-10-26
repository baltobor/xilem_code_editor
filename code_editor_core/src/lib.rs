// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Framework-independent model of a code editor: document storage,
//! selections, cursor movement and editing, snippets, completion policy,
//! popup timing, and the data-provider traits that external sources
//! (language servers, version control, debuggers) implement.
//!
//! This crate has no UI-framework dependency; the widget layer lives in
//! `masonry_code_editor` and the view layer in `xilem_code_editor`.

pub mod completion;
pub mod config;
pub mod cursor;
pub mod document;
pub mod edit;
pub mod highlight;
pub mod layout;
pub mod model;
pub mod provider;
pub mod selection;
pub mod snippet;
pub mod timing;
pub mod vcs;
