//! Orthography rules from "Ortografia di Papiamento", the annex to the Landsverordening
//! schrijfwijze Papiamento (Afkondigingsblad van Aruba 2018 no. 68).
//!
//! Only rules that can be checked without understanding the sentence are implemented, and
//! phrases that are also correct when written the other way in some contexts are left out
//! (for example `aki den Aruba`, "here in Aruba", next to `akiden`, "in here").

mod lowercase_days_months;
mod phrase_rule;

pub use lowercase_days_months::LowercaseDaysMonths;
pub use phrase_rule::{PhraseRule, Replacement};

use harper_core::linting::LintKind;

/// Words that are written as one (§9.6, §11.3c, §11.4b, §11.6, §11.7, §11.9, §11.10).
pub fn closed_compounds() -> PhraseRule {
    let mut r = Vec::new();

    // §11.6a: adverbs of place are written as one word. Only the ones whose second part
    // cannot stand alone as a preposition are included.
    for first in ["aki", "ey", "aya"] {
        for second in ["banda", "fo"] {
            r.push(Replacement::new(
                format!("{first} {second}"),
                format!("{first}{second}"),
                "§11.6a",
            ));
        }
    }
    r.push(Replacement::new("pa fo", "pafo", "§11.6a"));

    // §11.6b: adverbs written as one word.
    for (wrong, correct) in [
        ("ala bes", "alabes"),
        ("al a bes", "alabes"),
        ("ala fin", "alafin"),
        ("al a fin", "alafin"),
        ("ala fan", "alafan"),
        ("anta yera", "antayera"),
        ("awor aki", "aworaki"),
        ("awor ey", "aworey"),
        ("di ripiente", "diripiente"),
        ("en fin", "enfin"),
        ("en vano", "envano"),
        ("en bano", "envano"),
        ("meya nochi", "meyanochi"),
        ("por fin", "porfin"),
        ("sin embargo", "sinembargo"),
        ("tal bes", "talbes"),
        ("trei mer dia", "treimerdia"),
        ("treimer dia", "treimerdia"),
        ("una bes", "unabes"),
    ] {
        r.push(Replacement::new(wrong, correct, "§11.6b"));
    }

    // §11.3c, exception: encuanto.
    r.push(Replacement::new("en cuanto", "encuanto", "§11.3c"));

    // §11.4b, exceptions to noun + adjective being written apart.
    r.push(Replacement::new("pan bati", "panbati", "§11.4b"));
    r.push(Replacement::new("pan dushi", "pandushi", "§11.4b"));

    // §11.7: a basic word plus a part that cannot stand alone.
    r.push(Replacement::new("tatara nieto", "tataranieto", "§11.7"));

    // §11.9.1a: mal + verb is written as one word, and so are its derivations.
    for verb in [
        "gasta",
        "trata",
        "uza",
        "cria",
        "interpreta",
        "versa",
        "diciona",
        "logra",
        "criansa",
        "gastamento",
    ] {
        let correct = if verb == "logra" {
            "malogra".to_owned()
        } else {
            format!("mal{verb}")
        };
        r.push(Replacement::new(format!("mal {verb}"), correct, "§11.9.1a"));
    }

    // §11.9.1b, exceptions.
    r.push(Replacement::new(
        "mal agradecido",
        "malagradecido",
        "§11.9.1b",
    ));
    r.push(Replacement::new("mal contento", "malcontento", "§11.9.1b"));

    // §11.9.2, exceptions to bon being written apart.
    r.push(Replacement::new("bon bini", "bonbini", "§11.9.2"));
    r.push(Replacement::new("bon bida", "bonbida", "§11.9.2"));

    // §11.10b, exception: pocopoco is always written as one word.
    r.push(Replacement::new("poco poco", "pocopoco", "§11.10b"));

    // §5 dj-2a and §9.6: the days of the week are one word.
    for day in [
        "domingo", "luna", "mars", "ranson", "huebs", "bierna", "sabra",
    ] {
        r.push(Replacement::new(
            format!("dia {day}"),
            format!("dia{day}"),
            "§9.6",
        ));
    }

    PhraseRule::new(
        r,
        "Papiamento writes this as one word: `{correct}` (Ortografia di Papiamento {section}).",
        "Looks for words that \"Ortografia di Papiamento\" (AB 2018 no. 68) writes as one word, such as `sinembargo`, `talbes`, `bonbini`, `malgasta` and `pocopoco`.",
        LintKind::BoundaryError,
    )
}

