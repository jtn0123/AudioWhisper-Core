//! The Unicode data the Mac app's Swift ran on, generated from macOS by
//! `tools/unicode-tables.sh`: ICU's (behind `NSRegularExpression`) general categories and case
//! folding, and the Swift runtime's and Foundation's scalar sets. Rust's std and the regex crate
//! can be on a different Unicode version, so nothing that has to match the Swift is left to them,
//! except the case mappings and the Uppercase, Lowercase and White_Space properties, where Rust
//! 1.98 (the pinned toolchain) and the Swift runtime agree on every scalar.

mod folding;
mod icu;
mod swift;

use std::cmp::Ordering;

/// Whether `scalar` is in a flat `[first, last, ...]` range list.
fn contains(ranges: &[u32], scalar: char) -> bool {
    let value = u32::from(scalar);
    let (pairs, _) = ranges.as_chunks::<2>();
    pairs
        .binary_search_by(|&[first, last]| {
            if last < value {
                Ordering::Less
            } else if first > value {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}

/// A flat range list as the inside of a regex character class.
fn class_body(ranges: &[u32]) -> String {
    let (pairs, _) = ranges.as_chunks::<2>();
    pairs
        .iter()
        .map(|&[first, last]| format!(r"\x{{{first:X}}}-\x{{{last:X}}}"))
        .collect()
}

/// ICU's meaning of the property classes the formatter's patterns use, as character classes for
/// the regex crate: `\p{L}`, `\p{N}`, `\p{Lu}` and `\p{Ll}`.
pub fn icu_class(property: &str) -> Option<String> {
    let ranges = match property {
        "L" => icu::LETTER,
        "N" => icu::NUMBER,
        "Lu" => icu::UPPERCASE_LETTER,
        "Ll" => icu::LOWERCASE_LETTER,
        _ => return None,
    };
    Some(format!("[{}]", class_body(ranges)))
}

/// What case-insensitive matching adds to a class of letters and numbers.
pub fn word_case_closure() -> impl Iterator<Item = char> {
    icu::WORD_CASE_CLOSURE
        .iter()
        .filter_map(|&it| char::from_u32(it))
}

/// ICU: general category L.
pub fn is_icu_letter(scalar: char) -> bool {
    contains(icu::LETTER, scalar)
}

/// ICU: general category N.
pub fn is_icu_number(scalar: char) -> bool {
    contains(icu::NUMBER, scalar)
}

/// General category Lt.
pub fn is_titlecase(scalar: char) -> bool {
    contains(icu::TITLECASE_LETTER, scalar)
}

/// ICU's simple case folding (`u_foldCase`).
pub fn simple_fold(scalar: char) -> char {
    let (pairs, _) = folding::SIMPLE.as_chunks::<2>();
    let value = u32::from(scalar);
    pairs
        .binary_search_by_key(&value, |&[from, _]| from)
        .ok()
        .and_then(|index| char::from_u32(pairs[index][1]))
        .unwrap_or(scalar)
}

/// ICU's full case folding (`u_strFoldCase`) of one scalar: one to three scalars.
pub fn full_fold(scalar: char) -> impl Iterator<Item = char> {
    let (rows, _) = folding::EXPANSIONS.as_chunks::<4>();
    let value = u32::from(scalar);
    let expansion = rows
        .binary_search_by_key(&value, |&[from, ..]| from)
        .ok()
        .map(|index| rows[index]);
    let folded: [u32; 3] = match expansion {
        Some([_, a, b, c]) => [a, b, c],
        None => [u32::from(simple_fold(scalar)), 0, 0],
    };
    // The padding is 0, but so is the folding of U+0000, which is always one scalar.
    let length = 1 + folded[1..].iter().take_while(|&&it| it != 0).count();
    folded.into_iter().take(length).filter_map(char::from_u32)
}

/// Every scalar whose simple folding is `folded`, `folded` itself included.
pub fn simple_preimage(folded: char) -> Vec<char> {
    let (pairs, _) = folding::SIMPLE.as_chunks::<2>();
    let others = pairs
        .iter()
        .filter(|&&[_, to]| to == u32::from(folded))
        .filter_map(|&[from, _]| char::from_u32(from));
    let mut result: Vec<char> = (simple_fold(folded) == folded)
        .then_some(folded)
        .into_iter()
        .chain(others)
        .collect();
    result.sort_unstable();
    result
}

/// Every scalar whose full folding is exactly `folded`.
pub fn full_preimage(folded: &[char]) -> Vec<char> {
    let mut result: Vec<char> = match folded {
        [single] => simple_preimage(*single),
        _ => Vec::new(),
    };
    let (rows, _) = folding::EXPANSIONS.as_chunks::<4>();
    result.retain(|&it| full_fold(it).eq(folded.iter().copied()));
    let expanded = rows
        .iter()
        .filter_map(|&[from, ..]| char::from_u32(from))
        .filter(|&from| full_fold(from).eq(folded.iter().copied()));
    result.extend(expanded);
    result.sort_unstable();
    result
}

/// `Unicode.Scalar.Properties.numericType != nil`.
pub fn has_numeric_type(scalar: char) -> bool {
    contains(swift::NUMERIC, scalar)
}

/// `Unicode.Scalar.Properties.isAlphabetic`.
pub fn is_alphabetic(scalar: char) -> bool {
    contains(swift::ALPHABETIC, scalar)
}

/// `CharacterSet.whitespaces`.
pub fn is_foundation_whitespace(scalar: char) -> bool {
    contains(swift::WHITESPACES, scalar)
}

/// `CharacterSet.newlines`.
pub fn is_foundation_newline(scalar: char) -> bool {
    contains(swift::NEWLINES, scalar)
}

/// Whether Foundation's non-literal search refuses to end a match just before `scalar`, because
/// it combines with what precedes it.
pub fn is_foundation_extend(scalar: char) -> bool {
    contains(swift::EXTEND, scalar)
}

#[cfg(test)]
mod tests;
