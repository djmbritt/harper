use harper_core::linting::{Lint, LintKind, Linter, Suggestion};
use harper_core::{Document, TokenStringExt};

/// §9.6: the names of days and months are written in lowercase.
const DAYS_AND_MONTHS: [&str; 19] = [
    "diadomingo",
    "dialuna",
    "diamars",
    "diaranson",
    "diahuebs",
    "diabierna",
    "diasabra",
    "januari",
    "februari",
    "maart",
    "april",
    "mei",
    "juni",
    "juli",
    "augustus",
    "september",
    "october",
    "november",
    "december",
];

/// Flags capitalized days and months that are not at the start of a sentence (§9.6).
#[derive(Default)]
pub struct LowercaseDaysMonths;

impl Linter for LowercaseDaysMonths {
    fn lint(&mut self, document: &Document) -> Vec<Lint> {
        let source = document.get_source();
        let mut lints = Vec::new();

        for sentence in document.iter_sentences() {
            for word in sentence.iter_words().skip(1) {
                let chars = word.get_ch(source);
                if !chars.first().is_some_and(|c| c.is_uppercase()) {
                    continue;
                }
                let lower: String = chars.iter().flat_map(|c| c.to_lowercase()).collect();
                if !DAYS_AND_MONTHS.contains(&lower.as_str()) {
                    continue;
                }

                lints.push(Lint {
                    span: word.span,
                    lint_kind: LintKind::Capitalization,
                    suggestions: vec![Suggestion::ReplaceWith(lower.chars().collect())],
                    message: format!(
                        "Days and months are written in lowercase: `{lower}` (Ortografia di Papiamento §9.6)."
                    ),
                    priority: 31,
                });
            }
        }

        lints
    }

    fn description(&self) -> &str {
        "Looks for days of the week and months that are capitalized in the middle of a sentence. \"Ortografia di Papiamento\" (AB 2018 no. 68, §9.6) writes them in lowercase."
    }
}
