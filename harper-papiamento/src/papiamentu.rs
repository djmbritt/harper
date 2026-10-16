//! Mapping from Curaçao/Bonaire spelling (Papiamentu) to Aruban spelling (Papiamento).
//!
//! Aruba spells etymologically, Curaçao and Bonaire phonologically, so the same word is
//! often written differently on the islands (`kas` vs `cas`, `edukashon` vs `educacion`).
//! Each transformation below follows a rule of "Ortografia di Papiamento"
//! (AB 2018 no. 68). Because several of them are only *sometimes* applicable, we generate
//! every combination and let the caller keep the candidates that are in the dictionary.

use hashbrown::HashSet;

/// Upper bound on generated candidates, to keep pathological inputs cheap.
const MAX_CANDIDATES: usize = 64;

/// Whether `word` shows a feature that Aruban spelling does not use, so that it is worth
/// trying [`aruban_candidates`] on it.
pub fn looks_like_papiamentu(word: &str) -> bool {
    let lower = word.to_lowercase();
    lower.contains(['è', 'ò', 'ù', 'ü'])
        || lower.contains('k')
        || lower.ends_with("shon")
        || lower.ends_with('u')
        || lower.ends_with("dat")
        || lower.ends_with("tat")
        || lower.ends_with("tut")
        || ["ai", "ei", "oi", "ui"].iter().any(|d| lower.ends_with(d))
        || lower.contains("si")
        || lower.contains("se")
}

/// Generate possible Aruban spellings of a (lowercase) Curaçao/Bonaire spelling.
///
/// The input itself is never returned.
pub fn aruban_candidates(word: &str) -> Vec<String> {
    let lower = word.to_lowercase();

    // Deterministic rewrites first.
    let mut base: String = lower
        .chars()
        .map(|c| match c {
            // §1: the Aruban alphabet has no è, ò, ù or ü.
            'è' => 'e',
            'ò' => 'o',
            'ù' | 'ü' => 'u',
            c => c,
        })
        .collect();

    // §5 x-1: [ks] inside a word is written x (final ks stays, §5 x-2: boks, deks).
    if let Some(stripped) = base.strip_suffix("ks") {
        base = format!("{}ks", stripped.replace("ks", "x"));
    } else {
        base = base.replace("ks", "x");
    }

    // §3.1: a rising diphthong after a consonant is written with u (cuater, not kwater).
    base = base.replace("kw", "cu");

    // §5 c-1: [k] before a, o, u and before l, n, r, t is written c.
    base = rewrite_k(&base);

    // §5 d-2: the suffixes [dat], [tat], [tut] are written -dad, -tad, -tud.
    for (from, to) in [("dat", "dad"), ("tat", "tad"), ("tut", "tud")] {
        if let Some(stem) = base.strip_suffix(from) {
            base = format!("{stem}{to}");
        }
    }

    let mut candidates: Vec<String> = vec![base];

    // §5 sh, exception 1 and 2: nouns ending in [ʃoŋ] are written -cion or -sion.
    expand(&mut candidates, |w| {
        w.strip_suffix("shon")
            .map(|stem| vec![format!("{stem}cion"), format!("{stem}sion")])
            .unwrap_or_default()
    });

    // §3.2: falling diphthongs at the end of a word are written with y.
    expand(&mut candidates, |w| {
        for (from, to) in [("ai", "ay"), ("ei", "ey"), ("oi", "oy"), ("ui", "uy")] {
            if let Some(stem) = w.strip_suffix(from) {
                return vec![format!("{stem}{to}")];
            }
        }
        Vec::new()
    });

    // §2: a final [o]/[u] sound is written o (Papiamentu → Papiamento).
    expand(&mut candidates, |w| {
        w.strip_suffix('u')
            .filter(|stem| stem.chars().count() >= 2)
            .map(|stem| vec![format!("{stem}o")])
            .unwrap_or_default()
    });

    // §5 c-2: [s] before e or i is often written c (hasi → haci, sinta → cinta).
    expand(&mut candidates, |w| {
        let chars: Vec<char> = w.chars().collect();
        let mut out = Vec::new();
        for i in 0..chars.len().saturating_sub(1) {
            if chars[i] == 's'
                && matches!(chars[i + 1], 'e' | 'i')
                && (i == 0 || chars[i - 1] != 's')
            {
                let mut changed = chars.clone();
                changed[i] = 'c';
                out.push(changed.into_iter().collect());
            }
        }
        out
    });

    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter(|c| *c != lower && seen.insert(c.clone()))
        .collect()
}

/// Apply `alternatives` to every candidate, keeping the originals as well.
fn expand(candidates: &mut Vec<String>, alternatives: impl Fn(&str) -> Vec<String>) {
    let mut extra = Vec::new();
    for candidate in candidates.iter() {
        for alt in alternatives(candidate) {
            if candidates.len() + extra.len() >= MAX_CANDIDATES {
                break;
            }
            extra.push(alt);
        }
    }
    candidates.extend(extra);
}

fn rewrite_k(word: &str) -> String {
    let chars: Vec<char> = word.chars().collect();
    chars
        .iter()
        .enumerate()
        .map(|(i, &c)| match (c, chars.get(i + 1)) {
            ('k', Some('a' | 'o' | 'u' | 'l' | 'n' | 'r' | 't')) => 'c',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::aruban_candidates;

    fn has(word: &str, expected: &str) {
        let candidates = aruban_candidates(word);
        assert!(
            candidates.iter().any(|c| c == expected),
            "{word} -> {candidates:?} does not contain {expected}"
        );
    }

    #[test]
    fn k_before_a_o_u() {
        has("kas", "cas");
        has("kuminda", "cuminda");
        has("kosta", "costa");
    }

    #[test]
    fn keeps_k_before_e_i() {
        assert!(aruban_candidates("kere").iter().all(|c| c.starts_with('k')));
    }

    #[test]
    fn shon_to_cion() {
        has("edukashon", "educacion");
        has("televishon", "television");
    }

    #[test]
    fn final_u_to_o() {
        has("papiamentu", "papiamento");
    }

    #[test]
    fn accents() {
        has("bèrdè", "berde");
        has("kòrda", "corda");
    }

    #[test]
    fn diphthongs() {
        has("kabei", "cabey");
    }

    #[test]
    fn s_to_c() {
        has("hasi", "haci");
    }

    #[test]
    fn kw_to_cu() {
        has("kwater", "cuater");
    }

    #[test]
    fn suffix_dat() {
        has("komunidat", "comunidad");
    }

    #[test]
    fn ks_inside_word() {
        has("eksamen", "examen");
        assert!(aruban_candidates("deks").iter().all(|c| c.ends_with("ks")));
    }
}
