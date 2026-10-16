//! Tests for each orthography rule, run in isolation from the others.

use harper_core::Document;
use harper_core::linting::{Linter, Suggestion};

use super::*;
use crate::dictionary;

/// Apply the first suggestion of every lint `linter` produces.
fn fix(mut linter: impl Linter, text: &str) -> String {
    let dict = dictionary::curated();
    let document = Document::new_plain_english(text, &dict);
    let mut lints = linter.lint(&document);
    lints.sort_by_key(|l| std::cmp::Reverse(l.span.start));

    let mut chars: Vec<char> = text.chars().collect();
    for lint in lints {
        if let Some(Suggestion::ReplaceWith(with)) = lint.suggestions.first() {
            chars.splice(lint.span.start..lint.span.end, with.iter().copied());
        }
    }
    chars.into_iter().collect()
}

fn count(mut linter: impl Linter, text: &str) -> usize {
    let dict = dictionary::curated();
    let document = Document::new_plain_english(text, &dict);
    linter.lint(&document).len()
}

macro_rules! fixes {
    ($rule:expr, $($from:literal => $to:literal),+ $(,)?) => {
        $( assert_eq!(fix($rule, $from), $to, "fixing {:?}", $from); )+
    };
}

macro_rules! allows {
    ($rule:expr, $($text:literal),+ $(,)?) => {
        $( assert_eq!(count($rule, $text), 0, "{:?} should not be flagged", $text); )+
    };
}

#[test]
fn closed_compounds_true_positives() {
    fixes!(closed_compounds(),
        "Sin embargo, e no a bin." => "Sinembargo, e no a bin.",
        "E no a bin, sin embargo." => "E no a bin, sinembargo.",
        "Tal bes e ta bin mañan." => "Talbes e ta bin mañan.",
        "Bon bini na Aruba!" => "Bonbini na Aruba!",
        "BON BINI" => "BONBINI",
        "Nan ta mal gasta placa." => "Nan ta malgasta placa.",
        "No mal trata bestia." => "No maltrata bestia.",
        "E mucha ta mal cria." => "E mucha ta malcria.",
        "Su mal criansa ta conoci." => "Su malcriansa ta conoci.",
        "Bay poco poco." => "Bay pocopoco.",
        "Mi ta bin dia luna." => "Mi ta bin dialuna.",
        "Dia sabra nos ta hunga." => "Diasabra nos ta hunga.",
        "Por fin e a yega." => "Porfin e a yega.",
        "E ta un hende mal agradecido." => "E ta un hende malagradecido.",
        "Awor aki mi tin tempo." => "Aworaki mi tin tempo.",
        "Mi ta warda aki banda." => "Mi ta warda akibanda.",
        "Bay pa fo!" => "Bay pafo!",
        "Mi ta cumpra pan bati." => "Mi ta cumpra panbati.",
        "Bon bida na bo!" => "Bonbida na bo!",
        "En cuanto e yega, nos ta come." => "Encuanto e yega, nos ta come.",
    );
}

#[test]
fn closed_compounds_true_negatives() {
    allows!(
        closed_compounds(),
        "Sinembargo, e no a bin.",
        "Talbes e ta bin.",
        "Bonbini na Aruba!",
        // §11.9.1a, exception: the noun phrase `mal uzo` is written apart.
        "Esey ta un mal uzo di placa.",
        // §11.9.1b: `mal trato` is not the verb maltrata.
        "Nos a ricibi un mal trato.",
        // §11.9.2: other combinations with bon are written apart.
        "Bon dia, bon nochi.",
        // `aki den` can mean "here in", so it is not a rule.
        "Mi ta biba aki den Aruba.",
        // `por` (can) followed by `nada` is not `pornada`.
        "Mi no por nada.",
        // `poco` alone.
        "Mi tin poco tempo.",
        "E dia ei ta bunita.",
        "Un mal hende.",
        "Mi ta bisa: tal y tal.",
        "Nos ta bay pa nan cas.",
        "Mi tin un pan grandi y un pan chikito.",
        "Sin bo, mi no ta bay.",
    );
}

