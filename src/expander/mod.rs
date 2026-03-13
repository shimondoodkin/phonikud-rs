//! Text expander — expand dates, numbers, and times into Hebrew words with nikud.
//!
//! Mirrors the structure of phonikud's Python expander:
//! <https://github.com/thewh1teagle/phonikud/tree/main/phonikud/expander>

pub mod dates;
pub mod dictionary;
pub mod hebrew_chars;
pub mod numbers;
pub mod punctuation;
pub mod time;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct ExpandedToken {
    pub original_span: TextSpan,
    pub expanded_span: TextSpan,
    pub original_text: String,
    pub expanded_text: String,
}

#[derive(Debug, Clone)]
pub struct ExpandedText {
    pub text: String,
    pub tokens: Vec<ExpandedToken>,
}

/// Normalize unicode characters: replace newlines with spaces, normalize
/// various dashes, quotes, and whitespace to ASCII equivalents.
fn normalize_unicode(text: &str) -> String {
    let mut s = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            // Newlines / vertical whitespace → space
            '\n' | '\r' | '\x0B' | '\x0C' => s.push(' '),
            // Non-breaking space, thin space, hair space, em space, en space, etc.
            '\u{00A0}' | '\u{2002}' | '\u{2003}' | '\u{2004}' | '\u{2005}'
            | '\u{2006}' | '\u{2007}' | '\u{2008}' | '\u{2009}' | '\u{200A}'
            | '\u{202F}' | '\u{205F}' | '\u{3000}' => s.push(' '),
            // Zero-width spaces — drop them
            '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' => {}
            // Em dash, en dash → hyphen
            '\u{2013}' | '\u{2014}' => s.push('-'),
            // Smart quotes → ASCII quotes
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => s.push('\''),
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => s.push('"'),
            // Ellipsis → three dots
            '\u{2026}' => s.push_str("..."),
            // Bullet → hyphen
            '\u{2022}' => s.push('-'),
            // Everything else passes through
            _ => s.push(c),
        }
    }
    // Collapse multiple spaces into one
    let mut result = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        if c == ' ' {
            if !prev_space {
                result.push(' ');
            }
            prev_space = true;
        } else {
            prev_space = false;
            result.push(c);
        }
    }
    result.trim().to_string()
}

fn expand_token_pipeline(text: &str) -> String {
    // Dictionary lookup first — if the whole token maps to IPA, use it directly
    if let Some(ipa) = dictionary::lookup(text) {
        return ipa;
    }
    let mut result = text.to_string();
    result = punctuation::expand_punctuation(&result);
    result = hebrew_chars::expand_geresh(&result);
    result = dates::expand_dates(&result);
    result = time::expand_times(&result);
    result = numbers::expand_numbers(&result);
    result
}

fn tokenize_with_spans(text: &str) -> Vec<(TextSpan, String)> {
    let mut tokens = Vec::new();
    let mut current_start: Option<usize> = None;

    for (idx, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(start) = current_start.take() {
                tokens.push((
                    TextSpan { start, end: idx },
                    text[start..idx].to_string(),
                ));
            }
        } else if current_start.is_none() {
            current_start = Some(idx);
        }
    }

    if let Some(start) = current_start {
        tokens.push((
            TextSpan {
                start,
                end: text.len(),
            },
            text[start..].to_string(),
        ));
    }

    tokens
}

pub fn expand_text_with_spans(text: &str) -> ExpandedText {
    let normalized_input = normalize_unicode(text);
    let mut expanded = String::new();
    let mut tokens = Vec::new();

    for (original_span, token_text) in tokenize_with_spans(&normalized_input) {
        let expanded_text = expand_token_pipeline(&token_text);
        if expanded_text.is_empty() {
            continue;
        }

        if !expanded.is_empty() {
            expanded.push(' ');
        }
        let start = expanded.len();
        expanded.push_str(&expanded_text);
        let end = expanded.len();

        tokens.push(ExpandedToken {
            original_span,
            expanded_span: TextSpan { start, end },
            original_text: token_text,
            expanded_text,
        });
    }

    ExpandedText { text: expanded, tokens }
}

/// Expand dates, times, and numbers in `text` to Hebrew words (with nikud).
/// This should run before phonemization.
pub fn expand_text(text: &str) -> String {
    expand_text_with_spans(text).text
}
