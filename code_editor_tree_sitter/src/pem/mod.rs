// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! PEM / OpenSSL text highlighting via regular expressions (there is no
//! tree-sitter grammar for it).

mod certificate_highlight;

pub use certificate_highlight::{certificate_highlights, certificate_spans};