#[test]
fn open_compounds_true_positives() {
    fixes!(open_compounds(),
        "Apesar di tur cos, e a bin." => "A pesar di tur cos, e a bin.",
        "E a bin apesar di e awaceru." => "E a bin a pesar di e awaceru.",
        "E a bin apesar cu e tabata malo." => "E a bin a pesar cu e tabata malo.",
        "Abase di esaki nos ta sigui." => "A base di esaki nos ta sigui.",
        "Atraves di e anja." => "A traves di e anja.",
        "Enbista di e situacion." => "En bista di e situacion.",
        "Encambio, e no a papia." => "En cambio, e no a papia.",
        "Bondia, con ta bay?" => "Bon dia, con ta bay?",
        "Bonnochi tur hende." => "Bon nochi tur hende.",
        "Bontardi!" => "Bon tardi!",
        "Nan ta biba na Suramerica." => "Nan ta biba na Sur America.",
        "Nan ta biba na Noord-Korea." => "Nan ta biba na Noord Korea.",
        "E ta bin di Latinoamerica." => "E ta bin di Latino America.",
        "Mi a bishita Costarica." => "Mi a bishita Costa Rica.",
        "Puertorico ta bunita." => "Puerto Rico ta bunita.",
        "E tin malfama." => "E tin mal fama.",
    );
}

#[test]
fn open_compounds_true_negatives() {
    allows!(
        open_compounds(),
        "A pesar di tur cos, e a bin.",
        "Bon dia, con ta bay?",
        "Nan ta biba na Sur America.",
        // §11.11, exceptions written as one word.
        "E ta Costaricense.",
        "E ta Portoriqueño.",
        "E ta Estadounidense.",
        "Mi ta Islaribeño.",
        // The word on its own is fine.
        "E ta un base di pan.",
        "Nan ta gana hopi placa.",
        "Mi ta bay America.",
        "Bon mucha!",
        "En cambio di esey.",
        "A causa di e awaceru.",
        "Es decir, no.",
        "Mi ta papia di Korea.",
    );
}

#[test]
fn numerals_true_positives() {
    fixes!(numerals(),
        "E tin binti y seis aña." => "E tin bintiseis aña.",
        "E tin binti seis aña." => "E tin bintiseis aña.",
        "E tin binti-seis aña." => "E tin bintiseis aña.",
        "E tin trinta y dos aña." => "E tin trintidos aña.",
        "E tin trinta dos aña." => "E tin trintidos aña.",
        "E tin trinti dos aña." => "E tin trintidos aña.",
        "E tin cuarenta y cinco aña." => "E tin cuarenticinco aña.",
        "E tin cincuenta y un aña." => "E tin cincuentiun aña.",
        "E tin sesenta y uno aña." => "E tin sesentiun aña.",
        "E tin setenta y shete aña." => "E tin setentishete aña.",
        "E tin ochenta y ocho aña." => "E tin ochentiocho aña.",
        "E tin nobenta y nuebe aña." => "E tin nobentinuebe aña.",
        "E tin dies y shete aña." => "E tin diesshete aña.",
        "E tin dies y seis aña." => "E tin diesseis aña.",
        "E tin binti y cuater aña." => "E tin binticuater aña.",
        "E tin binti y cuatro aña." => "E tin binticuatro aña.",
        "Binti y tres hende a bin." => "Bintitres hende a bin.",
    );
}

#[test]
fn numerals_true_negatives() {
    allows!(
        numerals(),
        "E tin bintiseis aña.",
        "E tin binti aña.",
        "E tin trinta aña.",
        "Dies hende a bin.",
        "Mi tin dos cas.",
        "Binti, dos, tres.",
        "E tin binti aña y dos yiu.",
        // §11.8.1b: numbers above 100 are written apart.
        "Dos shen y trintiseis.",
        "Bintinuebe mil dos shen y trintitres.",
        "Nobenta porciento.",
        "Cincuenta florin.",
        "Diesseis.",
        "Seis y dos ta ocho.",
        "Trintidos.",
        "Diesshete.",
    );
}

#[test]
fn clock_times_true_positives() {
    fixes!(clock_times(),
        "Mi ta bin cuatro or." => "Mi ta bin cuat'or.",
        "Mi ta bin cuater or." => "Mi ta bin cuat'or.",
        "E ta lanta cinco or." => "E ta lanta cinc'or.",
        "Nos ta come shete or." => "Nos ta come shet'or.",
        "Ocho or mi ta traha." => "Och'or mi ta traha.",
        "E ta drumi nuebe or." => "E ta drumi nueb'or.",
        "Cuanto or e ta bin?" => "Cuant'or e ta bin?",
        "CINCO OR" => "CINC'OR",
    );
    // Seven more variations of spacing and position.
    fixes!(clock_times(),
        "Na cinco or." => "Na cinc'or.",
        "Te ocho or" => "Te och'or",
        "(shete or)" => "(shet'or)",
        "cuanto  or" => "cuant'or",
        "Prome cu nuebe or, bin." => "Prome cu nueb'or, bin.",
        "Di cinco or te ocho or." => "Di cinc'or te och'or.",
        "Cuatro or di mardugada." => "Cuat'or di mardugada.",
    );
}

