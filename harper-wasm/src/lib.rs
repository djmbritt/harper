#![doc = include_str!("../README.md")]

#[cfg(feature = "bench")]
mod bench;

use std::collections::HashMap;
use std::convert::Into;
use std::io::Cursor;
use std::sync::Arc;

use harper_core::language_detection::is_doc_likely_english;
use harper_core::linting::{FlatConfig, HumanReadableStructuredConfig, StructuredConfig};
use harper_core::linting::{LintGroup, Linter as _};
use harper_core::parsers::{IsolateEnglish, Markdown, Mask, OopsAllHeadings, Parser, PlainEnglish};
use harper_core::remove_overlaps_map;
use harper_core::weirpack::Weirpack;
use harper_core::{
    CharString, DictWordMetadata, Document, IgnoredLints, LintContext, Lrc, remove_overlaps,
    spell::{Dictionary, FstDictionary, MergedDictionary, MutableDictionary},
};
use harper_core::{DialectFlags, RegexMasker};
use harper_stats::{Record, RecordKind, Stats};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

/// Set up the WebAssembly module's logging.
#[wasm_bindgen(start)]
pub fn setup() {
    console_error_panic_hook::set_once();

    // If `setup` gets called more than once, we want to allow this error to fall through.
    let _ = tracing_wasm::try_set_as_global_default();
}

macro_rules! make_serialize_fns_for {
    ($name:ident) => {
        #[wasm_bindgen]
        impl $name {
            pub fn to_json(&self) -> String {
                serde_json::to_string(&self).unwrap()
            }

            pub fn from_json(json: String) -> Result<Self, String> {
                serde_json::from_str(&json).map_err(|err| err.to_string())
            }
        }
    };
}

make_serialize_fns_for!(Suggestion);
make_serialize_fns_for!(Lint);
make_serialize_fns_for!(Span);

#[wasm_bindgen]
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum Language {
    Plain,
    Markdown,
    Typst,
}

impl Language {
    fn create_parser(&self) -> Box<dyn Parser> {
        match self {
            Language::Plain => Box::new(PlainEnglish),
            // TODO: Have a way to configure the Markdown parser
            Language::Markdown => Box::new(Markdown::default()),
            Language::Typst => {
                #[cfg(feature = "typst")]
                {
                    use harper_typst::Typst;
                    Box::new(Typst)
                }
                #[cfg(not(feature = "typst"))]
                {
                    panic!(
                        "Typst is not supported in this version of Harper. Please use the Typst-supported binary."
                    )
                }
            }
        }
    }
}

/// Specifies an English Dialect, often used for linting.
#[wasm_bindgen]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Dialect {
    American,
    British,
    Australian,
    Canadian,
    Indian,
}

/// The natural language to check. Not to be confused with [`Language`], which is the markup
/// format of the text.
#[wasm_bindgen]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WritingLanguage {
    #[default]
    English,
    /// Aruban Papiamento, following "Ortografia di Papiamento" (AB 2018 no. 68).
    Papiamento,
}

impl WritingLanguage {
    /// The curated dictionary for this language.
    fn curated_dictionary(self) -> Arc<FstDictionary> {
        match self {
            WritingLanguage::English => FstDictionary::curated(),
            WritingLanguage::Papiamento => harper_papiamento::dictionary::curated(),
        }
    }

    /// A lint group with this language's rules, and an empty configuration.
    fn create_lint_group(self, dictionary: Arc<MergedDictionary>, dialect: Dialect) -> LintGroup {
        match self {
            WritingLanguage::English => {
                LintGroup::new_curated_empty_config(dictionary, dialect.into())
            }
            WritingLanguage::Papiamento => {
                let mut group = harper_papiamento::lint_group(dictionary);
                group.config.clear();
                group
            }
        }
    }

    /// Fill the rules that `config` leaves unset with this language's defaults.
    fn fill_with_defaults(self, config: &mut FlatConfig) {
        match self {
            WritingLanguage::English => config.fill_with_curated(),
            WritingLanguage::Papiamento => harper_papiamento::fill_with_defaults(config),
        }
    }

