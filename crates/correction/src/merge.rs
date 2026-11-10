//! Whether to take a model's correction: the pure parts of the Mac app's
//! `SemanticCorrectionService` (`safeMerge`, `normalizedEditDistance`) and
//! `DictationWriting.accepted`.

use apple_text::swift;

use crate::content::{self, Symbols};
use crate::integrity;

/// One conservative bound on how much grammar cleanup may change, in every app.
pub const MAX_CHANGE_RATIO: f64 = 0.6;

/// Above this many `Character`s, the change is measured by the bounded comparison
/// ([`content::ratio`]): the full table runs in well under a second at about 4,000, but a 30+
/// minute transcript would not.
pub const SAFE_MERGE_LENGTH_CAP: usize = 4000;

/// What the model was asked to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Conservative cleanup, which must stay close to the source.
    Cleanup,
    /// Rewriting style or format, which may change much more but keeps the facts.
    Rewrite,
}

/// The correction, trimmed, if it keeps what [`integrity::allows`] protects and changes at most
/// `max_change_ratio` of the text; otherwise the original.
pub fn safe_merge(original: &str, corrected: &str, max_change_ratio: f64) -> String {
    if corrected.is_empty() || !integrity::allows(original, corrected, false) {
        return original.to_owned();
    }
    let accepted = swift::trim_whitespaces_and_newlines(corrected).to_owned();
    if swift::count(original).max(swift::count(corrected)) > SAFE_MERGE_LENGTH_CAP {
        return match content::ratio(original, corrected) {
            Some(ratio) if ratio <= max_change_ratio => accepted,
            _ => original.to_owned(),
        };
    }
    if normalized_edit_distance(original, corrected) > max_change_ratio {
        return original.to_owned();
    }
    accepted
}

/// The edit distance in `Character`s over the longer length: 0 for equal texts, 1 if just one
/// is empty.
pub fn normalized_edit_distance(lhs: &str, rhs: &str) -> f64 {
    if swift::equivalent(lhs, rhs) {
        return 0.0;
    }
    let mut symbols = Symbols::default();
    let lhs = symbols.characters(lhs);
    let rhs = symbols.characters(rhs);
    if lhs.is_empty() || rhs.is_empty() {
        return 1.0;
    }
    // The shorter text runs along the rows, so they stay short.
    let (shorter, longer) = if lhs.len() > rhs.len() {
        (&rhs, &lhs)
    } else {
        (&lhs, &rhs)
    };
    let distance = content::levenshtein(longer, shorter);
    distance as f64 / lhs.len().max(rhs.len()) as f64
}

/// The model's output if it may replace the dictation, else the original. Cleanup goes through
/// [`safe_merge`]; a rewrite may change any amount, but must not be empty or balloon, and must
/// keep what [`integrity::allows`] protects (a subject line is allowed).
pub fn accepted(original: &str, output: &str, operation: Operation) -> String {
    if operation == Operation::Cleanup {
        return safe_merge(original, output, MAX_CHANGE_RATIO);
    }
    let result = swift::trim_whitespaces_and_newlines(output);
    let limit = 256.max(swift::count(original).saturating_mul(4));
    if result.is_empty()
        || swift::count(result) > limit
        || !integrity::allows(original, result, true)
    {
        return original.to_owned();
    }
    result.to_owned()
}
