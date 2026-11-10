//! The deterministic text rules between speech recognition and pasting, shared by AudioWhisper's
//! Mac app and its Android keyboard: spoken punctuation, filler words, custom vocabulary, and
//! fitting a transcript to the text before the cursor.
//!
//! They began as the Mac app's Swift and keep its exact behaviour, Unicode details included:
//! [`swift`] (from `apple-text`, shared with the other ports) reproduces Swift's and Foundation's
//! text semantics, and `icu` the ICU regular expressions behind `NSRegularExpression`.

pub mod cursor;
pub mod formatter;
mod icu;
mod phrase;
pub mod vocabulary;

pub use apple_text::swift;
use apple_text::tables;

pub use vocabulary::{Replacement, Vocabulary};

/// What [`format`] does, captured when a dictation starts so a retry formats exactly as the
/// original attempt would have.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub vocabulary: Vocabulary,
    pub remove_fillers: bool,
    pub spoken_punctuation: bool,
}

impl Options {
    /// Whether [`format`] returns the text as it is.
    pub fn is_identity(&self) -> bool {
        self.vocabulary.is_empty() && !self.remove_fillers && !self.spoken_punctuation
    }
}

/// The transcript with spoken punctuation turned into marks, fillers removed and the vocabulary
/// applied, as `options` ask.
pub fn format(text: &str, options: &Options) -> String {
    if options.is_identity() {
        return text.to_owned();
    }
    let mut result = text.to_owned();
    if options.spoken_punctuation {
        result = formatter::apply_spoken_punctuation(&result);
    }
    if options.remove_fillers {
        result = formatter::remove_fillers(&result);
    }
    options.vocabulary.apply(&result)
}