    /// The labelled rule settings for this language.
    fn structured_config(self) -> StructuredConfig {
        match self {
            WritingLanguage::English => StructuredConfig::curated(),
            WritingLanguage::Papiamento => harper_papiamento::structured_config(),
        }
    }

    /// The default state of every rule for this language.
    fn default_lint_config(self) -> FlatConfig {
        match self {
            // The same as the global `get_default_lint_config_as_json`.
            WritingLanguage::English => {
                LintGroup::new_curated(MutableDictionary::new().into(), Dialect::American.into())
                    .config
            }
            WritingLanguage::Papiamento => harper_papiamento::default_lint_config(),
        }
    }
}

impl From<Dialect> for harper_core::Dialect {
    fn from(dialect: Dialect) -> Self {
        match dialect {
            Dialect::American => harper_core::Dialect::American,
            Dialect::Canadian => harper_core::Dialect::Canadian,
            Dialect::Australian => harper_core::Dialect::Australian,
            Dialect::British => harper_core::Dialect::British,
            Dialect::Indian => harper_core::Dialect::Indian,
        }
    }
}

#[wasm_bindgen]
pub struct Linter {
    lint_group: LintGroup,
    /// The user-supplied dictionary.
    ///
    /// To make changes affect linting, run [`Self::synchronize_lint_dict`].
    user_dictionary: MutableDictionary,
    dictionary: Arc<MergedDictionary>,
    weirpack_dictionaries: Vec<Arc<MutableDictionary>>,
    ignored_lints: IgnoredLints,
    dialect: Dialect,
    writing_language: WritingLanguage,
    stats: Stats,
}

#[derive(Serialize)]
struct WeirpackTestFailure {
    expected: String,
    got: String,
}

#[wasm_bindgen]
impl Linter {
    /// Construct a new `Linter`.
    /// Note that this can mean constructing the curated dictionary, which is the most expensive operation
    /// in Harper.
    pub fn new(dialect: Dialect) -> Self {
        let writing_language = WritingLanguage::default();
        let dictionary = Self::construct_merged_dict(
            writing_language,
            &[Arc::new(MutableDictionary::default())],
        );
        let lint_group = writing_language.create_lint_group(dictionary.clone(), dialect);

        Self {
            lint_group,
            user_dictionary: MutableDictionary::new(),
            weirpack_dictionaries: Vec::new(),
            dictionary,
            ignored_lints: IgnoredLints::default(),
            dialect,
            writing_language,
            stats: Stats::default(),
        }
    }

    /// Get the natural language this linter checks.
    pub fn get_writing_language(&self) -> WritingLanguage {
        self.writing_language
    }

    /// Switch the natural language this linter checks. This swaps the curated dictionary and
    /// the rules; the user dictionary, ignored lints and the rule configuration are kept.
    /// Like [`Self::import_words`], it rebuilds the rules, so rules from imported Weirpacks
    /// must be imported again.
    pub fn set_writing_language(&mut self, writing_language: WritingLanguage) {
        if self.writing_language != writing_language {
            self.writing_language = writing_language;
            self.synchronize_lint_dict();
        }
    }

    /// Update the dictionary inside [`Self::lint_group`] to include [`Self::user_dictionary`].
    /// This clears any linter caches, so use it sparingly.
    fn synchronize_lint_dict(&mut self) {
        let lint_config = self.lint_group.config.clone();

        let mut constituent_dictionaries = vec![Arc::new(self.user_dictionary.clone())];
        constituent_dictionaries.extend(self.weirpack_dictionaries.iter().cloned());

        self.dictionary =
            Self::construct_merged_dict(self.writing_language, &constituent_dictionaries);

        self.lint_group = self
            .writing_language
            .create_lint_group(self.dictionary.clone(), self.dialect);

        self.lint_group.config.merge_from(lint_config);
    }

