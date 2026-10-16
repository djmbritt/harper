//! The curated Papiamento dictionary.

use std::sync::{Arc, LazyLock};

use harper_core::spell::{FstDictionary, MutableDictionary};

static DICT: LazyLock<Arc<FstDictionary>> = LazyLock::new(|| {
    let mutable = MutableDictionary::from_rune_files(
        include_str!("../dictionary.dict"),
        include_str!("../annotations.json"),
    )
    .expect("The Papiamento dictionary should be well-formed.");

    Arc::new(mutable.into())
});

/// The Papiamento dictionary compiled into the binary.
///
/// It is generated from the "Anexo: Vocabulario di Papiamento" of the
/// Landsverordening schrijfwijze Papiamento (Afkondigingsblad van Aruba 2018 no. 68).
/// See `scripts/build_dictionary.py`.
pub fn curated() -> Arc<FstDictionary> {
    DICT.clone()
}

#[cfg(test)]
mod tests {
    use harper_core::spell::Dictionary;

    use super::curated;

    #[test]
    fn loads() {
        assert!(curated().word_count() > 16_000);
    }

    #[test]
    fn contains_common_words() {
        let dict = curated();
        for word in [
            "papia",
            "mucha",
            "cas",
            "dushi",
            "bonbini",
            "Papiamento",
            "dialuna",
            "awor",
            "a",
            "or",
            "Aruba",
            "San",
            "Nicolas",
        ] {
            assert!(dict.contains_word_str(word), "missing {word}");
        }
    }

    #[test]
    fn is_case_insensitive() {
        assert!(curated().contains_word_str("Cas"));
    }
}
