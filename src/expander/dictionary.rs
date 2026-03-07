//! Dictionary-based word expansion.
//!
//! Loads JSON dictionaries (symbols, special words, abbreviations) that map
//! Hebrew words or symbols directly to IPA phonemes, bypassing phonemization.
//!
//! Mirrors phonikud's `dictionary.py` and `data/*.json` files.
//!
//! Built-in dictionaries are embedded at compile time.

use std::collections::HashMap;
use std::sync::LazyLock;

// Embed the built-in JSON dictionaries at compile time.
const SYMBOLS_JSON: &str = include_str!("data/symbols.json");
const SPECIAL_JSON: &str = include_str!("data/special.json");
const RASHEJ_TEVOT_JSON: &str = include_str!("data/rashej_tevot.json");

/// Combined dictionary: key = source word/symbol, value = IPA phonemes.
static DICTIONARY: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    let mut dict = HashMap::new();

    // Built-in dictionaries
    for json_str in [SYMBOLS_JSON, SPECIAL_JSON, RASHEJ_TEVOT_JSON] {
        if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(json_str) {
            dict.extend(map);
        }
    }

    dict
});

/// Look up a word in the dictionary. Returns IPA phonemes if found.
pub fn lookup(word: &str) -> Option<&str> {
    DICTIONARY.get(word).map(|s| s.as_str())
}

/// Expand known dictionary words in text, replacing them with IPA.
/// Words not in the dictionary are left unchanged.
pub fn expand_dictionary(text: &str) -> String {
    let mut result = Vec::new();
    for word in text.split_whitespace() {
        if let Some(ipa) = lookup(word) {
            result.push(ipa.to_string());
        } else {
            result.push(word.to_string());
        }
    }
    result.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_lookup() {
        assert_eq!(lookup("₪"), Some("ʃˈekel"));
        assert_eq!(lookup("$"), Some("dˈolar"));
        assert_eq!(lookup("%"), Some("axˈuz"));
    }

    #[test]
    fn test_special_lookup() {
        assert_eq!(lookup("יאללה"), Some("jˈala"));
    }

    #[test]
    fn test_abbreviation_lookup() {
        assert_eq!(lookup("צה״ל"), Some("tsˈahal"));
    }

    #[test]
    fn test_unknown() {
        assert_eq!(lookup("שלום"), None);
    }

    #[test]
    fn test_expand() {
        let r = expand_dictionary("מחיר 50 ₪");
        assert!(r.contains("ʃˈekel"), "got: {r}");
    }
}
