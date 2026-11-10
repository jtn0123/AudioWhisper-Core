//! Fits a transcript to the text it is inserted after: the Mac app's `CursorContextFormatter`. A
//! space goes between them so words do not run together, and the first word's case is matched to
//! whether the insertion starts or continues a sentence.
//!
//! The recognisers capitalise the first word of every transcript, which is wrong mid-sentence.
//! Lowercasing is limited to common function words, so a name ("Paris") or anything outside
//! English keeps its capital.

use unicode_normalization::UnicodeNormalization;

use crate::swift;

/// `text` as it should be inserted after `preceding`, the text before the cursor if known.
pub fn fit(text: &str, preceding: Option<&str>) -> String {
    let (Some(preceding), Some(first)) = (preceding, swift::first(text)) else {
        return text.to_owned();
    };
    if swift::is_whitespace(first) {
        return text.to_owned();
    }
    let trimmed_before = swift::trim_whitespaces(preceding);
    let result = if trimmed_before.is_empty()
        || swift::last(trimmed_before).is_some_and(swift::is_newline)
        || ends_sentence(trimmed_before)
    {
        capitalizing_first_word(text)
    } else if continues_sentence(trimmed_before) {
        lowercasing_first_word(text)
    } else {
        text.to_owned()
    };
    if needs_space(preceding, &result) {
        format!(" {result}")
    } else {
        result
    }
}

const SENTENCE_ENDERS: &[&str] = &[".", "!", "?", "…"];
const OPENERS: &[&str] = &["(", "[", "{", "\"", "'", "“", "‘", "«", "/", "@", "#", "$"];
const ATTACHES_LEFT: &[&str] = &[
    ",", ".", ";", ":", "!", "?", ")", "]", "}", "…", "'", "’", "%",
];
const CLOSERS: &[&str] = &["\"", "'", "”", "’", ")", "]"];
const CONTINUERS: &[&str] = &[",", ";", ":", "-", "–", "—"];

/// Whether `character` is one of `set`, by Swift's `Character` equality.
fn is_one_of(character: &str, set: &[&str]) -> bool {
    set.iter().any(|it| swift::equivalent(character, it))
}

fn ends_sentence(text: &str) -> bool {
    let mut trimmed = text;
    // A sentence can end inside a closing quote or bracket: `He left.")`
    while swift::last(trimmed).is_some_and(|last| is_one_of(last, CLOSERS)) {
        trimmed = swift::drop_last(trimmed);
    }
    swift::last(trimmed).is_some_and(|last| is_one_of(last, SENTENCE_ENDERS))
}

fn continues_sentence(text: &str) -> bool {
    swift::last(text).is_some_and(|last| {
        swift::is_letter(last) || swift::is_number(last) || is_one_of(last, CONTINUERS)
    })
}

fn needs_space(preceding: &str, text: &str) -> bool {
    let (Some(last), Some(first)) = (swift::last(preceding), swift::first(text)) else {
        return false;
    };
    !swift::is_whitespace(last) && !is_opening(last, preceding) && !is_one_of(first, ATTACHES_LEFT)
}

/// A straight quote opens only at the start or after a space; `said "stop."` ends with a closing
/// one.
fn is_opening(last: &str, preceding: &str) -> bool {
    if !is_one_of(last, OPENERS) {
        return false;
    }
    if !is_one_of(last, &["\"", "'"]) {
        return true;
    }
    swift::last(swift::drop_last(preceding)).is_none_or(swift::is_whitespace)
}

fn capitalizing_first_word(text: &str) -> String {
    let word = first_word(text);
    let all_uncapitalised = swift::characters(word).all(|it| !swift::is_uppercase(it));
    if word.is_empty() || !all_uncapitalised || !swift::first(word).is_some_and(swift::is_lowercase)
    {
        return text.to_owned();
    }
    with_first_mapped(text, swift::uppercased)
}

fn lowercasing_first_word(text: &str) -> String {
    let word = first_word(text);
    let key: String = swift::foundation_replace(&swift::lowercased(word), "’", "'")
        .nfd()
        .collect();
    let rest_uncapitalised =
        swift::characters(swift::drop_first(word)).all(|it| !swift::is_uppercase(it));
    if !rest_uncapitalised || !CONTINUATION_WORDS.contains(&key.as_str()) {
        return text.to_owned();
    }
    with_first_mapped(text, swift::lowercased)
}

/// `text.prefix(1)` mapped, followed by `text.dropFirst()`.
fn with_first_mapped(text: &str, map: fn(&str) -> String) -> String {
    let first = swift::first(text).unwrap_or("");
    map(first) + &text[first.len()..]
}

/// The leading letters and apostrophes.
fn first_word(text: &str) -> &str {
    let length = swift::characters(text)
        .take_while(|it| swift::is_letter(it) || is_one_of(it, &["'", "’"]))
        .map(str::len)
        .sum();
    &text[..length]
}

/// Words that capitalise only because they open a transcript. "I" and its contractions are absent
/// on purpose.
const CONTINUATION_WORDS: &[&str] = &[
    "a", "about", "after", "again", "all", "also", "an", "and", "any", "are", "as", "at",
    "because", "been", "before", "but", "by", "can", "could", "did", "do", "does", "each", "even",
    "every", "for", "from", "had", "has", "have", "he", "her", "here", "his", "how", "if", "in",
    "into", "is", "it", "it's", "its", "just", "let's", "like", "maybe", "more", "most", "my",
    "no", "not", "now", "of", "on", "one", "only", "or", "our", "she", "should", "so", "some",
    "than", "that", "that's", "the", "their", "them", "then", "there", "there's", "these", "they",
    "they're", "this", "those", "though", "to", "too", "until", "was", "we", "we're", "were",
    "what", "when", "where", "which", "while", "who", "why", "will", "with", "would", "yes", "yet",
    "you", "you're", "your",
];
