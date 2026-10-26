// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! A reading (prose) font stack for rendered Markdown and other long text.
//!
//! Distinct from `editor::font_family` (which picks the *code*
//! editor's monospace face) — this picks a proportional, screen-
//! optimized reading serif for rendered Markdown body text,
//! following the same cross-platform fallback-stack approach: an
//! ordered candidate list per OS, ending in a generic family so
//! something always renders. parley resolves the first font
//! actually installed.
//!
//! Choices, one per OS, all reading-optimized serifs that ship by
//! default (no extra install required):
//! - macOS: Iowan Old Style (Apple's own book/reading face, bundled
//!   since OS X 10.11) → Georgia (universally bundled, designed for
//!   screen reading) → generic serif.
//! - Windows: Cambria (part of the ClearType Font Collection
//!   bundled since Vista, purpose-built for on-screen reading) →
//!   Georgia → generic serif.
//! - Linux: Noto Serif (very broadly bundled across modern distros,
//!   excellent Unicode coverage) → DejaVu Serif (near-universal
//!   fallback) → Liberation Serif → generic serif.

use std::borrow::Cow;

use xilem::masonry::parley;

/// macOS reading-serif stack.
static MACOS_STACK: &[parley::FontFamilyName<'static>] = &[
    parley::FontFamilyName::Named(Cow::Borrowed("Iowan Old Style")),
    parley::FontFamilyName::Named(Cow::Borrowed("Georgia")),
    parley::FontFamilyName::Generic(parley::GenericFamily::Serif),
];

/// Windows reading-serif stack.
static WINDOWS_STACK: &[parley::FontFamilyName<'static>] = &[
    parley::FontFamilyName::Named(Cow::Borrowed("Cambria")),
    parley::FontFamilyName::Named(Cow::Borrowed("Georgia")),
    parley::FontFamilyName::Generic(parley::GenericFamily::Serif),
];

/// Linux (and other Unix) reading-serif stack.
static LINUX_STACK: &[parley::FontFamilyName<'static>] = &[
    parley::FontFamilyName::Named(Cow::Borrowed("Noto Serif")),
    parley::FontFamilyName::Named(Cow::Borrowed("DejaVu Serif")),
    parley::FontFamilyName::Named(Cow::Borrowed("Liberation Serif")),
    parley::FontFamilyName::Generic(parley::GenericFamily::Serif),
];

/// The reading-font fallback stack for the current OS. Pass to
/// as `RichTextStyle::font_stack`.
pub fn reading_font_stack() -> &'static [parley::FontFamilyName<'static>] {
    if cfg!(target_os = "macos") {
        MACOS_STACK
    } else if cfg!(target_os = "windows") {
        WINDOWS_STACK
    } else {
        LINUX_STACK
    }
}