    /// Construct the actual dictionary to be used for linting and parsing from the curated dictionary
    /// and any other runtime-provided dictionaries.
    fn construct_merged_dict(
        writing_language: WritingLanguage,
        dicts: &[Arc<impl Dictionary + 'static>],
    ) -> Arc<MergedDictionary> {
        let mut lint_dict = MergedDictionary::new();

        lint_dict.add_dictionary(writing_language.curated_dictionary());

        for dict in dicts {
            lint_dict.add_dictionary(Arc::new(dict.clone()));
        }

        Arc::new(lint_dict)
    }

    /// Helper method to quickly check if a plain string is likely intended to be English
    pub fn is_likely_english(&self, text: String) -> bool {
        let document = Document::new_plain_english(&text, &self.dictionary);
        is_doc_likely_english(&document, &self.dictionary)
    }

    /// Helper method to remove non-English text from a plain English document.
    pub fn isolate_english(&self, text: String) -> String {
        let document = Document::new(
            &text,
            &IsolateEnglish::new(Box::new(PlainEnglish), self.dictionary.clone()),
            &self.dictionary,
        );

        document.to_string()
    }

    /// Get a JSON map containing the descriptions of all the linting rules, formatted as HTML.
    pub fn get_lint_descriptions_html_as_json(&self) -> String {
        serde_json::to_string(&self.lint_group.all_descriptions_html()).unwrap()
    }

    /// Get a Record containing the descriptions of all the linting rules, formatted as HTML.
    pub fn get_lint_descriptions_html_as_object(&self) -> JsValue {
        let serializer = Serializer::json_compatible();
        self.lint_group
            .all_descriptions_html()
            .serialize(&serializer)
            .unwrap()
    }

    /// Get a JSON map containing the descriptions of all the linting rules, formatted as Markdown.
    pub fn get_lint_descriptions_as_json(&self) -> String {
        serde_json::to_string(&self.lint_group.all_descriptions()).unwrap()
    }

    /// Get a Record containing the descriptions of all the linting rules, formatted as Markdown.
    pub fn get_lint_descriptions_as_object(&self) -> JsValue {
        let serializer = Serializer::json_compatible();
        self.lint_group
            .all_descriptions()
            .serialize(&serializer)
            .unwrap()
    }

    pub fn get_lint_config_as_json(&self) -> String {
        serde_json::to_string(&self.lint_group.config).unwrap()
    }

    pub fn get_structured_lint_config_as_json(&self) -> String {
        let mut config = self.writing_language.structured_config();
        config.copy_from_flat_config(&self.lint_group.config);

        let human = HumanReadableStructuredConfig::from_structured_config(&config);
        serde_json::to_string(&human).unwrap()
    }

    pub fn set_lint_config_from_json(&mut self, json: String) -> Result<(), String> {
        self.lint_group.config = serde_json::from_str(&json).map_err(|v| v.to_string())?;
        Ok(())
    }

    pub fn summarize_stats(&self, start_time: Option<i64>, end_time: Option<i64>) -> JsValue {
        let mut operable_copy = self.stats.clone();

        if let Some(start_time) = start_time {
            operable_copy.records.retain(|i| i.when > start_time);
        }

        if let Some(end_time) = end_time {
            operable_copy.records.retain(|i| i.when < end_time);
        }

        operable_copy
            .summarize()
            .serialize(&Serializer::json_compatible())
            .unwrap()
    }

    pub fn get_lint_config_as_object(&self) -> JsValue {
        // Important for downstream JSON serialization
        let serializer = Serializer::json_compatible();

        self.lint_group.config.serialize(&serializer).unwrap()
    }

    pub fn get_structured_lint_config_as_object(&self) -> JsValue {
        let serializer = Serializer::json_compatible();

        let mut config = self.writing_language.structured_config();
        config.copy_from_flat_config(&self.lint_group.config);

        let human = HumanReadableStructuredConfig::from_structured_config(&config);
        human.serialize(&serializer).unwrap()
    }

    pub fn set_lint_config_from_object(&mut self, object: JsValue) -> Result<(), String> {
        self.lint_group.config =
            serde_wasm_bindgen::from_value(object).map_err(|v| v.to_string())?;
        Ok(())
    }

    pub fn ignore_lints(&mut self, source_text: String, lints: Vec<Lint>) {
        let source: Lrc<_> = source_text.chars().collect();

        for lint in lints {
            let document = Document::new_from_chars(
                source.clone(),
                &lint.language.create_parser(),
                &self.dictionary,
            );

            self.ignored_lints.ignore_lint(&lint.inner, &document);
        }
    }

    /// Add a specific context hash to the ignored lints list.
    pub fn ignore_hashes(&mut self, hashes: Vec<u64>) {
        for hash in hashes {
            self.ignored_lints.ignore_hash(hash);
        }
    }

    /// Compute the context hash of a given lint.
    pub fn context_hash(&self, source_text: String, lint: &Lint) -> u64 {
        let source: Vec<_> = source_text.chars().collect();

        let document = Document::new_from_chars(
            source.into(),
            &lint.language.create_parser(),
            &self.dictionary,
        );

        let ctx = LintContext::from_lint(&lint.inner, &document);
        ctx.default_hash()
    }

    fn create_lint_parser(
        &self,
        language: Language,
        all_headings: bool,
        regex_mask: Option<String>,
        isolate_english: bool,
    ) -> Option<Box<dyn Parser>> {
        let mut parser = language.create_parser();

        if let Some(regex) = regex_mask {
            let Some(masker) = RegexMasker::new(regex.as_str(), true) else {
                tracing::warn!("Ignoring lint request because the regex mask is invalid: {regex}");
                return None;
            };

            parser = Box::new(Mask::new(masker, parser));
        }

        if all_headings {
            parser = Box::new(OopsAllHeadings::new(parser));
        }

        if isolate_english {
            parser = Box::new(IsolateEnglish::new(parser, self.dictionary.clone()));
        }

        Some(parser)
    }

    fn with_curated_config<T>(&mut self, lint: impl FnOnce(&mut LintGroup) -> T) -> T {
        let config = self.lint_group.config.clone();
        self.writing_language
            .fill_with_defaults(&mut self.lint_group.config);

        let output = lint(&mut self.lint_group);

        self.lint_group.config = config;
        output
    }

    pub fn organized_lints(
        &mut self,
        text: String,
        language: Language,
        all_headings: bool,
        regex_mask: Option<String>,
        dedup: bool,
        isolate_english: bool,
    ) -> Vec<OrganizedGroup> {
        let source: Lrc<_> = text.chars().collect();
        let Some(parser) =
            self.create_lint_parser(language, all_headings, regex_mask, isolate_english)
        else {
            return vec![];
        };

        let document = Document::new_from_chars(source.clone(), &parser, &self.dictionary);

        let mut lints =
            self.with_curated_config(|lint_group| lint_group.organized_lints(&document));

        for value in lints.values_mut() {
            self.ignored_lints.remove_ignored(value, &document);
        }

        if dedup {
            remove_overlaps_map(&mut lints);
        }

        lints
            .into_iter()
            .map(|(s, ls)| OrganizedGroup {
                group: s,
                lints: ls
                    .into_iter()
                    .map(|l| create_lint(l, &source, language))
                    .collect(),
            })
            .collect()
    }

    /// Perform the configured linting on the provided text.
    ///
    /// If the provided regex mask cannot be parsed, this method will return an empty array.
    pub fn lint(
        &mut self,
        text: String,
        language: Language,
        all_headings: bool,
        regex_mask: Option<String>,
        dedup: bool,
        isolate_english: bool,
    ) -> Vec<Lint> {
        let source: Lrc<_> = text.chars().collect();
        let Some(parser) =
            self.create_lint_parser(language, all_headings, regex_mask, isolate_english)
        else {
            return vec![];
        };

        let document = Document::new_from_chars(source.clone(), &parser, &self.dictionary);

        let mut lints = self.with_curated_config(|lint_group| lint_group.lint(&document));

        self.ignored_lints.remove_ignored(&mut lints, &document);
        if dedup {
            remove_overlaps(&mut lints);
        }

        lints
            .into_iter()
            .map(|l| create_lint(l, &source, language))
            .collect()
    }

    /// Export the linter's ignored lints as a privacy-respecting JSON list of hashes.
    pub fn export_ignored_lints(&self) -> String {
        serde_json::to_string(&self.ignored_lints).unwrap()
    }

    /// Import into the linter's ignored lints from a privacy-respecting JSON list of hashes.
    pub fn import_ignored_lints(&mut self, json: String) -> Result<(), String> {
        let list: IgnoredLints = serde_json::from_str(&json).map_err(|err| err.to_string())?;

        self.ignored_lints.append(list);

        Ok(())
    }

    pub fn clear_ignored_lints(&mut self) {
        self.ignored_lints = IgnoredLints::new();
    }

    /// Clear the user dictionary.
    pub fn clear_words(&mut self) {
        self.user_dictionary = MutableDictionary::new();
        self.synchronize_lint_dict();
    }

    /// Import words into the dictionary.
    pub fn import_words(&mut self, additional_words: Vec<String>) {
        let init_len = self.user_dictionary.word_count();

        self.user_dictionary
            .extend_words(additional_words.iter().map(|word| {
                (
                    word.chars().collect::<CharString>(),
                    DictWordMetadata {
                        dialects: DialectFlags::from_dialect(self.dialect.into()),
                        ..Default::default()
                    },
                )
            }));

        // Only synchronize if we added words that were not there before.
        if self.user_dictionary.word_count() > init_len {
            self.synchronize_lint_dict();
        }
    }

    /// Export words from the dictionary.
    /// Note: this will only return words previously added by [`Self::import_words`].
    pub fn export_words(&mut self) -> Vec<String> {
        self.user_dictionary
            .words_iter()
            .map(|v| v.iter().collect())
            .collect()
    }

    /// Get the dialect this struct was constructed for.
    pub fn get_dialect(&self) -> Dialect {
        self.dialect
    }

    /// Get the default rule configuration for the language this linter checks.
    pub fn get_default_lint_config_as_json(&self) -> String {
        serde_json::to_string(&self.writing_language.default_lint_config()).unwrap()
    }

    /// Apply a suggestion from a given lint.
    /// This action will be logged to the Linter's statistics.
    pub fn apply_suggestion(
        &mut self,
        source_text: String,
        lint: &Lint,
        suggestion: &Suggestion,
    ) -> Result<String, String> {
        let mut source: Vec<_> = source_text.chars().collect();

        let doc = Document::new_from_chars(
            source.clone().into(),
            &lint.language.create_parser(),
            &self.dictionary,
        );

        self.stats
            .records
            .push(Record::now(RecordKind::from_lint(&lint.inner, &doc)));

        suggestion.inner.apply(lint.inner.span, &mut source);

        Ok(source.iter().collect())
    }

    pub fn generate_stats_file(&self) -> String {
        let mut output = Vec::new();
        self.stats.write(&mut output).unwrap();

        String::from_utf8(output).unwrap()
    }

    pub fn import_stats_file(&mut self, file: String) -> Result<(), String> {
        let data = file.as_bytes();
        let mut read = Cursor::new(data);

        let mut new_stats = Stats::read(&mut read).map_err(|err| err.to_string())?;
        self.stats.records.append(&mut new_stats.records);

        Ok(())
    }

    /// Load a Weirpack from raw bytes, merging its rules into the current linter.
    /// Returns test failures if any are found, and does not import in that case.
    pub fn import_weirpack(&mut self, bytes: Vec<u8>) -> Result<JsValue, String> {
        let pack = Weirpack::from_bytes(&bytes).map_err(|err| err.to_string())?;
        let failures = pack.run_tests().map_err(|err| err.to_string())?;

        if !failures.is_empty() {
            let mapped: HashMap<String, Vec<WeirpackTestFailure>> = failures
                .into_iter()
                .map(|(rule, results)| {
                    let failures = results
                        .into_iter()
                        .map(|result| WeirpackTestFailure {
                            expected: result.expected,
                            got: result.got,
                        })
                        .collect();
                    (rule, failures)
                })
                .collect();

            let serializer = Serializer::json_compatible();
            let value = mapped
                .serialize(&serializer)
                .map_err(|err| err.to_string())?;
            return Ok(value);
        }

        if let Some(dict) = pack.load_dictionary().map_err(|err| err.to_string())? {
            self.weirpack_dictionaries.push(Arc::new(dict));
            self.synchronize_lint_dict();
        }

        let group = pack.to_lint_group().map_err(|err| err.to_string())?;
        self.lint_group.merge_from(group);
        Ok(JsValue::UNDEFINED)
    }
}

fn create_lint(lint: harper_core::linting::Lint, source: &[char], language: Language) -> Lint {
    let problem_text = lint.get_str(source);
    let span = Into::<Span>::into(lint.span).to_js_indices(source);

    Lint::new(lint, span, problem_text, language)
}

#[wasm_bindgen]
pub fn to_title_case(text: String) -> String {
    harper_core::make_title_case_str(&text, &PlainEnglish, &FstDictionary::curated())
}

/// A suggestion to fix a Lint.
#[derive(Debug, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct Suggestion {
    inner: harper_core::linting::Suggestion,
}

