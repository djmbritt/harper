# harper-papiamento

Papiamento support for Harper, following the Aruban standard: **"Ortografia di Papiamento"**, the annex to the *Landsverordening schrijfwijze Papiamento* (Afkondigingsblad van Aruba 2018 no. 68, in force since 1 December 2018).

Papiamento is written differently on Aruba (*Papiamento*, etymological spelling) and on Curaçao and Bonaire (*Papiamentu*, phonological spelling). This crate checks Aruban spelling, and recognizes Curaçao/Bonaire spellings so it can point you to the Aruban form.

## Usage

```sh
harper-cli lint --language pap my-text.md
```

From Rust:

```rust
use harper_core::Document;
use harper_core::linting::Linter;

let dictionary = harper_papiamento::dictionary::curated();
let mut linter = harper_papiamento::lint_group(dictionary.clone());
let document = Document::new_plain_english("Sin embargo mi ta bay cas.", &dictionary);
let lints = linter.lint(&document);
```

Parse documents with the Papiamento dictionary (not Harper's English one), so that word tokens get the right metadata.

## Rules

| Rule | What it checks | Section |
| --- | --- | --- |
| `PapiamentoSpellCheck` | Words missing from the official vocabulary. Accepts `-nan` plurals (`palabranan`, `pannan`), the pronoun `e` joined to a verb (`bistie`, `bistiele`) and apostrophe elisions (`cumpr'e`, `wes'i`, `t'asina`). Suggests the Aruban spelling for Curaçao/Bonaire spellings (`kas` → `cas`, `edukashon` → `educacion`, `Papiamentu` → `Papiamento`). | Vocabulario, §1–§5, §8.8b, §10, §11.1–11.2 |
| `PapiamentoClosedCompounds` | Words written as one: `sinembargo`, `talbes`, `porfin`, `akibanda`, `pafo`, `bonbini`, `malgasta`, `pocopoco`, `dialuna`… | §9.6, §11.3c, §11.4b, §11.6, §11.7, §11.9, §11.10 |
| `PapiamentoOpenCompounds` | Words written apart: `a pesar di`, `en bista di`, `bon dia`, `mal fama`, and geographic names such as `Sur America`, `Noord Korea` (no hyphen). | §11.3c, §11.9, §11.11, §12.2b |
| `PapiamentoNumerals` | Numbers 11–99 written as one word: `binti y seis` → `bintiseis`. | §11.8.1a |
| `PapiamentoClockTimes` | `cuatro or` → `cuat'or`, `cinco or` → `cinc'or`… | §10.1.3 |
| `PapiamentoPrefixHyphen` | `vice presidente` → `vice-presidente`, `ex minister` → `ex-minister`. | §8.8d |
| `PapiamentoLowercaseDaysMonths` | Days and months are lowercase mid-sentence: `Dialuna` → `dialuna`, `Mei` → `mei`. | §9.6 |

Every message cites the section of the orthography it comes from.

### Deliberately left out

Rules that would also flag correct sentences are not implemented. For example, `akiden` ("in here") and `aki den Aruba` ("here in Aruba") are both correct, and `pornada` ("in vain") and `mi no por nada` ("I can't do anything") are both correct. The same goes for rules that need grammatical analysis, such as when to write `c` versus `s` in a derivation.

Capitalized words in the middle of a sentence are treated as names. They are only flagged when they are a Curaçao/Bonaire spelling of a known word, because the vocabulary contains very few personal or place names.

## Data

- `data/vocabulario_ab_2018_68.csv`: the 16,199 entries of the "Anexo: Vocabulario di Papiamento", extracted from the published PDF. Spelling variants (`abao/abou`) and cross-references (`aardappel [ardaplo]`) are separate rows, and notes such as `(pisca)` or `(mus.)` are in their own column. The original line is kept in `source_entry`.
- `data/supplement.txt`: words that the rules of the orthography use but the vocabulary is missing (`or`, `a`, `ne`, `signo`, `cuat'or`…), each with the section it comes from.
- `dictionary.dict`: generated from the two files above by `scripts/build_dictionary.py`. Multi-word, hyphenated and abbreviated entries are split into the tokens Harper's lexer produces. Do not edit it by hand; run:

  ```sh
  python3 harper-papiamento/scripts/build_dictionary.py
  ```

The vocabulary is part of a published law. Dutch copyright law excludes legislation from protection (Auteurswet, art. 11). Check that Aruba's Auteursverordening does the same before you redistribute the data elsewhere.

Linting the rule text of AB 2018 no. 68 itself turns up three typos in the official document: `skibi` (§5 ch), `pronuniciacion` (§11.1c) and `derivacionnnan` (§5 ñ). It also turns up one inconsistency: §2 writes `apesar cu`, which §11.3c says should be `a pesar cu`.

`examples/check.rs` lints a text file and prints a summary of what it found:

```sh
cargo run -p harper-papiamento --example check -- some-text.txt
```
