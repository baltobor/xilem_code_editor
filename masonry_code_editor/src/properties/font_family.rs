//! Editor font-family choice.
//!
//! The code editor renders with a generic monospace by default, which the
//! OS resolves to a serif-ish fixed-width face. The user can instead pick a
//! sans-serif coding face. To stay cross-platform we never hardcode a single
//! OS-specific font: each choice maps to an ordered candidate list ending in
//! a generic family. parley resolves the **first font actually installed**
//! (CSS-style fallback), so the same choice lands on SF Mono on macOS
//! (Xcode's editor font), on Consolas / Cascadia on Windows, on DejaVu /
//! Liberation Mono on Linux, and on the guaranteed generic `monospace`
//! anywhere none of the named faces exist. The font that actually renders is
//! whatever the fallback chain first resolves to.

use std::borrow::Cow;

use masonry::parley;
use serde::{Deserialize, Serialize};

/// Which typeface family the editor renders code in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EditorFontFamily {
    /// Generic monospace — the historical default (serif-ish fixed width).
    #[default]
    Serif,
    /// A readable sans-serif coding face, resolved per-OS via the fallback
    /// stack in [`EditorFontFamily::families`].
    SansSerif,
}

/// Generic monospace only — whatever the OS maps `monospace` to.
static SERIF_STACK: &[parley::FontFamilyName<'static>] = &[parley::FontFamilyName::Generic(
    parley::GenericFamily::Monospace,
)];

/// Sans-serif coding candidates, most-preferred first, ending in the generic
/// monospace floor so something always renders. macOS resolves the head
/// (SF Mono → Menlo, i.e. Xcode's editor font); Windows lands on Cascadia /
/// Consolas; Linux on DejaVu / Liberation Mono.
static SANS_STACK: &[parley::FontFamilyName<'static>] = &[
    parley::FontFamilyName::Named(Cow::Borrowed("SF Mono")),
    parley::FontFamilyName::Named(Cow::Borrowed("Menlo")),
    parley::FontFamilyName::Named(Cow::Borrowed("Cascadia Code")),
    parley::FontFamilyName::Named(Cow::Borrowed("Consolas")),
    parley::FontFamilyName::Named(Cow::Borrowed("DejaVu Sans Mono")),
    parley::FontFamilyName::Named(Cow::Borrowed("Liberation Mono")),
    parley::FontFamilyName::Generic(parley::GenericFamily::Monospace),
];

impl EditorFontFamily {
    /// The ordered candidate stack for this choice. Push it as the default
    /// font on a parley builder via `StyleProperty::from(family.families())`.
    pub fn families(self) -> &'static [parley::FontFamilyName<'static>] {
        match self {
            EditorFontFamily::Serif => SERIF_STACK,
            EditorFontFamily::SansSerif => SANS_STACK,
        }
    }
}
