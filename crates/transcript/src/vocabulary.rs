//! The user's words, spellings to enforce and phrases to rewrite: the Mac app's
//! `CustomVocabulary`.
//!
//! Edited as plain text, one entry per line:
//!
//! ```text
//! Kubernetes
//! AudioWhisper
//! post gress -> Postgres
//! ```
//!
//! A bare line is a *term*: any case-insensitive whole-word match is rewritten to that exact
//! spelling, and a CamelCase term also matches its spoken split ("audio whisper" →
//! "AudioWhisper"). A `heard -> written` (or `=>`) line is a *replacement*. Blank lines and lines
//! starting with `#` are ignored.

use std::collections::HashSet;

use unicode_normalization::UnicodeNormalization;

use crate::phrase::Phrase;
use crate::swift;

/// A `heard -> written` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replacement {
    pub heard: String,
    pub written: String,
}

/// The parsed vocabulary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Vocabulary {
    pub terms: Vec<String>,
    pub replacements: Vec<Replacement>,
}

/// Whisper's prompt window is 224 tokens and is shared with the decoder context, so the prompt
/// stays well under it.
pub const MAX_PROMPT_CHARACTERS: usize = 400;

impl Vocabulary {
    /// Whether there is nothing to apply.
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty() && self.replacements.is_empty()
    }

    /// The vocabulary written one entry per line.
    pub fn parse(text: &str) -> Self {
        let mut vocabulary = Self::default();
        for raw_line in text.split(swift::is_foundation_newline) {
            let line = swift::trim_whitespaces(raw_line);
            if line.is_empty() || has_prefix(line, "#") {
                continue;
            }
            if let Some((start, end)) = find(line, "->").or_else(|| find(line, "=>")) {
                let heard = swift::trim_whitespaces(&line[..start]);
                let written = swift::trim_whitespaces(&line[end..]);
                if heard.is_empty() {
                    continue;
                }
                vocabulary.replacements.push(Replacement {
                    heard: heard.to_owned(),
                    written: written.to_owned(),
                });
            } else if !vocabulary
                .terms
                .iter()
                .any(|term| swift::equivalent(term, line))
            {
                vocabulary.terms.push(line.to_owned());
            }
        }
        vocabulary
    }

    /// Spellings the recogniser should lean toward, as a natural comma list. Only the leading
    /// entries that fit [`MAX_PROMPT_CHARACTERS`] are used.
    pub fn prompt_text(&self) -> Option<String> {
        let written = self.replacements.iter().map(|it| it.written.as_str());
        let spellings = self
            .terms
            .iter()
            .map(String::as_str)
            .chain(written.filter(|it| !it.is_empty()));
        let mut prompt = String::new();
        let mut seen = HashSet::new();
        for spelling in spellings {
            if !seen.insert(spelling.nfd().collect::<String>()) {
                continue;
            }
            let next = if prompt.is_empty() {
                spelling.to_owned()
            } else {
                format!("{prompt}, {spelling}")
            };
            if swift::characters(&next).count() > MAX_PROMPT_CHARACTERS {
                break;
            }
            prompt = next;
        }
        (!prompt.is_empty()).then_some(prompt)
    }

    /// Replacements first, so a rewrite can produce a term's spelling; terms then normalise case
    /// and spoken CamelCase splits.
    pub fn apply(&self, text: &str) -> String {
        let mut result = text.to_owned();
        for replacement in &self.replacements {
            result = replace_whole_phrase(&replacement.heard, &replacement.written, &result, false);
        }
        for term in &self.terms {
            result = replace_whole_phrase(term, term, &result, true);
        }
        result
    }
}

/// `line.hasPrefix(prefix)`: whole characters, compared by canonical equivalence.
fn has_prefix(line: &str, prefix: &str) -> bool {
    let mut characters = swift::characters(line);
    swift::characters(prefix).all(|wanted| {
        characters
            .next()
            .is_some_and(|character| swift::equivalent(character, wanted))
    })
}

/// `line.range(of: needle)`, which compares whole characters: the first occurrence of `needle`
/// that starts and ends between characters.
fn find(line: &str, needle: &str) -> Option<(usize, usize)> {
    let boundaries: Vec<usize> = std::iter::once(0)
        .chain(swift::characters(line).scan(0, |end, character| {
            *end += character.len();
            Some(*end)
        }))
        .collect();
    boundaries
        .iter()
        .filter(|&&start| line[start..].starts_with(needle))
        .map(|&start| (start, start + needle.len()))
        .find(|(_, end)| boundaries.contains(end))
}

/// Every whole-word, case-insensitive match of `phrase` (its words separated by any whitespace,
/// and with `matching_split_camel_case` its CamelCase parts by an optional space or hyphen)
/// replaced by `replacement`.
pub fn replace_whole_phrase(
    phrase: &str,
    replacement: &str,
    text: &str,
    matching_split_camel_case: bool,
) -> String {
    let parts = if matching_split_camel_case {
        camel_case_parts(phrase)
    } else {
        vec![phrase.to_owned()]
    };
    let words = parts.iter().map(|part| split_on_whitespace(part).collect());
    Phrase::new(words).replace_all(text, replacement)
}

/// `part.split(whereSeparator: \.isWhitespace)`: the runs of non-whitespace characters.
fn split_on_whitespace(part: &str) -> impl Iterator<Item = &str> {
    let mut words = Vec::new();
    let mut start = None;
    let mut offset = 0;
    for character in swift::characters(part) {
        if swift::is_whitespace(character) {
            if let Some(from) = start.take() {
                words.push(&part[from..offset]);
            }
        } else if start.is_none() {
            start = Some(offset);
        }
        offset += character.len();
    }
    if let Some(from) = start {
        words.push(&part[from..]);
    }
    words.into_iter()
}

/// "AudioWhisper" → ["Audio", "Whisper"]; "PostgreSQL" → ["Postgre", "SQL"]. A lowercase →
/// uppercase step starts a new part; acronym runs stay whole.
pub fn camel_case_parts(term: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut previous: Option<&str> = None;
    for character in swift::characters(term) {
        if previous.is_some_and(swift::is_lowercase)
            && swift::is_uppercase(character)
            && !current.is_empty()
        {
            parts.push(std::mem::take(&mut current));
        }
        current.push_str(character);
        previous = Some(character);
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}
