// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Language configurations and their lookup.

mod language_config;
mod language_registry;

pub use language_config::LanguageConfig;
pub use language_registry::{LanguageRegistry, registry};
