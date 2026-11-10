//! Swift's and Foundation's text rules, as the Mac app's formatter relied on them.
//!
//! A Swift `Character` is an extended grapheme cluster, and most of its properties are its first
//! scalar's. `==` on strings and characters is canonical equivalence, so "é" written as one
//! scalar equals "e" and a combining acute. Case mapping is per scalar, with the full mappings and
//! no context (no final sigma). Foundation's `CharacterSet.whitespaces` is general category Zs, the
//! tab and U+200B; `.newlines` the line and paragraph separators and U+000A to U+000D and U+0085.
//! Its default string search won't end a match before a scalar that combines with the match.

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

use crate::tables;

/// The text's Swift `Character`s.
pub fn characters(text: &str) -> impl DoubleEndedIterator<Item = &str> {
    text.graphemes(true)
}

/// `text.first`.
pub fn first(text: &str) -> Option<&str> {
    characters(text).next()
}

/// `text.last`.
pub fn last(text: &str) -> Option<&str> {
    characters(text).next_back()
}

/// `text.dropLast()`.
pub fn drop_last(text: &str) -> &str {
    last(text).map_or(text, |last| &text[..text.len() - last.len()])
}

/// `text.dropFirst()`.
pub fn drop_first(text: &str) -> &str {
    first(text).map_or(text, |first| &text[first.len()..])
}

/// Swift's `==` on strings and on characters: canonical equivalence.
pub fn equivalent(a: &str, b: &str) -> bool {
    a.nfd().eq(b.nfd())
}

fn first_scalar(character: &str) -> char {
    character.chars().next().unwrap_or('\0')
}

fn is_single_scalar(character: &str) -> bool {
    character.chars().nth(1).is_none()
}

/// `Character.isWhitespace`: its first scalar is White_Space.
pub fn is_whitespace(character: &str) -> bool {
    first_scalar(character).is_whitespace()
}

/// `Character.isNewline`.
pub fn is_newline(character: &str) -> bool {
    matches!(
        first_scalar(character),
        '\n'..='\r' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}

/// `Character.isLetter`: its first scalar is Alphabetic.
pub fn is_letter(character: &str) -> bool {
    tables::is_alphabetic(first_scalar(character))
}

/// `Character.isNumber`: its first scalar has a numeric type.
pub fn is_number(character: &str) -> bool {
    tables::has_numeric_type(first_scalar(character))
}

/// `Unicode.Scalar.Properties.isCased`: Uppercase, Lowercase or title case.
fn is_cased_scalar(scalar: char) -> bool {
    scalar.is_uppercase() || scalar.is_lowercase() || tables::is_titlecase(scalar)
}

/// `Character.isUppercase`.
pub fn is_uppercase(character: &str) -> bool {
    if is_single_scalar(character) && first_scalar(character).is_uppercase() {
        return true;
    }
    equivalent(character, &uppercased(character)) && is_cased(character)
}

/// `Character.isLowercase`.
pub fn is_lowercase(character: &str) -> bool {
    if is_single_scalar(character) && first_scalar(character).is_lowercase() {
        return true;
    }
    equivalent(character, &lowercased(character)) && is_cased(character)
}

/// `Character.isCased`.
fn is_cased(character: &str) -> bool {
    if is_single_scalar(character) && is_cased_scalar(first_scalar(character)) {
        return true;
    }
    !equivalent(character, &uppercased(character)) || !equivalent(character, &lowercased(character))
}

/// `String.uppercased()`: each scalar's full upper-case mapping.
pub fn uppercased(text: &str) -> String {
    text.chars().flat_map(char::to_uppercase).collect()
}

/// `String.lowercased()`: each scalar's full lower-case mapping, without context.
pub fn lowercased(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).collect()
}

/// Whether `scalar` is in `CharacterSet.whitespaces`: general category Zs, the tab, and (though
/// Apple's documentation doesn't say so) the zero width space.
pub fn is_foundation_whitespace(scalar: char) -> bool {
    tables::is_foundation_whitespace(scalar)
}

/// Whether `scalar` is in `CharacterSet.newlines`.
pub fn is_foundation_newline(scalar: char) -> bool {
    tables::is_foundation_newline(scalar)
}

/// `text.trimmingCharacters(in: .whitespaces)`.
pub fn trim_whitespaces(text: &str) -> &str {
    text.trim_matches(is_foundation_whitespace)
}

/// `haystack.range(of: needle)`, Foundation's default (non-literal) search, for a needle that has
/// no canonical equivalents: its first occurrence that a following scalar doesn't combine with.
pub fn foundation_find(haystack: &str, needle: &str) -> Option<(usize, usize)> {
    haystack
        .match_indices(needle)
        .map(|(start, found)| (start, start + found.len()))
        .find(|&(_, end)| {
            !haystack[end..]
                .chars()
                .next()
                .is_some_and(tables::is_foundation_extend)
        })
}

/// `haystack.replacingOccurrences(of: needle, with: replacement)`, with [`foundation_find`]'s
/// matching.
pub fn foundation_replace(haystack: &str, needle: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(haystack.len());
    let mut rest = haystack;
    while let Some((start, end)) = foundation_find(rest, needle) {
        result.push_str(&rest[..start]);
        result.push_str(replacement);
        rest = &rest[end..];
    }
    result.push_str(rest);
    result
}
