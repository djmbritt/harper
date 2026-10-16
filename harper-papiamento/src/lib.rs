//! Papiamento support for Harper.
//!
//! Papiamento is written differently on Aruba (Papiamento, etymological spelling) and on
//! Curaçao and Bonaire (Papiamentu, phonological spelling). This crate follows the Aruban
//! standard, "Ortografia di Papiamento", the annex to the Landsverordening schrijfwijze
//! Papiamento (Afkondigingsblad van Aruba 2018 no. 68):
//!
//! - [`dictionary::curated`] is built from the "Anexo: Vocabulario di Papiamento".
//! - [`PapiamentoSpellCheck`] checks words against it and points out Curaçao/Bonaire
//!   spellings.
//! - [`rules`] implements the orthography rules that can be checked mechanically.
//!
//! Use [`lint_group`] to get a [`LintGroup`] with all of them enabled, and parse documents
//! with the Papiamento dictionary so that words get the right metadata:
//!
//! ```
//! use harper_core::Document;
//! use harper_core::linting::Linter;
//!
//! let dictionary = harper_papiamento::dictionary::curated();
//! let mut linter = harper_papiamento::lint_group(dictionary.clone());
//! let document = Document::new_plain_english("Sin embargo mi ta bay cas.", &dictionary);
//!
//! assert_eq!(linter.lint(&document).len(), 1);
//! ```

pub mod dictionary;
mod papiamentu;
pub mod rules;
mod spell;

use std::sync::Arc;

use harper_core::linting::{
    FlatConfig, HumanReadableStructuredConfig, LintGroup, StructuredConfig,
};
use harper_core::spell::Dictionary;

pub use papiamentu::aruban_candidates;
pub use spell::PapiamentoSpellCheck;

/// The Papiamento rules grouped and labelled for settings screens, with their default
/// states (all enabled). Mirrors `StructuredConfig::curated()` for English.
pub fn structured_config() -> StructuredConfig {
    let human: HumanReadableStructuredConfig =
        serde_json::from_str(include_str!("../default_config.json"))
            .expect("default_config.json should be valid");

    human
        .to_structured_config()
        .expect("default_config.json should be a valid structured config")
}

/// The default state of every Papiamento rule. Mirrors `FlatConfig::new_curated()` for
/// English.
pub fn default_lint_config() -> FlatConfig {
    structured_config()
        .to_flat_config()
        .expect("default_config.json should be a valid structured config")
}

/// Fill the Papiamento rules that `config` does not mention with their defaults, keeping
/// everything that is set. Mirrors `FlatConfig::fill_with_curated()` for English.
pub fn fill_with_defaults(config: &mut FlatConfig) {
    let mut filled = default_lint_config();
    filled.merge_from(std::mem::take(config));
    *config = filled;
}

/// Create a [`LintGroup`] with every Papiamento rule enabled.
pub fn lint_group(dictionary: Arc<impl Dictionary + 'static>) -> LintGroup {
    let mut group = LintGroup::empty();

    group.add(
        "PapiamentoSpellCheck",
        PapiamentoSpellCheck::new(dictionary),
    );
    group.add("PapiamentoLowercaseDaysMonths", rules::LowercaseDaysMonths);
    group.add_chunk_expr_linter("PapiamentoClosedCompounds", rules::closed_compounds());
    group.add_chunk_expr_linter("PapiamentoOpenCompounds", rules::open_compounds());
    group.add_chunk_expr_linter("PapiamentoNumerals", rules::numerals());
    group.add_chunk_expr_linter("PapiamentoClockTimes", rules::clock_times());
    group.add_chunk_expr_linter("PapiamentoPrefixHyphen", rules::prefix_hyphen());

    group.config = default_lint_config();
    group
}

#[cfg(test)]
mod tests {
    use harper_core::linting::{Lint, Linter, Suggestion};
    use harper_core::{Document, Span};

    use super::{dictionary, lint_group};

    fn lint(text: &str) -> Vec<Lint> {
        let dict = dictionary::curated();
        let document = Document::new_plain_english(text, &dict);
        let mut lints = lint_group(dict).lint(&document);
        lints.sort_by_key(|l| l.span.start);
        lints
    }

    fn first_fix(text: &str) -> String {
        let lints = lint(text);
        assert!(!lints.is_empty(), "no lint for {text:?}");
        let lint = &lints[0];
        let mut chars: Vec<char> = text.chars().collect();
        match &lint.suggestions[0] {
            Suggestion::ReplaceWith(with) => {
                chars.splice(lint.span.start..lint.span.end, with.iter().copied());
            }
            other => panic!("unexpected suggestion {other:?}"),
        }
        chars.into_iter().collect()
    }

