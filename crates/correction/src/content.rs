//! How much an edit changed, for long texts: the Mac app's `CorrectionContentComparison`.

use std::collections::HashMap;
use std::sync::LazyLock;

use apple_text::icu::Regex;
use apple_text::swift;

/// The most dynamic-programming cells one comparison may fill.
const COMPARISON_BUDGET: usize = 1_000_000;
/// Longer texts (in UTF-16 units) aren't compared at all.
const MAXIMUM_CHARACTERS: usize = 250_000;

static TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[\p{L}\p{M}\p{N}_]+|[^\s]").unwrap_or_else(|| panic!("bad token pattern"))
});

/// The edit distance over the longer length: in `Character`s when the changed span is small
/// enough, else in word and punctuation tokens. `None` when even the tokens are too many to
/// compare, or a text is too long; uncertainty keeps the source.
pub fn ratio(original: &str, corrected: &str) -> Option<f64> {
    let too_long = |text: &str| text.encode_utf16().count() > MAXIMUM_CHARACTERS;
    if too_long(original) || too_long(corrected) {
        return None;
    }
    let mut symbols = Symbols::default();
    let source = symbols.characters(original);
    let edited = symbols.characters(corrected);
    if let Some(distance) = distance(&source, &edited) {
        return Some(over_longer(distance, source.len(), edited.len()));
    }
    let source = symbols.tokens(original);
    let edited = symbols.tokens(corrected);
    let distance = distance(&source, &edited)?;
    Some(over_longer(distance, source.len(), edited.len()))
}

fn over_longer(distance: usize, source: usize, edited: usize) -> f64 {
    distance as f64 / source.max(edited).max(1) as f64
}

/// Strings as numbers, equal exactly when Swift's `==` calls the strings equal, so the
/// comparisons below are Swift's on `Character`s and `String`s.
#[derive(Default)]
pub struct Symbols(HashMap<String, u32>);

impl Symbols {
    fn symbol(&mut self, text: &str) -> u32 {
        let next = u32::try_from(self.0.len()).unwrap_or(u32::MAX);
        *self.0.entry(swift::canonical(text)).or_insert(next)
    }

    /// `Array(text)`.
    pub fn characters(&mut self, text: &str) -> Vec<u32> {
        swift::characters(text).map(|it| self.symbol(it)).collect()
    }

    /// Runs of letters, marks, numbers and underscores, and every other non-space scalar, each
    /// lower-cased.
    fn tokens(&mut self, text: &str) -> Vec<u32> {
        TOKEN
            .matches(text)
            .iter()
            .map(|it| self.symbol(&swift::lowercased(it)))
            .collect()
    }
}

/// The Levenshtein distance, after a common prefix and suffix are set aside; `None` if what is
/// left would take more than the budget.
fn distance(source: &[u32], edited: &[u32]) -> Option<usize> {
    let shorter = source.len().min(edited.len());
    let prefix = source
        .iter()
        .zip(edited)
        .take_while(|(a, b)| a == b)
        .count();
    let suffix = source[prefix..]
        .iter()
        .rev()
        .zip(edited[prefix..].iter().rev())
        .take(shorter - prefix)
        .take_while(|(a, b)| a == b)
        .count();
    let left = &source[prefix..source.len() - suffix];
    let right = &edited[prefix..edited.len() - suffix];
    if left.is_empty() || right.is_empty() {
        return Some(left.len().max(right.len()));
    }
    if left.len() > COMPARISON_BUDGET / right.len() {
        return None;
    }
    Some(levenshtein(left, right))
}

/// Two rows of the classic table, `right` along them.
pub fn levenshtein(left: &[u32], right: &[u32]) -> usize {
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = previous.clone();
    for (row, &a) in left.iter().enumerate() {
        current[0] = row + 1;
        for (column, &b) in right.iter().enumerate() {
            let substitution = previous[column] + usize::from(a != b);
            current[column + 1] = (previous[column + 1] + 1)
                .min(current[column] + 1)
                .min(substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}