/// Tags the variant of suggestion.
#[derive(Debug, Serialize, Deserialize)]
#[wasm_bindgen]
pub enum SuggestionKind {
    /// Replace the problematic text.
    Replace = 0,
    /// Remove the problematic text.
    Remove = 1,
    /// Insert additional text after the error.
    InsertAfter = 2,
}

#[wasm_bindgen]
impl Suggestion {
    pub(crate) fn new(inner: harper_core::linting::Suggestion) -> Self {
        Self { inner }
    }

    /// Get the text that is going to replace the problematic section.
    /// If [`Self::kind`] is `SuggestionKind::Remove`, this will return an empty
    /// string.
    pub fn get_replacement_text(&self) -> String {
        match &self.inner {
            harper_core::linting::Suggestion::Remove => "".to_string(),
            harper_core::linting::Suggestion::ReplaceWith(chars) => chars.iter().collect(),
            harper_core::linting::Suggestion::InsertAfter(chars) => chars.iter().collect(),
        }
    }

    pub fn kind(&self) -> SuggestionKind {
        match &self.inner {
            harper_core::linting::Suggestion::Remove => SuggestionKind::Remove,
            harper_core::linting::Suggestion::ReplaceWith(_) => SuggestionKind::Replace,
            harper_core::linting::Suggestion::InsertAfter(_) => SuggestionKind::InsertAfter,
        }
    }
}

