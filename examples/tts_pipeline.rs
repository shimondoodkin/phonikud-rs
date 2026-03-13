/*
Full TTS pre-processing pipeline demo — no audio rendering, just prints each step.

Shows how LightBlue TTS uses phonikud-rs:
  1. Load custom dictionaries from disk
  2. Expand text (dates, numbers, symbols → Hebrew words) with span tracking
  3. Split into paragraphs (for BERT 512-token limit)
  4. Diacritize each paragraph
  5. Compute per-word timing events using span tracking + IPA length estimation

Run with:
    wget https://huggingface.co/thewh1teagle/phonikud-onnx/resolve/main/phonikud-1.0.int8.onnx -O phonikud.onnx
    wget https://huggingface.co/dicta-il/dictabert-large-char-menaked/raw/main/tokenizer.json -O tokenizer.json
    cargo run --example tts_pipeline
*/

use phonikud_rs::{expander, Phonikud};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // --- Step 1: Load custom dictionaries (optional) ---
    let dict_dir = Path::new("./dictionaries");
    if dict_dir.is_dir() {
        let n = expander::dictionary::load_extra_dictionaries(dict_dir);
        println!("=== Dictionaries ===");
        println!("Loaded {} custom entries from {}\n", n, dict_dir.display());
    }

    // --- Input text ---
    let original = "\
בתאריך 14/03/2026 המחיר הוא 50 ₪.
זה לא 100 $ אלא חמישים שקלים בלבד!