/// Words that are written apart (§11.3c, §11.9.1b, §11.9.2, §11.11, §12.2b).
pub fn open_compounds() -> PhraseRule {
    let mut r = Vec::new();

    // §11.3c: prepositional constructions are written apart.
    for (wrong, correct) in [
        ("abase di", "a base di"),
        ("acausa di", "a causa di"),
        ("acosto di", "a costo di"),
        ("apesar di", "a pesar di"),
        ("apesar cu", "a pesar cu"),
        ("atraves di", "a traves di"),
        ("enbista di", "en bista di"),
        ("pavia di", "pa via di"),
        ("alo largo", "a lo largo"),
        ("encambio", "en cambio"),
        ("entodo caso", "en todo caso"),
        ("esdecir", "es decir"),
    ] {
        r.push(Replacement::new(wrong, correct, "§11.3c"));
    }

    // §11.9.1b: mal(a) + a basic word is written apart.
    for (wrong, correct) in [
        ("malpara", "mal para"),
        ("malfashi", "mal fashi"),
        ("malfama", "mal fama"),
    ] {
        r.push(Replacement::new(wrong, correct, "§11.9.1b"));
    }

    // §11.9.2: bon + a basic word is written apart.
    for (wrong, correct) in [
        ("bondia", "bon dia"),
        ("bonnochi", "bon nochi"),
        ("bontardi", "bon tardi"),
    ] {
        r.push(Replacement::new(wrong, correct, "§11.9.2"));
    }

    // §11.11: multi-word geographic names are written apart, and (§12.2b) without hyphen.
    for correct in [
        "Sur Africa",
        "Sur Africano",
        "Sur America",
        "Sur Americano",
        "Sur Korea",
        "Sur Koreano",
        "Norte America",
        "Norte Americano",
        "Noord Amerika",
        "Noord Korea",
        "Noord Koreano",
        "Latino America",
        "Latino Americano",
        "Medio Oriente",
        "Costa Rica",
        "Puerto Rico",
        "Santo Domingo",
        "Gran Britania",
    ] {
        let lower = correct.to_lowercase();
        r.push(Replacement::new(lower.replace(' ', ""), correct, "§11.11"));
        r.push(Replacement::new(lower.replace(' ', "-"), correct, "§12.2b"));
    }

    PhraseRule::new(
        r,
        "Papiamento writes this apart: `{correct}` (Ortografia di Papiamento {section}).",
        "Looks for words that \"Ortografia di Papiamento\" (AB 2018 no. 68) writes apart, such as `a pesar di`, `bon dia` and geographic names like `Sur America`.",
        LintKind::BoundaryError,
    )
}

/// §11.8.1a: numbers from 11 to 99 are written as one word, with `i` for the conjunction
/// `y` (`bintiseis`, `trintidos`).
pub fn numerals() -> PhraseRule {
    // (the word on its own, the stem used in compounds)
    let tens = [
        ("dies", "dies"),
        ("binti", "binti"),
        ("trinta", "trinti"),
        ("cuarenta", "cuarenti"),
        ("cincuenta", "cincuenti"),
        ("sesenta", "sesenti"),
        ("setenta", "setenti"),
        ("ochenta", "ochenti"),
        ("nobenta", "nobenti"),
    ];
    // (how the unit may be written, how it is written in the compound)
    let units = [
        ("un", "un"),
        ("uno", "un"),
        ("dos", "dos"),
        ("tres", "tres"),
        ("cuater", "cuater"),
        ("cuatro", "cuatro"),
        ("cinco", "cinco"),
        ("seis", "seis"),
        ("shete", "shete"),
        ("ocho", "ocho"),
        ("nuebe", "nuebe"),
    ];

    let mut r = Vec::new();
    for (alone, stem) in tens {
        for (written, unit) in units {
            let correct = format!("{stem}{unit}");
            let mut firsts = vec![alone];
            if stem != alone {
                firsts.push(stem);
            }
            for first in firsts {
                for joiner in [" y ", " ", "-"] {
                    r.push(Replacement::new(
                        format!("{first}{joiner}{written}"),
                        correct.clone(),
                        "§11.8.1a",
                    ));
                }
            }
        }
    }

    PhraseRule::new(
        r,
        "Numbers from 11 to 99 are written as one word: `{correct}` (Ortografia di Papiamento {section}).",
        "Looks for numbers from 11 to 99 that are written as more than one word, such as `binti y seis` for `bintiseis`.",
        LintKind::BoundaryError,
    )
}