/// An error found in provided text.
///
/// May include zero or more suggestions that may fix the problematic text.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[wasm_bindgen]
pub struct Lint {
    inner: harper_core::linting::Lint,
    /// Indexed in a proverbial JS string
    span: Span,
    /// The problematic text that produced this lint.
    problem_text: String,
    language: Language,
}

#[wasm_bindgen]
impl Lint {
    pub(crate) fn new(
        inner: harper_core::linting::Lint,
        span: Span,
        problem_text: String,
        language: Language,
    ) -> Self {
        Self {
            inner,
            span,
            problem_text,
            language,
        }
    }

    /// Get the content of the source material pointed to by [`Self::span`]
    pub fn get_problem_text(&self) -> String {
        self.problem_text.clone()
    }

    /// Get a string representing the general category of the lint.
    pub fn lint_kind(&self) -> String {
        self.inner.lint_kind.to_string_key()
    }

    /// Get a string representing the general category of the lint.
    pub fn lint_kind_pretty(&self) -> String {
        self.inner.lint_kind.to_string()
    }

    /// Equivalent to calling `.length` on the result of `suggestions()`.
    pub fn suggestion_count(&self) -> usize {
        self.inner.suggestions.len()
    }

    /// Get an array of any suggestions that may resolve the issue.
    pub fn suggestions(&self) -> Vec<Suggestion> {
        self.inner
            .suggestions
            .iter()
            .map(|s| Suggestion::new(s.clone()))
            .collect()
    }

