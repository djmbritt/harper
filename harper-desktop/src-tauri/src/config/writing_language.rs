use std::sync::Arc;

use harper_core::{
    Dialect,
    linting::{FlatConfig, LintGroup},
    spell::{Dictionary, FstDictionary},
};
use serde::{Deserialize, Serialize};

/// The natural language Harper checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WritingLanguage {
    #[default]
    English,
    /// Aruban Papiamento, following "Ortografia di Papiamento" (AB 2018 no. 68).
    Papiamento,
}

impl WritingLanguage {
    /// The curated dictionary for this language.
    pub fn curated_dictionary(self) -> Arc<FstDictionary> {
        match self {
            WritingLanguage::English => FstDictionary::curated(),
            WritingLanguage::Papiamento => harper_papiamento::dictionary::curated(),
        }
    }

    /// A lint group with this language's rules. The dialect only applies to English.
    pub fn create_lint_group(
        self,
        dictionary: Arc<impl Dictionary + 'static>,
        dialect: Dialect,
    ) -> LintGroup {
        match self {
            WritingLanguage::English => LintGroup::new_curated(dictionary, dialect),
            WritingLanguage::Papiamento => harper_papiamento::lint_group(dictionary),
        }
    }

    /// Fill unset rules with their defaults, for every language. The saved rule
    /// configuration holds the rules of all languages, so switching language keeps the
    /// choices made for each.
    pub fn fill_lint_config(config: &mut FlatConfig) {
        config.fill_with_curated();
        harper_papiamento::fill_with_defaults(config);
    }

    /// The default rule configuration for every language.
    pub fn default_lint_config() -> FlatConfig {
        let mut config = FlatConfig::new_curated();
        harper_papiamento::fill_with_defaults(&mut config);
        config
    }
}

#[cfg(test)]
mod tests {
    use super::WritingLanguage;

    #[test]
    fn default_config_enables_both_languages() {
        let config = WritingLanguage::default_lint_config();

        assert!(config.is_rule_enabled("SpellCheck"));
        assert!(config.is_rule_enabled("PapiamentoSpellCheck"));
    }

    #[test]
    fn serializes_like_dialect() {
        assert_eq!(
            serde_json::to_string(&WritingLanguage::Papiamento).unwrap(),
            "\"Papiamento\""
        );
    }

    #[test]
    fn papiamento_dictionary_is_not_english() {
        use harper_core::spell::Dictionary;

        assert!(
            WritingLanguage::Papiamento
                .curated_dictionary()
                .contains_word_str("bonbini")
        );
        assert!(
            !WritingLanguage::English
                .curated_dictionary()
                .contains_word_str("bonbini")
        );
    }
}
