//! Spell checking against the official Papiamento vocabulary.

use harper_core::linting::{Lint, LintKind, Linter, Suggestion};
use harper_core::spell::{Dictionary, suggest_correct_spelling};
use harper_core::{CharString, Document, TokenStringExt};
use hashbrown::HashMap;

use crate::papiamentu::{aruban_candidates, looks_like_papiamentu};

const MAX_SUGGESTIONS: usize = 3;

/// Abbreviated titles from the vocabulary and §9.4 that are followed by a name.
const TITLE_ABBREVIATIONS: [&str; 14] = [
    "Sr", "Sra", "Srta", "Dr", "dr", "Drs", "drs", "Mr", "mr", "Mgr", "Lic", "Ing", "ing", "ir",
];
const VOWELS: [char; 5] = ['a', 'e', 'i', 'o', 'u'];

/// Flags words that are not in the Papiamento vocabulary.
///
/// Besides plain dictionary lookups, it accepts the regular forms that the vocabulary
/// does not list because "Ortografia di Papiamento" derives them by rule:
///
/// - plurals in `-nan` (§8.8b, §5.1: `palabranan`, `pannan`),
/// - the personal pronoun `e` joined to a verb ending in `i` (§11.1a, §11.2a: `bistie`,
///   `bistiele`, `construye`),
/// - words shortened with an apostrophe (§10: `cumpr'e`, `wes'i`, `t'asina`, `dun'ele`).
///
/// When a word is written in Curaçao/Bonaire spelling and its Aruban spelling is in the
/// vocabulary, the lint says so and suggests the Aruban form.
pub struct PapiamentoSpellCheck<D: Dictionary> {
    dictionary: D,
    cache: HashMap<String, (LintKind, Vec<String>)>,
}

impl<D: Dictionary> PapiamentoSpellCheck<D> {
    pub fn new(dictionary: D) -> Self {
        Self {
            dictionary,
            cache: HashMap::new(),
        }
    }

    /// Whether `word` is correctly spelled Papiamento.
    pub fn is_correct(&self, word: &str) -> bool {
        let word = normalize(word);

        if word.chars().count() <= 1
            || word.chars().any(|c| c.is_ascii_digit())
            || is_acronym(&word)
        {
            return true;
        }

        // Abbreviations keep their periods in the token (§8.1b: `p.e.`, `w.c.`, `etc.`).
        if word.contains('.') {
            let letters: String = word.chars().filter(|c| *c != '.').collect();
            return word.split('.').all(|part| part.chars().count() <= 1)
                || self.contains(&letters)
                || self.is_valid_form(&letters);
        }

        if word.contains('\'') {
            return self.is_valid_elision(&word);
        }

        self.is_valid_form(&word)
    }

    /// A dictionary word, or a regular derivation of one.
    fn is_valid_form(&self, word: &str) -> bool {
        if self.contains(word) {
            return true;
        }

        let lower = word.to_lowercase();

        // Plural -nan (§8.8b). A stem ending in n keeps it: pan + nan = pannan (§5.1).
        if let Some(stem) = lower.strip_suffix("nan")
            && stem.chars().count() >= 2
            && (self.contains(stem) || self.is_clitic_form(stem))
        {
            return true;
        }

        self.is_clitic_form(&lower)
    }

    /// The pronoun `e` (and `ele`, §11.2a) written together with a verb ending in `i`
    /// (§11.1a: `bistie`); an `i` that becomes [j] is written `y` (§3.1: `construye`).
    fn is_clitic_form(&self, lower: &str) -> bool {
        for suffix in ["ele", "e"] {
            let Some(stem) = lower.strip_suffix(suffix) else {
                continue;
            };
            if stem.ends_with('i') && self.contains(stem) {
                return true;
            }
            if let Some(before_y) = stem.strip_suffix('y')
                && self.contains(&format!("{before_y}i"))
            {
                return true;
            }
        }
        false
    }

    /// Words written with an apostrophe because sounds were dropped (§10).
    fn is_valid_elision(&self, word: &str) -> bool {
        if self.contains(word) {
            return true;
        }

        word.split('\'').all(|part| self.is_valid_fragment(part))
    }

    /// One side of an apostrophe: a word with a vowel or a sound dropped at its start, end
    /// or inside (`cumpr` < cumpra, `i` < di, `botr` < boter), or a regular form.
    fn is_valid_fragment(&self, part: &str) -> bool {
        let lower = part.to_lowercase();

        if lower.chars().count() <= 1 || lower == "ele" || self.is_valid_form(&lower) {
            return true;
        }

        let chars: Vec<char> = lower.chars().collect();
        (0..=chars.len()).any(|i| {
            VOWELS.iter().any(|v| {
                let mut restored = chars.clone();
                restored.insert(i, *v);
                self.contains(&restored.into_iter().collect::<String>())
            })
        })
    }