צה״ל הודיע היום כי הפעולה הצליחה.
יאללה, בוא נחגוג ב-15:30.";

    println!("=== Original text ===");
    println!("{}\n", original);

    // --- Step 2: Expand text (dates, numbers, symbols, abbreviations) ---
    let expanded = expander::expand_text_with_spans(original);
    println!("=== Expanded text ===");
    println!("{}\n", expanded.text);

    // Show what changed
    println!("=== Expansion details ===");
    for token in &expanded.tokens {
        if token.original_text != token.expanded_text {
            println!(
                "  '{}' [{}..{}] → '{}' [{}..{}]",
                token.original_text,
                token.original_span.start,
                token.original_span.end,
                token.expanded_text,
                token.expanded_span.start,
                token.expanded_span.end,
            );
        }
    }
    println!();

    // --- Step 3: Split into paragraphs ---
    let paragraphs: Vec<&str> = expanded
        .text
        .split("\n\n")
        .flat_map(|p| p.split("\r\n\r\n"))
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    println!("=== Paragraphs ({}) ===", paragraphs.len());
    for (i, para) in paragraphs.iter().enumerate() {
        println!("  [{}] {}", i + 1, para);
    }
    println!();

    // --- Step 4: Diacritize each paragraph ---
    let mut phonikud = Phonikud::new("phonikud.onnx", "tokenizer.json")?;

    println!("=== Diacritized output ===");
    let mut full_diacritized = String::new();
    for (i, para) in paragraphs.iter().enumerate() {
        let t0 = std::time::Instant::now();
        let diacritized = phonikud.add_diacritics(para)?;
        let elapsed = t0.elapsed();
        println!("  [{}] ({:?}) {}", i + 1, elapsed, diacritized);

        if !full_diacritized.is_empty() {
            full_diacritized.push(' ');
        }
        full_diacritized.push_str(&diacritized);
    }
    println!();

    // --- Step 5: Word timing events using span tracking ---
    //
    // In a real TTS engine (like SAPI 5), you need to fire word boundary events
    // so screen readers and apps can highlight the current word. The span tracking
    // from expand_text_with_spans lets you map back to the original text positions.
    //
    // The idea:
    //   - Each token from the expander knows its original position in the input text
    //   - After TTS renders audio for a sentence, you know the total audio duration
    //   - Distribute time across words proportionally to their IPA phoneme count
    //   - Fire events at the computed timestamps with original text offsets
    //
    // This gives accurate word highlighting even when "50 ₪" expanded to
    // "חמישים שקל" — the event still points at position of "50" in the original.

    println!("=== Word timing events (simulated) ===");
    println!("  Assuming 22050 Hz sample rate, ~80ms per IPA phoneme\n");

    let sample_rate = 22050_u32;
    let ms_per_phoneme = 80.0_f64; // rough estimate

    // Simulate per-sentence processing (split expanded text at sentence punctuation)
    let sentences: Vec<&str> = expanded.text
        .split_inclusive(|c| c == '.' || c == '!' || c == '?')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut cumulative_audio_ms = 0.0_f64;

    for (sent_idx, sentence) in sentences.iter().enumerate() {
        println!("  --- Sentence {} ---", sent_idx + 1);

        // Find which tokens belong to this sentence by checking if their
        // expanded_span overlaps with the sentence's position in expanded.text
        let sent_start = expanded.text.find(sentence).unwrap_or(0);
        let sent_end = sent_start + sentence.len();

        let sentence_tokens: Vec<_> = expanded.tokens.iter()
            .filter(|t| t.expanded_span.start >= sent_start && t.expanded_span.end <= sent_end)
            .collect();

        // Count total IPA-like characters for proportional timing
        // (In a real system you'd use actual IPA from the phonemizer)
        let total_phonemes: usize = sentence_tokens.iter()
            .map(|t| count_phoneme_chars(&t.expanded_text))
            .sum();
        let sentence_duration_ms = total_phonemes as f64 * ms_per_phoneme;

        let mut word_offset_ms = cumulative_audio_ms;

        for token in &sentence_tokens {
            let phonemes = count_phoneme_chars(&token.expanded_text);
            let word_duration_ms = if total_phonemes > 0 {
                sentence_duration_ms * (phonemes as f64 / total_phonemes as f64)
            } else {
                0.0
            };

            // Convert to audio byte offset (16-bit PCM, mono)
            let byte_offset = (word_offset_ms / 1000.0 * sample_rate as f64 * 2.0) as u64;

            println!(
                "    {:6.0}ms  byte:{:>8}  orig[{}..{}] '{}' → '{}'",
                word_offset_ms,
                byte_offset,
                token.original_span.start,
                token.original_span.end,
                token.original_text,
                token.expanded_text,
            );

            word_offset_ms += word_duration_ms;
        }

        cumulative_audio_ms = word_offset_ms;

        // In a real TTS, add inter-sentence silence (e.g. 250ms)
        let silence_ms = 250.0;
        cumulative_audio_ms += silence_ms;
        println!("    {:6.0}ms  [silence {}ms]\n", cumulative_audio_ms - silence_ms, silence_ms as u32);
    }

    // --- Summary ---
    println!("=== Pipeline summary ===");
    println!("  Original chars:    {}", original.len());
    println!("  Expanded chars:    {}", expanded.text.len());
    println!("  Tokens expanded:   {}", expanded.tokens.len());
    println!(
        "  Tokens changed:    {}",
        expanded
            .tokens
            .iter()
            .filter(|t| t.original_text != t.expanded_text)
            .count()
    );
    println!("  Paragraphs:        {}", paragraphs.len());
    println!("  Diacritized chars: {}", full_diacritized.len());
    println!("  Estimated audio:   {:.0}ms", cumulative_audio_ms);

    Ok(())
}

/// Rough phoneme count for timing estimation.
/// Counts Hebrew letters (ignoring diacritics) and Latin letters.
/// In a real TTS pipeline you'd use actual IPA phoneme counts.
fn count_phoneme_chars(text: &str) -> usize {
    text.chars()
        .filter(|c| {
            // Hebrew consonants (U+05D0..U+05EA)
            ('\u{05D0}'..='\u{05EA}').contains(c)
            // Latin letters
            || c.is_ascii_alphabetic()
            // Digits (each digit word has been expanded, but just in case)
            || c.is_ascii_digit()
        })
        .count()
        .max(1) // at least 1 so every word gets some duration
}