/// §10.1.3: these times of day are always written with an apostrophe.
pub fn clock_times() -> PhraseRule {
    let r = [
        ("cuater or", "cuat'or"),
        ("cuatro or", "cuat'or"),
        ("cinco or", "cinc'or"),
        ("shete or", "shet'or"),
        ("ocho or", "och'or"),
        ("nuebe or", "nueb'or"),
        ("cuanto or", "cuant'or"),
    ]
    .into_iter()
    .map(|(wrong, correct)| Replacement::new(wrong, correct, "§10.1.3"))
    .collect();

    PhraseRule::new(
        r,
        "This time of day is written with an apostrophe: `{correct}` (Ortografia di Papiamento {section}).",
        "Looks for `cuatro or`, `cinco or`, `shete or`, `ocho or`, `nuebe or` and `cuanto or`, which are written `cuat'or`, `cinc'or`, `shet'or`, `och'or`, `nueb'or` and `cuant'or`.",
        LintKind::Punctuation,
    )
}

/// §8.8d: a hyphen follows the prefixes `ex` and `vice`.
pub fn prefix_hyphen() -> PhraseRule {
    let hyphenated = [
        // From the vocabulary.
        "ex-casa",
        "ex-presidente",
        "vice-admiraal",
        "vice-consul",
        "vice-presidencia",
        "vice-president",
        "vice-presidente",
        "vice-rector",
        "vice-secretario",
        // Examples in §5 x-1e and §8.8d.
        "ex-minister",
    ];

    let mut r = Vec::new();
    for correct in hyphenated {
        r.push(Replacement::new(
            correct.replace('-', " "),
            correct,
            "§8.8d",
        ));
        r.push(Replacement::new(correct.replace('-', ""), correct, "§8.8d"));
    }

    PhraseRule::new(
        r,
        "Write a hyphen after the prefixes `ex` and `vice`: `{correct}` (Ortografia di Papiamento {section}).",
        "Looks for the prefixes `ex` and `vice` written without a hyphen, such as `vice presidente` for `vice-presidente`.",
        LintKind::Punctuation,
    )
}

#[cfg(test)]
mod tests {
    use harper_core::spell::Dictionary;

    use super::*;
    use crate::dictionary;

    /// Every correction we suggest must itself pass the spell checker's dictionary.
    #[test]
    fn corrections_are_in_the_vocabulary() {
        let dict = dictionary::curated();
        for rule in [
            closed_compounds(),
            open_compounds(),
            numerals(),
            clock_times(),
            prefix_hyphen(),
        ] {
            for replacement in rule.replacements() {
                for word in replacement.correct.split([' ', '-']) {
                    assert!(
                        dict.contains_word_str(word),
                        "`{}` (from `{}`) is not in the dictionary",
                        word,
                        replacement.correct
                    );
                }
            }
        }
    }

    #[test]
    fn no_wrong_form_is_a_correct_form() {
        let dict = dictionary::curated();
        for rule in [closed_compounds(), open_compounds()] {
            for replacement in rule.replacements() {
                if !replacement.wrong.contains([' ', '-']) {
                    assert!(
                        !dict.contains_word_str(&replacement.wrong),
                        "`{}` is in the vocabulary",
                        replacement.wrong
                    );
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod rule_tests;