    fn contains(&self, word: &str) -> bool {
        self.dictionary.contains_word_str(word)
    }

    fn papiamentu_suggestions(&self, word: &str) -> Vec<String> {
        if !looks_like_papiamentu(word) {
            return Vec::new();
        }

        aruban_candidates(word)
            .into_iter()
            .filter_map(|c| {
                // Use the capitalization the vocabulary gives the word (`Papiamento`).
                let chars: CharString = c.chars().collect();
                self.dictionary
                    .get_correct_capitalization_of(&chars)
                    .map(|correct| correct.iter().collect())
            })
            .take(MAX_SUGGESTIONS)
            .collect()
    }

    fn spelling_suggestions(&self, word: &str) -> Vec<String> {
        let chars: CharString = word.chars().collect();
        for distance in 2..5 {
            let suggestions: Vec<String> =
                suggest_correct_spelling(&chars, 200, distance, &self.dictionary)
                    .into_iter()
                    .take(MAX_SUGGESTIONS)
                    .map(|s| s.iter().collect())
                    .collect();

            if !suggestions.is_empty() {
                return suggestions;
            }
        }
        Vec::new()
    }

    fn analyze(&mut self, word: &str) -> (LintKind, Vec<String>) {
        if let Some(hit) = self.cache.get(word) {
            return hit.clone();
        }

        let papiamentu = self.papiamentu_suggestions(word);
        let result = if papiamentu.is_empty() {
            (LintKind::Spelling, self.spelling_suggestions(word))
        } else {
            (LintKind::Regionalism, papiamentu)
        };

        self.cache.insert(word.to_owned(), result.clone());
        result
    }
}

impl<D: Dictionary> Linter for PapiamentoSpellCheck<D> {
    fn lint(&mut self, document: &Document) -> Vec<Lint> {
        let source = document.get_source();
        let mut lints = Vec::new();

        let words = document.iter_sentences().flat_map(|sentence| {
            sentence
                .iter_words()
                .enumerate()
                .map(|(i, token)| (i == 0, token))
        });

        let mut previous_word = String::new();
        for (first_in_sentence, token) in words {
            let text = token.get_str(source);
            // "Sr. Figaroa": the period of a title does not start a new sentence.
            let sentence_start =
                first_in_sentence && !TITLE_ABBREVIATIONS.contains(&previous_word.as_str());
            previous_word = text.clone();

            if self.is_correct(&text) {
                continue;
            }

            let (lint_kind, suggestions) = self.analyze(&normalize(&text));

            // A capitalized word inside a sentence is most likely a name, and the vocabulary
            // lists very few names. Only flag it when it is a Curaçao/Bonaire spelling of a
            // word we know (`Papiamentu`).
            let capitalized = text.chars().next().is_some_and(char::is_uppercase);
            if capitalized && !sentence_start && lint_kind != LintKind::Regionalism {
                continue;
            }
            let template = token.get_ch(source);
            let suggestions: Vec<Vec<char>> = suggestions
                .iter()
                .map(|s| match_case(s, template))
                .collect();

            let message = match (lint_kind, suggestions.first()) {
                (LintKind::Regionalism, Some(first)) => format!(
                    "`{text}` is Curaçao/Bonaire (Papiamentu) spelling. In Aruban Papiamento it is written `{}`.",
                    first.iter().collect::<String>()
                ),
                _ => format!("`{text}` is not in the Papiamento vocabulary."),
            };

            lints.push(Lint {
                span: token.span,
                lint_kind,
                suggestions: suggestions
                    .into_iter()
                    .map(Suggestion::ReplaceWith)
                    .collect(),
                message,
                priority: 63,
            });
        }

        lints
    }

    fn description(&self) -> &str {
        "Looks for words that are not in the official Papiamento vocabulary (Ortografia di Papiamento, AB 2018 no. 68) and suggests Aruban spellings for Curaçao/Bonaire (Papiamentu) spellings."
    }
}

/// Capitalize the suggestion like the misspelled word, keeping capitals the suggestion has.
fn match_case(suggestion: &str, template: &[char]) -> Vec<char> {
    let mut out: Vec<char> = suggestion.chars().collect();
    let all_caps = template.len() > 1 && template.iter().all(|c| !c.is_lowercase());
    if all_caps {
        return out.iter().flat_map(|c| c.to_uppercase()).collect();
    }
    if template.first().is_some_and(|c| c.is_uppercase())
        && let Some(first) = out.first_mut()
    {
        *first = first.to_uppercase().next().unwrap_or(*first);
    }
    out
}

/// Use a plain apostrophe for every apostrophe-like character.
fn normalize(word: &str) -> String {
    word.chars()
        .map(|c| match c {
            '\u{2018}' | '\u{2019}' | '\u{02BC}' | '\u{FF07}' => '\'',
            c => c,
        })
        .collect()
}

