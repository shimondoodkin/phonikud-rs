# phonikud-rs

Add diacritics to Hebrew text along with phonetic marks.

Rust port of the [phonikud](https://github.com/thewh1teagle/phonikud) project

## Features

- **Diacritization**: Adds nikud (vowel points) to unvocalized Hebrew text using a DictaBERT ONNX model
- **Text expansion**: Expands dates, times, numbers, symbols, and abbreviations into Hebrew words before diacritization
- **Dictionaries**: Built-in word→IPA dictionaries for symbols (`₪` → shekel), special words, and abbreviations (`צה״ל` → tsahal), with support for loading custom dictionaries from disk
- **Fast**: ~0.1s per sentence (macOS M1)
- **Lightweight**: Runs with ONNX Runtime, no heavy dependencies

## Setup

Add to your `Cargo.toml`:

```toml
[dependencies]
phonikud-rs = { git = "https://github.com/shimondoodkin/phonikud-rs.git" }
```

Download the required model files:

```console
wget https://huggingface.co/thewh1teagle/phonikud-onnx/resolve/main/phonikud-1.0.int8.onnx -O phonikud.onnx
wget https://huggingface.co/dicta-il/dictabert-large-char-menaked/raw/main/tokenizer.json -O tokenizer.json
```

## Usage

### Basic — add diacritics

```rust
use phonikud_rs::Phonikud;

fn main() -> anyhow::Result<()> {
    let mut phonikud = Phonikud::new("./phonikud.onnx", "./tokenizer.json")?;
    let text = "שלום עולם";
    let vocalized = phonikud.add_diacritics(text)?;
    println!("{}", vocalized); // שָׁלוֹם עוֹלָם
    Ok(())
}
```

### Mark matres lectionis (nikud male)

You can mark matres lectionis (אמות קריאה) with a special Unicode character to distinguish them from consonantal letters. This is useful for phonemization — knowing which letters are silent vowel markers vs. pronounced consonants.

```rust
let vocalized = phonikud.add_diacritics_with_options(text, Some("\u{05af}"))?;
// The marker \u{05af} (Hebrew accent munah) is placed on matres lectionis letters
```

### Text expansion (pre-processing)

The `expander` module normalizes text before diacritization — expanding dates, times, numbers, punctuation, and dictionary words into Hebrew words:

```rust
use phonikud_rs::expander;

// Expand all known patterns (dates, times, numbers, symbols, abbreviations)
let expanded = expander::expand_text("התאריך הוא 14/03/2026 והמחיר 50 ₪");
// Numbers, dates, and ₪ symbol are expanded to Hebrew words
```

#### Expansion with span tracking

If you need to map expanded text back to original character positions (e.g. for word boundary events in TTS):

```rust
use phonikud_rs::expander;

let result = expander::expand_text_with_spans("מחיר 50 ₪");
println!("Expanded: {}", result.text);
for token in &result.tokens {
    println!(
        "  '{}' (orig {}..{}) -> '{}' (expanded {}..{})",
        token.original_text,
        token.original_span.start, token.original_span.end,
        token.expanded_text,
        token.expanded_span.start, token.expanded_span.end,
    );
}
```

### Dictionaries

Built-in dictionaries map symbols, special words, and abbreviations directly to IPA phonemes. These are embedded at compile time from `src/expander/data/*.json`.

#### Loading custom dictionaries at runtime

You can load additional `.json` dictionary files from a directory. Each file should be a JSON object mapping words to IPA:

```json
{
    "ביבי": "bˈibi",
    "נתב״ג": "natbˈag"
}
```

Load them at startup:

```rust
use std::path::Path;
use phonikud_rs::expander::dictionary;

// Load all .json files from the dictionaries/ folder
let count = dictionary::load_extra_dictionaries(Path::new("./dictionaries"));
println!("Loaded {} custom dictionary entries", count);
```

Custom entries override built-in ones. This lets users add names, brand names, slang, or domain-specific abbreviations without recompiling.

#### Dictionary lookup

```rust
use phonikud_rs::expander::dictionary;

if let Some(ipa) = dictionary::lookup("₪") {
    println!("₪ -> {}", ipa); // ʃˈekel
}
```

#### Built-in dictionary files

| File | Contents |
|------|----------|
| `symbols.json` | Currency and math symbols (`₪`, `$`, `%`) |
| `special.json` | Loanwords and slang (`יאללה`, `וואצאפ`, `פינגוין`) |
| `rashej_tevot.json` | Hebrew abbreviations (`צה״ל`) |

## Real-world integration: TTS pipeline

This example shows how [LightBlue TTS](https://github.com/shimondoodkin/light-blue-tts-sapi) uses phonikud-rs in a full Hebrew text-to-speech pipeline. The key patterns are:

1. **Expand first, diacritize second** — numbers, dates, and symbols become Hebrew words before the BERT model sees them
2. **Paragraph chunking** — DictaBERT has a 512-token limit, so long text is split at paragraph boundaries (`\n\n`) before diacritization, preserving sentence context for the model
3. **Span tracking** — `expand_text_with_spans` maps each expanded word back to its position in the original text, enabling accurate word-boundary events during audio playback

```rust
use phonikud_rs::{Phonikud, expander};

// Step 1: Load custom dictionaries (user-editable, on disk)
let dict_dir = install_dir.join("dictionaries");
let n = expander::dictionary::load_extra_dictionaries(&dict_dir);
println!("Loaded {} custom dictionary entries", n);

// Step 2: Initialize the diacritizer
let mut phonikud = Phonikud::new("phonikud.onnx", "tokenizer.json")?;

// Step 3: Expand text — dates, numbers, symbols, abbreviations → Hebrew words
// expand_text_with_spans returns both the expanded string and a token map
// so you can trace each expanded word back to its original position
let original = "בתאריך 14/03/2026 המחיר הוא 50 ₪ ולא 100 $.";
let expanded = expander::expand_text_with_spans(original);
println!("Expanded: {}", expanded.text);

// Each token maps original_span → expanded_span
// e.g. "50" at position 29..31 might expand to "חמישים" at position 40..46
for token in &expanded.tokens {
    if token.original_text != token.expanded_text {
        println!(
            "  '{}' [{}..{}] → '{}' [{}..{}]",
            token.original_text,
            token.original_span.start, token.original_span.end,
            token.expanded_text,
            token.expanded_span.start, token.expanded_span.end,
        );
    }
}

// Step 4: Split into paragraphs for diacritization (BERT 512-token limit)
// Each paragraph is diacritized independently while keeping full sentence
// context for the model (better accuracy than sentence-by-sentence)
let paragraphs: Vec<&str> = expanded.text
    .split("\n\n")
    .map(|p| p.trim())
    .filter(|p| !p.is_empty())
    .collect();

let mut diacritized_full = String::new();
for para in &paragraphs {
    let diacritized = phonikud.add_diacritics(para)?;
    if !diacritized_full.is_empty() {
        diacritized_full.push(' ');
    }
    diacritized_full.push_str(&diacritized);
}
println!("Diacritized: {}", diacritized_full);

// Step 5: The diacritized text can now be phonemized to IPA for TTS synthesis
// (phonemization is outside phonikud-rs — see lightblue-sapi for that part)
```

### Pipeline diagram

```
Original text
    │
    ▼
┌─────────────────────────┐
│ expander::expand_text   │  "50 ₪" → "חמישים שקל"
│ with_spans()            │  + span map: orig pos → expanded pos
└─────────────────────────┘
    │
    ▼
┌─────────────────────────┐
│ Split into paragraphs   │  Stay within BERT's 512-token limit
│ (split on \n\n)         │  while keeping sentence context
└─────────────────────────┘
    │
    ▼
┌─────────────────────────┐
│ phonikud.add_diacritics │  "שלום עולם" → "שָׁלוֹם עוֹלָם"
│ (per paragraph)         │  DictaBERT ONNX inference
└─────────────────────────┘
    │
    ▼
  Diacritized Hebrew text
  (ready for phonemization / TTS)
```

## Module overview

| Module | Purpose |
|--------|---------|
| `Phonikud` | Main struct — loads ONNX model, runs diacritization |
| `expander` | Pre-processing: dates, times, numbers, punctuation, dictionary expansion |
| `expander::dictionary` | Word→IPA dictionaries (built-in + runtime) |

## Note

This crate uses `ndarray 0.17` which is incompatible with `ndarray 0.16`. If your project depends on `ndarray 0.16`, you will need to upgrade to `0.17`.

## Examples

See [examples](examples) for runnable code.
