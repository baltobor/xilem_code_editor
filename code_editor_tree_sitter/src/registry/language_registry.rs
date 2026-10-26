// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::OnceLock;

use super::LanguageConfig;
use crate::languages;

/// Lookup of the compiled-in languages by file extension or language id.
/// Which languages exist depends on the enabled `lang-*` features.
pub struct LanguageRegistry {
    by_extension: HashMap<&'static str, &'static dyn LanguageConfig>,
    by_id: HashMap<&'static str, &'static dyn LanguageConfig>,
    languages: Vec<&'static dyn LanguageConfig>,
}

impl LanguageRegistry {
    fn new() -> Self {
        Self {
            by_extension: HashMap::new(),
            by_id: HashMap::new(),
            languages: Vec::new(),
        }
    }

    fn register(&mut self, lang: &'static dyn LanguageConfig) {
        self.by_id.insert(lang.language_id(), lang);
        for ext in lang.extensions() {
            self.by_extension.insert(*ext, lang);
        }
        self.languages.push(lang);
    }

    /// Language for a file extension (case-insensitive); plain text when
    /// unknown. Files without an extension (`Justfile`) pass their name.
    pub fn from_extension(&self, ext: &str) -> &'static dyn LanguageConfig {
        let lower = ext.to_ascii_lowercase();
        self.by_extension
            .get(lower.as_str())
            .copied()
            .unwrap_or(&languages::plain::PLAIN_TEXT)
    }

    pub fn from_id(&self, id: &str) -> Option<&'static dyn LanguageConfig> {
        self.by_id.get(id).copied()
    }

    /// Language for an editor language string: an id (`"rust"`) or a file
    /// extension (`"rs"`).
    pub fn resolve(&self, language: &str) -> &'static dyn LanguageConfig {
        self.from_id(language)
            .unwrap_or_else(|| self.from_extension(language))
    }

    pub fn all(&self) -> &[&'static dyn LanguageConfig] {
        &self.languages
    }
}

/// The global registry of compiled-in languages.
pub fn registry() -> &'static LanguageRegistry {
    static REGISTRY: OnceLock<LanguageRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut reg = LanguageRegistry::new();
        #[cfg(feature = "lang-rust")]
        reg.register(&languages::rust::RUST);
        #[cfg(feature = "lang-c")]
        reg.register(&languages::c::C);
        #[cfg(feature = "lang-cpp")]
        reg.register(&languages::cpp::CPP);
        #[cfg(feature = "lang-proto")]
        reg.register(&languages::proto::PROTO);
        #[cfg(feature = "lang-toml")]
        reg.register(&languages::toml::TOML);
        #[cfg(feature = "lang-markdown")]
        {
            reg.register(&languages::markdown::MARKDOWN);
            reg.register(&languages::markdown_inline::MARKDOWN_INLINE);
        }
        #[cfg(feature = "lang-json")]
        reg.register(&languages::json::JSON);
        #[cfg(feature = "lang-yaml")]
        reg.register(&languages::yaml::YAML);
        #[cfg(feature = "lang-just")]
        reg.register(&languages::just::JUST);
        #[cfg(feature = "lang-zig")]
        reg.register(&languages::zig::ZIG);
        #[cfg(feature = "lang-pem")]
        reg.register(&languages::certificate::CERTIFICATE);
        reg.register(&languages::plain::PLAIN_TEXT);
        reg
    })
}