#[test]
fn clock_times_true_negatives() {
    allows!(
        clock_times(),
        "Mi ta bin cuat'or.",
        "Mi ta bin cuat’or.",
        // §11.8.3: other hours are written apart.
        "E ta lanta seis or.",
        "Nos ta come 7 or.",
        "Dos or.",
        "Tres or.",
        "Dies or.",
        "12 or y mey.",
        "11.30 or.",
        "Mi tin cinco buki.",
        "Ocho hende.",
        "Cuanto e ta costa?",
        "Shete dia.",
        "Nuebe luna.",
        "Cuatro cas.",
    );
}

#[test]
fn prefix_hyphen_true_positives() {
    fixes!(prefix_hyphen(),
        "E vice presidente a papia." => "E vice-presidente a papia.",
        "E vicepresidente a papia." => "E vice-presidente a papia.",
        "Vice presidente Croes." => "Vice-presidente Croes.",
        "E ex minister a bin." => "E ex-minister a bin.",
        "E exminister a bin." => "E ex-minister a bin.",
        "Su ex casa a bin." => "Su ex-casa a bin.",
        "E ex presidente." => "E ex-presidente.",
        "E vice rector." => "E vice-rector.",
        "E vicerector." => "E vice-rector.",
        "E vice consul." => "E vice-consul.",
        "E vice secretario." => "E vice-secretario.",
        "E vice admiraal." => "E vice-admiraal.",
        "E vice presidencia." => "E vice-presidencia.",
        "E vice president." => "E vice-president.",
        "E expresidente." => "E ex-presidente.",
    );
}

#[test]
fn prefix_hyphen_true_negatives() {
    allows!(
        prefix_hyphen(),
        "E vice-presidente a papia.",
        "E ex-minister a bin.",
        // `vice versa` is a fixed expression.
        "Y vice versa.",
        // §8.8d, exception.
        "Nan a excomunica e.",
        // Words starting with ex that are not the prefix.
        "E examen ta facil.",
        "E ta exagera.",
        "E ta experto.",
        "Mi ex ta bin.",
        "E presidente a bin.",
        "E minister a papia.",
        "Esaki ta extra.",
        "E ta exacto.",
        "Exporta.",
        "Mi tin un vice.",
        "E casa ta grandi.",
    );
}

#[test]
fn lowercase_days_months_true_positives() {
    fixes!(LowercaseDaysMonths,
        "E ta bin Dialuna cu su mama." => "E ta bin dialuna cu su mama.",
        "E ta bin riba Diamars." => "E ta bin riba diamars.",
        "Nos ta bay Diaranson." => "Nos ta bay diaranson.",
        "Te Diahuebs!" => "Te diahuebs!",
        "Mi ta traha Diabierna." => "Mi ta traha diabierna.",
        "Nos ta descansa Diasabra." => "Nos ta descansa diasabra.",
        "Nos ta bay misa Diadomingo." => "Nos ta bay misa diadomingo.",
        "E a nace na Mei." => "E a nace na mei.",
        "E a nace na Januari." => "E a nace na januari.",
        "Den Februari." => "Den februari.",
        "Den Maart y April." => "Den maart y april.",
        "Te Juni." => "Te juni.",
        "Na Augustus, September y October." => "Na augustus, september y october.",
        "Den November." => "Den november.",
        "Te December." => "Te december.",
        "Den Juli." => "Den juli.",
    );
}

#[test]
fn lowercase_days_months_true_negatives() {
    allows!(
        LowercaseDaysMonths,
        "Dialuna e ta bin.",
        "Mei ta un luna bunita.",
        "E ta bin dialuna.",
        "E a nace na mei.",
        "Den januari.",
        "Te diahuebs!",
        "Mi ta bin. Diamars mi no ta bin.",
        "Su nomber ta Maria.",
        "E ta bin na Aruba.",
        "Diasabra y diadomingo.",
        "Januari, februari y maart.",
        "Ki dia? Dialuna.",
        "Mi ta bin.",
        "Diabierna!",
        "October ta e di dies luna.",
    );
}