    /// Get the location of the problematic text.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Get a description of the error.
    pub fn message(&self) -> String {
        self.inner.message.clone()
    }

    /// Get a description of the error as HTML.
    pub fn message_html(&self) -> String {
        self.inner.message_html()
    }
}

/// Convert Harper's character index into a UTF-16 code unit index understood by JS.
fn char_idx_to_js_str_idx(char_idx: usize, char_str: &[char]) -> usize {
    char_str.iter().take(char_idx).fold(0usize, |acc, ch| {
        acc + if (*ch as u32) <= 0xFFFF { 1 } else { 2 }
    })
}

#[wasm_bindgen]
pub fn get_default_lint_config_as_json() -> String {
    let config =
        LintGroup::new_curated(MutableDictionary::new().into(), Dialect::American.into()).config;

    serde_json::to_string(&config).unwrap()
}

#[wasm_bindgen]
pub fn get_default_lint_config() -> JsValue {
    let config =
        LintGroup::new_curated(MutableDictionary::new().into(), Dialect::American.into()).config;

    // Important for downstream JSON serialization
    let serializer = Serializer::json_compatible();

    config.serialize(&serializer).unwrap()
}

/// A struct that represents two character indices in a string: a start and an end.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[wasm_bindgen]
impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        Into::<harper_core::Span<char>>::into(*self).len()
    }
}