    #[test]
    fn clean_text_has_no_lints() {
        // Sentences taken from the examples in "Ortografia di Papiamento".
        for text in [
            "Mañan mi no ta bay hunga bala.",
            "E ta biba na Rincon (Boneiro).",
            "Lo kita coriente den e siguiente barionan: Santa Helena, un parti; Pos Abou, un parti; Tanki Flip.",
            "Su dianan di consulta ta dialuna, diaranson, diabierna y diasabra.",
            "Si, pero ... no t'asina facil.",
            "Tur mainta - ta Pedro a bisa mi - e ta bebe awa di coco cu ijs.",
            "El a sirbi pisca hasa, banana hasa y funchi: un plato crioyo di berdad.",
            "Bin aki mesora!",
            "Nos ta invita Bo pa un reunion.",
            "E ta lanta seis or.",
            "E ta cumpr'e auto.",
            "Sinembargo, e no a yega na tempo.",
        ] {
            let messages: Vec<String> = lint(text).iter().map(|l| l.message.clone()).collect();
            assert!(messages.is_empty(), "{text}: {messages:?}");
        }
    }

    #[test]
    fn structured_config_covers_every_rule() {
        let group = lint_group(dictionary::curated());
        let config = super::default_lint_config();

        let mut rules: Vec<&str> = group.iter_keys().collect();
        rules.sort_unstable();
        for rule in &rules {
            assert!(
                config.is_rule_enabled(rule),
                "{rule} is not enabled by default"
            );
        }
        let serialized = serde_json::to_value(&config).unwrap();
        assert_eq!(serialized.as_object().unwrap().len(), rules.len());
    }

    #[test]
    fn fill_with_defaults_keeps_user_choices() {
        let mut config = harper_core::linting::FlatConfig::default();
        config.set_rule_enabled("PapiamentoNumerals", false);
        config.set_rule_enabled("SpellCheck", true);
        super::fill_with_defaults(&mut config);

        assert!(!config.is_rule_enabled("PapiamentoNumerals"));
        assert!(config.is_rule_enabled("PapiamentoSpellCheck"));
        assert!(config.is_rule_enabled("SpellCheck"));
    }

    #[test]
    fn closed_compounds() {
        assert_eq!(
            first_fix("Sin embargo, e no a bin."),
            "Sinembargo, e no a bin."
        );
        assert_eq!(first_fix("Tal bes e ta bin."), "Talbes e ta bin.");
        assert_eq!(first_fix("Bon bini na Aruba!"), "Bonbini na Aruba!");
        assert_eq!(
            first_fix("Nan ta mal gasta placa."),
            "Nan ta malgasta placa."
        );
        assert_eq!(first_fix("Bay poco poco."), "Bay pocopoco.");
        assert_eq!(first_fix("Mi ta bin dia luna."), "Mi ta bin dialuna.");
    }

    #[test]
    fn open_compounds() {
        assert_eq!(
            first_fix("Apesar di tur cos, e a bin."),
            "A pesar di tur cos, e a bin."
        );
        assert_eq!(first_fix("Bondia, con ta bay?"), "Bon dia, con ta bay?");
        assert_eq!(
            first_fix("Nan ta biba na Suramerica."),
            "Nan ta biba na Sur America."
        );
        assert_eq!(
            first_fix("Nan ta biba na Noord-Korea."),
            "Nan ta biba na Noord Korea."
        );
    }

    #[test]
    fn numerals() {
        assert_eq!(first_fix("E tin binti y seis aña."), "E tin bintiseis aña.");
        assert_eq!(first_fix("E tin trinta dos aña."), "E tin trintidos aña.");
        assert_eq!(first_fix("E tin dies y shete aña."), "E tin diesshete aña.");
    }

    #[test]
    fn clock_times() {
        assert_eq!(first_fix("Mi ta bin cuatro or."), "Mi ta bin cuat'or.");
    }

    #[test]
    fn prefix_hyphen() {
        assert_eq!(
            first_fix("E vice presidente a papia."),
            "E vice-presidente a papia."
        );
    }

    #[test]
    fn lowercase_days_and_months() {
        assert_eq!(
            first_fix("E ta bin Dialuna cu su mama."),
            "E ta bin dialuna cu su mama."
        );
        assert_eq!(first_fix("E a nace na Mei."), "E a nace na mei.");
    }

    #[test]
    fn sentence_initial_day_is_fine() {
        assert!(lint("Dialuna e ta bin.").is_empty());
    }

    #[test]
    fn papiamentu_spelling() {
        assert_eq!(first_fix("Mi ta bay kas."), "Mi ta bay cas.");
        let lints = lint("Mi ta papia Papiamentu.");
        assert_eq!(lints.len(), 1);
        assert!(lints[0].message.contains("Curaçao"));
        assert_eq!(lints[0].span, Span::new(12, 22));
    }
}
