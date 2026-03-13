//! Dictionary-based word expansion.
//!
//! Loads JSON dictionaries (symbols, special words, abbreviations) that map
//! Hebrew words or symbols directly to IPA phonemes, bypassing phonemization.
//!
//! Mirrors phonikud's `dictionary.py` and `data/*.json` files.
//!
//! Built-in dictionaries are embedded at compile time. Additional dictionaries
//! can be loaded from disk at runtime via [`load_extra_dictionaries`].

use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, RwLock};

// Embed the built-in JSON dictionaries at compile time.
const SYMBOLS_JSON: &str = include_str!("data/symbols.json");
const SPECIAL_JSON: &str = include_str!("data/special.json");
const RASHEJ_TEVOT_JSON: &str = include_str!("data/rashej_tevot.json");

/// Combined dictionary: key = source word/symbol, value = IPA phonemes.
static DICTIONARY: LazyLock<RwLock<HashMap<String, String>>> = LazyLock::new(|| {
    let mut dict = HashMap::new();

    // Built-in dictionaries
    for json_str in [SYMBOLS_JSON, SPECIAL_JSON, RASHEJ_TEVOT_JSON] {
        if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(json_str) {
            dict.extend(map);
        }
    }

    RwLock::new(dict)
});

/// Load additional dictionaries from a directory on disk.
///
/// Reads all `.json` files in `dir`, each expected to be a JSON object mapping
/// words/symbols to IPA phonemes. Entries are merged on top of the built-in
/// dictionaries (user entries override built-ins).
///
/// Returns the number of extra entries loaded.
pub fn load_extra_dictionaries(dir: &Path) -> usize {
    let mut count = 0;
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    let mut extras = HashMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&contents) {
                    count += map.len();
                    extras.extend(map);
                }
            }
        }
    }
    if !extras.is_empty() {
        if let Ok(mut dict) = DICTIONARY.write() {
            dict.extend(extras);
        }
    }
    count
}

/// Look up a word in the dictionary. Returns IPA phonemes if found.
pub fn lookup(word: &str) -> Option<String> {
    DICTIONARY.read().ok()?.get(word).cloned()
}

/// Expand known dictionary words in text, replacing them with IPA.
/// Words not in the dictionary are left unchanged.
pub fn expand_dictionary(text: &str) -> String {
    let mut result = Vec::new();
    for word in text.split_whitespace() {
        if let Some(ipa) = lookup(word) {
            result.push(ipa);
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
        assert_eq!(lookup("₪").as_deref(), Some("ʃˈekel"));
        assert_eq!(lookup("$").as_deref(), Some("dˈolar"));
        assert_eq!(lookup("%").as_deref(), Some("axˈuz"));
    }

    #[test]
    fn test_special_lookup() {
        assert_eq!(lookup("יאללה").as_deref(), Some("jˈala"));
    }

    #[test]
    fn test_abbreviation_lookup() {
        assert_eq!(lookup("צה״ל").as_deref(), Some("tsˈahal"));
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