impl Span {
    pub fn to_js_indices(&self, source: &[char]) -> Self {
        Self::new(
            char_idx_to_js_str_idx(self.start, source),
            char_idx_to_js_str_idx(self.end, source),
        )
    }
}

impl From<Span> for harper_core::Span<char> {
    fn from(value: Span) -> Self {
        harper_core::Span::new(value.start, value.end)
    }
}

impl From<harper_core::Span<char>> for Span {
    fn from(value: harper_core::Span<char>) -> Self {
        Span::new(value.start, value.end)
    }
}

/// Used exclusively for [`Linter::organized_lints`]
#[wasm_bindgen]
#[derive(Serialize, Deserialize, Clone)]
pub struct OrganizedGroup {
    #[wasm_bindgen(getter_with_clone)]
    pub group: String,
    #[wasm_bindgen(getter_with_clone)]
    pub lints: Vec<Lint>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

    /// Get memory usage for the process with the given PID, in bytes.
    ///
    /// This will fail if the requested process does not exist.
    #[must_use]
    fn get_mem_usage_of_process(sys: &mut System, pid: Pid) -> Option<u64> {
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            false,
            ProcessRefreshKind::nothing().with_memory(),
        );
        sys.process(pid).map(|process| process.memory())
    }

    /// If a word from another dialect is added to the user dictionary, it should be considered
    /// part of the user's dialect as well.
    #[test]
    fn issue_2216() {
        let text = "Aeon".to_owned();
        let mut linter = Linter::new(Dialect::American);

        linter.import_words(vec![text.clone()]);
        dbg!(linter.dictionary.get_word_metadata_str(&text));

        let lints = linter.lint(text, Language::Plain, false, None, true, false);
        assert!(lints.is_empty());
    }

    fn papiamento_linter() -> Linter {
        let mut linter = Linter::new(Dialect::American);
        linter.set_writing_language(WritingLanguage::Papiamento);
        linter
    }

    fn lint_plain(linter: &mut Linter, text: &str) -> Vec<Lint> {
        linter.lint(text.to_owned(), Language::Plain, false, None, true, false)
    }

    #[test]
    fn defaults_to_english() {
        let linter = Linter::new(Dialect::American);
        assert_eq!(linter.get_writing_language(), WritingLanguage::English);
    }

    #[test]
    fn papiamento_text_is_clean_in_papiamento() {
        let mut linter = papiamento_linter();
        let lints = lint_plain(&mut linter, "Bonbini na Aruba! Mi ta papia Papiamento.");
        assert!(
            lints.is_empty(),
            "{:?}",
            lints.iter().map(|l| l.message()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn papiamento_rules_run_after_switching() {
        let mut linter = papiamento_linter();
        let groups = linter.organized_lints(
            "Sin embargo mi ta bay kas.".to_owned(),
            Language::Plain,
            false,
            None,
            true,
            false,
        );
        let names: Vec<&str> = groups.iter().map(|g| g.group.as_str()).collect();
        assert!(names.contains(&"PapiamentoClosedCompounds"), "{names:?}");
        assert!(names.contains(&"PapiamentoSpellCheck"), "{names:?}");
    }

    #[test]
    fn english_rules_do_not_run_on_papiamento() {
        let mut linter = papiamento_linter();
        let lints = lint_plain(&mut linter, "This is an test.");
        // English words are not in the Papiamento vocabulary, but no English grammar rule
        // (like AnA) should fire.
        assert!(lints.iter().all(|l| l.lint_kind() != "Miscellaneous"));
    }

    #[test]
    fn switching_back_restores_english() {
        let mut linter = papiamento_linter();
        linter.set_writing_language(WritingLanguage::English);
        assert!(lint_plain(&mut linter, "This is a grammatically correct sentence.").is_empty());
    }

    #[test]
    fn keeps_user_words_and_config_when_switching() {
        let mut linter = Linter::new(Dialect::American);
        linter.import_words(vec!["Tinchiwordo".to_owned()]);
        linter
            .set_lint_config_from_json(r#"{"PapiamentoNumerals": false}"#.to_owned())
            .unwrap();
        linter.set_writing_language(WritingLanguage::Papiamento);

        assert!(lint_plain(&mut linter, "Tinchiwordo.").is_empty());
        assert!(lint_plain(&mut linter, "E tin binti y seis aña.").is_empty());
    }

    #[test]
    fn structured_config_follows_language() {
        let english = Linter::new(Dialect::American).get_structured_lint_config_as_json();
        let papiamento = papiamento_linter().get_structured_lint_config_as_json();
        assert!(english.contains("SpellCheck") && !english.contains("PapiamentoSpellCheck"));
        assert!(papiamento.contains("PapiamentoSpellCheck") && !papiamento.contains("\"AnA\""));
    }

    #[test]
    fn english_default_lint_config_is_unchanged() {
        assert_eq!(
            Linter::new(Dialect::American).get_default_lint_config_as_json(),
            get_default_lint_config_as_json()
        );
    }

    #[test]
    fn default_lint_config_follows_language() {
        let config = papiamento_linter().get_default_lint_config_as_json();
        assert!(config.contains("\"PapiamentoNumerals\":true"), "{config}");
    }

    #[test]
    fn no_memory_leak_with_repeated_lints() {
        let pid = Pid::from_u32(std::process::id());
        let mut sys = System::new();

        let mut prev_memory_usage = get_mem_usage_of_process(&mut sys, pid).unwrap();

        if (0..10).all(|_| {
            // Run a few times.
            for _ in 0..10 {
                let mut linter = Linter::new(Dialect::American);

                let results = linter.lint(
                    "This is a grammatically correct sentence.".to_string(),
                    Language::Plain,
                    false,
                    None,
                    true,
                    false,
                );

                assert!(results.is_empty())
            }
            // Check if our process' memory usage increased.
            let curr_memory_usage = get_mem_usage_of_process(&mut sys, pid).unwrap();
            let memory_usage_increased = curr_memory_usage > prev_memory_usage;
            prev_memory_usage = curr_memory_usage;
            memory_usage_increased
        }) {
            panic!("Memory leak!");
        }
    }
}
