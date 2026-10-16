use harper_core::expr::{Expr, FixedPhrase, LongestMatchOf};
use harper_core::linting::{Chunk, ExprLinter, Lint, LintKind, Suggestion};
use harper_core::{Token, TokenStringExt};

/// One wrong form and the form "Ortografia di Papiamento" prescribes for it.
#[derive(Debug, Clone)]
pub struct Replacement {
    /// Lowercase, words separated by single spaces or hyphens.
    pub wrong: String,
    pub correct: String,
    /// The section of AB 2018 no. 68 the rule comes from, e.g. "§11.6".
    pub section: &'static str,
}

impl Replacement {
    pub fn new(
        wrong: impl Into<String>,
        correct: impl Into<String>,
        section: &'static str,
    ) -> Self {
        Self {
            wrong: wrong.into().to_lowercase(),
            correct: correct.into(),
            section,
        }
    }
}

/// A linter that replaces fixed phrases, citing the section of the orthography for each.
pub struct PhraseRule {
    expr: LongestMatchOf,
    replacements: Vec<Replacement>,
    /// Message template; `{correct}` and `{section}` are filled in.
    message: &'static str,
    description: &'static str,
    lint_kind: LintKind,
}

impl PhraseRule {
    pub fn new(
        replacements: Vec<Replacement>,
        message: &'static str,
        description: &'static str,
        lint_kind: LintKind,
    ) -> Self {
        let expr = LongestMatchOf::new(replacements.iter().map(|r| {
            let expr: Box<dyn Expr> = Box::new(FixedPhrase::from_phrase(&r.wrong));
            expr
        }));

        Self {
            expr,
            replacements,
            message,
            description,
            lint_kind,
        }
    }

    pub fn replacements(&self) -> &[Replacement] {
        &self.replacements
    }
}

/// Lowercase, normalize apostrophes and collapse whitespace so matched text can be compared
/// with [`Replacement::wrong`].
fn canonical(text: &[char]) -> String {
    let mut out = String::new();
    let mut last_space = false;
    for c in text {
        if c.is_whitespace() {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
            continue;
        }
        last_space = false;
        match c {
            '\u{2018}' | '\u{2019}' | '\u{02BC}' => out.push('\''),
            '\u{2013}' | '\u{2014}' => out.push('-'),
            c => out.extend(c.to_lowercase()),
        }
    }
    out
}

/// Keep the capitalization of a sentence-initial match ("Sin embargo" → "Sinembargo"),
/// but never lowercase a correction that contains capitals itself ("Sur America").
fn apply_case(correct: &str, matched: &[char]) -> Vec<char> {
    let letters: Vec<&char> = matched.iter().filter(|c| c.is_alphabetic()).collect();
    if letters.len() > 1 && letters.iter().all(|c| c.is_uppercase()) {
        return correct.chars().flat_map(char::to_uppercase).collect();
    }

    let mut out: Vec<char> = correct.chars().collect();
    let starts_upper = matched.first().is_some_and(|c| c.is_uppercase());
    if starts_upper && let Some(first) = out.first_mut() {
        *first = first.to_uppercase().next().unwrap_or(*first);
    }
    out
}

impl ExprLinter for PhraseRule {
    type Unit = Chunk;

    fn expr(&self) -> &dyn Expr {
        &self.expr
    }

    fn match_to_lint(&self, matched_tokens: &[Token], source: &[char]) -> Option<Lint> {
        let span = matched_tokens.span()?;
        let matched = span.get_content(source);
        let key = canonical(matched);

        let hits: Vec<&Replacement> = self
            .replacements
            .iter()
            .filter(|r| r.wrong == key)
            .collect();
        let first = hits.first()?;

        let mut suggestions: Vec<Suggestion> = Vec::new();
        for hit in &hits {
            let suggestion = Suggestion::ReplaceWith(apply_case(&hit.correct, matched));
            if !suggestions.contains(&suggestion) {
                suggestions.push(suggestion);
            }
        }

        Some(Lint {
            span,
            lint_kind: self.lint_kind,
            suggestions,
            message: self
                .message
                .replace("{correct}", &first.correct)
                .replace("{section}", first.section),
            priority: 31,
        })
    }

    fn description(&self) -> &str {
        self.description
    }
}
