//! Lint a text file with the Papiamento rules and print a summary.
//!
//! cargo run -p harper-papiamento --example check -- path/to/file.txt

use std::collections::BTreeMap;

use harper_core::linting::{Linter, Suggestion};
use harper_core::{Document, TokenStringExt};

fn main() {
    let path = std::env::args().nth(1).expect("usage: check <file>");
    let text = std::fs::read_to_string(path).expect("could not read file");

    let dictionary = harper_papiamento::dictionary::curated();
    let mut linter = harper_papiamento::lint_group(dictionary.clone());
    let document = Document::new_plain_english(&text, &dictionary);

    let source = document.get_source();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for lint in linter.lint(&document) {
        let found = lint.get_str(source);
        let fix = lint
            .suggestions
            .first()
            .map(|s| match s {
                Suggestion::ReplaceWith(c) => c.iter().collect(),
                _ => String::new(),
            })
            .unwrap_or_default();
        *counts
            .entry(format!("{:?}\t{found}\t-> {fix}", lint.lint_kind))
            .or_default() += 1;
    }

    let words = document.iter_words().count();
    let lints: usize = counts.values().sum();
    for (line, n) in &counts {
        println!("{n:>4}  {line}");
    }
    println!("\n{lints} lints in {words} words");
}
