// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! One configuration per language, each behind its `lang-*` feature.

#[cfg(feature = "lang-c")]
pub mod c;
#[cfg(feature = "lang-pem")]
pub mod certificate;
#[cfg(feature = "lang-cpp")]
pub mod cpp;
#[cfg(feature = "lang-json")]
pub mod json;
#[cfg(feature = "lang-just")]
pub mod just;
#[cfg(feature = "lang-markdown")]
pub mod markdown;
#[cfg(feature = "lang-markdown")]
pub mod markdown_inline;
pub mod plain;
#[cfg(feature = "lang-proto")]
pub mod proto;
#[cfg(feature = "lang-rust")]
pub mod rust;
#[cfg(feature = "lang-toml")]
pub mod toml;
#[cfg(feature = "lang-yaml")]
pub mod yaml;
#[cfg(feature = "lang-zig")]
pub mod zig;