/// Acronyms are written as the organization writes them (§8.1b: UNESCO, C.I.A.).
fn is_acronym(word: &str) -> bool {
    word.chars().all(|c| c.is_uppercase()) && word.chars().count() <= 8
}

#[cfg(test)]
mod tests {
    use harper_core::Document;
    use harper_core::linting::{LintKind, Linter};

    use super::PapiamentoSpellCheck;
    use crate::dictionary;

    fn checker() -> PapiamentoSpellCheck<std::sync::Arc<harper_core::spell::FstDictionary>> {
        PapiamentoSpellCheck::new(dictionary::curated())
    }

    fn lint(text: &str) -> Vec<harper_core::linting::Lint> {
        let dict = dictionary::curated();
        let document = Document::new_plain_english(text, &dict);
        checker().lint(&document)
    }

    #[test]
    fn accepts_dictionary_words() {
        assert!(lint("Mi ta papia Papiamento cu mi mama y tata.").is_empty());
    }

    #[test]
    fn accepts_plurals() {
        let c = checker();
        assert!(c.is_correct("palabranan"));
        assert!(c.is_correct("derivacionnan"));
        assert!(c.is_correct("pannan"));
        assert!(c.is_correct("bukinan"));
    }

    #[test]
    fn accepts_clitic_pronoun() {
        let c = checker();
        assert!(c.is_correct("bistie"));
        assert!(c.is_correct("bistiele"));
        assert!(c.is_correct("construye"));
    }

    #[test]
    fn accepts_document_examples_of_apostrophes() {
        let c = checker();
        for word in [
            "cumpr'e",
            "fang'e",
            "sunch'e",
            "bend'e",
            "p'e",
            "cuat'or",
            "mashin'i",
            "t'asina",
            "m'a",
            "wes'i",
            "sop'i",
            "botr'i",
            "dun'ele",
            "bos'e",
            "ta'tin",
            "ta'ta",
            "machic'e",
            "cantic'i",
            "lag'e",
            "carg'i",
            "p'esey",
            "puntr'e",
            "cumpr’e",
        ] {
            assert!(c.is_correct(word), "{word} should be accepted");
        }
    }

    #[test]
    fn accepts_document_sentences() {
        for sentence in [
            "Tecla a cumpr'e.",
            "Su mama a bistiele.",
            "Mi n' sa.",
            "Den hofi nos tin promenton, tomati, bonchi y lechuga.",
            "Su sapato a kibra; p’esey e no por a bay fiesta.",
            "Ayera mi a mira Boy Ecury, nos heroe nacional.",
            "Mi ta gradici Sr. Figaroa y Sra. Marcia.",
        ] {
            let lints = lint(sentence);
            let flagged: Vec<_> = lints.iter().map(|l| l.message.clone()).collect();
            assert!(flagged.is_empty(), "{sentence}: {flagged:?}");
        }
    }

    #[test]
    fn flags_unknown_word_at_sentence_start() {
        assert_eq!(lint("Papiaa cu mi.").len(), 1);
    }

    #[test]
    fn flags_misspelling_with_suggestion() {
        let lints = lint("Mi ta papiaa.");
        assert_eq!(lints.len(), 1);
        assert_eq!(lints[0].lint_kind, LintKind::Spelling);
        assert!(!lints[0].suggestions.is_empty());
    }

    #[test]
    fn suggests_aruban_spelling_for_papiamentu() {
        for (wrong, right) in [
            ("kas", "cas"),
            ("Papiamentu", "Papiamento"),
            ("edukashon", "educacion"),
            ("kuminda", "cuminda"),
        ] {
            let lints = lint(wrong);
            assert_eq!(lints.len(), 1, "{wrong}");
            assert_eq!(lints[0].lint_kind, LintKind::Regionalism, "{wrong}");
            let first = match &lints[0].suggestions[0] {
                harper_core::linting::Suggestion::ReplaceWith(c) => c.iter().collect::<String>(),
                other => panic!("{other:?}"),
            };
            assert_eq!(first, right, "{wrong}");
        }
    }

    #[test]
    fn allows_abbreviations() {
        assert!(lint("Bo por trece fruta, p.e. manga y papaya, etc. pa e w.c. nobo.").is_empty());
    }

    #[test]
    fn catches_typos_in_the_official_document() {
        // These three typos occur in the rule text of AB 2018 no. 68 itself.
        let c = checker();
        assert!(!c.is_correct("skibi"));
        assert!(!c.is_correct("pronuniciacion"));
        assert!(!c.is_correct("derivacionnnan"));
    }

    #[test]
    fn allows_acronyms_and_numbers() {
        assert!(lint("UNESCO a bin na 1945.").is_empty());
    }
}
