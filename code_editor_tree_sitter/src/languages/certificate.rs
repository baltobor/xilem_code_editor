// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Certificate / PEM language configuration.
//!
//! Covers two file shapes that come up when working with TLS / PKI:
//!
//! 1. **PEM-encoded files** (`.pem`, `.crt`, `.cer`, `.key`,
//!    `.csr`) — armoured base64 with `-----BEGIN …-----` /
//!    `-----END …-----` markers. The body is opaque base64; the
//!    markers carry all the structural information.
//! 2. **OpenSSL `x509 -text` output** — human-readable dump with
//!    nested `Certificate:` / `Data:` / `Issuer:` / `Validity` /
//!    `Subject:` blocks, hex-coded fields (`aa:bb:cc:…`), and
//!    distinguished-name attributes (`CN = example.com`).
//!
//! There's no published tree-sitter grammar for either format, and
//! writing one for cosmetic highlighting alone would be heavy. Instead
//! we ship a lightweight regex-based highlighter as a side channel:
//! the `LanguageConfig` impl returns `None` for `highlight_config`
//! (which tells the tree-sitter pipeline to skip), and
//! `editor::highlight::SyntaxHighlighter::highlight` dispatches to
//! `certificate_highlights` when the active `Language` is
//! `Certificate`. The split lets future tree-sitter languages keep
//! using the existing pipeline unchanged.

use crate::registry::LanguageConfig;
use tree_sitter_highlight::HighlightConfiguration;

pub static CERTIFICATE: CertificateLanguage = CertificateLanguage;

pub struct CertificateLanguage;

impl LanguageConfig for CertificateLanguage {
    fn language_id(&self) -> &'static str {
        "certificate"
    }

    fn extensions(&self) -> &'static [&'static str] {
        // `.csr` is a Certificate Signing Request — same PEM
        // armour, same `-----BEGIN CERTIFICATE REQUEST-----`
        // marker. `.key` is a private key in PEM form. `.cer` is
        // the DER-or-PEM ambiguous extension; we route the PEM
        // case here, the DER case sniffs as binary and lands in
        // the hex view instead.
        &["pem", "crt", "cer", "key", "csr"]
    }

    fn highlight_config(&self) -> Option<HighlightConfiguration> {
        // Highlighting is provided by the regex side-channel in
        // `editor::highlight`; no tree-sitter grammar to wire up.
        None
    }
}
