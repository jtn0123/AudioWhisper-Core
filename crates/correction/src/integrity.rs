//! Conservative checks for details an edit must keep: the Mac app's `CorrectionIntegrity`. They
//! are safeguards, not proof that two sentences mean the same thing; the user keeps the original.

use std::collections::HashMap;
use std::sync::LazyLock;

use apple_text::icu::{Regex, escaped_pattern};
use apple_text::swift;

/// How many times each value occurs, keyed as a Swift `[String: Int]` is: canonically
/// equivalent strings are one key.
type Counts = HashMap<String, usize>;

fn constant(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|| panic!("bad pattern {pattern}"))
}

static NEGATION: LazyLock<Regex> = LazyLock::new(|| {
    constant(concat!(
        r"(?i)\b(?:not|no|never|cannot|unable|neither|nor|dont|doesnt|didnt|isnt|arent|wasnt|werent|",
        r"havent|hasnt|hadnt|wont|wouldnt|shouldnt|couldnt|mustnt|cant)\b|\b[\p{L}]+n't\b"
    ))
});

/// Negation, then the other words whose loss or addition flips the meaning.
static POLARITY: LazyLock<[Regex; 3]> = LazyLock::new(|| {
    [
        r"(?i)\b(?:negative|minus)\b",
        r"(?i)\b(?:positive|plus)\b",
        r"(?i)\b(?:unless|without)\b",
    ]
    .map(constant)
});

static NUMBER: LazyLock<Regex> =
    LazyLock::new(|| constant(r"(?i)(?<![\p{L}\p{N}_])[+-]?\d+(?:[.,:]\d+)*(?:\s*[ap]m|%)?"));

static SPOKEN_NUMBER: LazyLock<Regex> = LazyLock::new(|| {
    constant(concat!(
        r"(?i)\b(?:zero|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|hundred|",
        r"thousand|million)\b"
    ))
});

/// Flags, emails, paths and URLs, and identifiers.
static LITERAL: LazyLock<Regex> = LazyLock::new(|| {
    constant(
        &[
            r"(?<![\p{L}\p{N}_])-{1,2}[a-zA-Z][a-zA-Z0-9_-]*",
            r"[\w.+%-]+@[\w.-]+\.[a-zA-Z]+",
            r#"(?:https?://|~/|/)[^\s`\"'<>]+"#,
            r"\$[a-zA-Z_]\w*|\b[a-z]+[A-Z][\p{L}\p{N}_]*\b|\b[\p{L}_]+_[\p{L}\p{N}_]+\b",
        ]
        .join("|"),
    )
});

static CODE: LazyLock<Regex> = LazyLock::new(|| constant(r"`[^`]+`"));

static NAME: LazyLock<Regex> =
    LazyLock::new(|| constant(r"\b\p{Lu}[\p{L}\p{M}]*(?:['’]\p{Lu}[\p{L}\p{M}]*)?\b"));

static WORD: LazyLock<Regex> = LazyLock::new(|| constant(r"[\p{L}\p{N}_]+(?:['’][\p{L}\p{N}_]+)*"));

/// A greeting, a sign-off, or a placeholder for a name: what formatting must not invent.
static RECIPIENT_FRAMES: LazyLock<[Regex; 3]> = LazyLock::new(|| {
    [
        r"(?im)^\s*(?:hi|hello|hey|dear)\b",
        r"(?im)^\s*(?:best(?: regards)?|kind regards|regards|sincerely|cheers)\b",
        r"(?i)\[(?:your|sender|recipient|name)\b[^\]]*\]",
    ]
    .map(constant)
});

static SUBJECT: LazyLock<Regex> = LazyLock::new(|| constant(r"(?im)^\s*subject\s*:"));

/// Whether `corrected` keeps what `original` must not lose: its polarity, its digits, its literal
/// flags, addresses and code, its names, and no greeting, sign-off or (unless `allow_framing`)
/// subject line it didn't have.
pub fn allows(original: &str, corrected: &str, allow_framing: bool) -> bool {
    if polarity(original) != polarity(corrected) {
        return false;
    }
    let source_numbers = numbers(original);
    let edited_numbers = numbers(corrected);
    // Spoken numbers may become digits, but existing digits may never change.
    if source_numbers != edited_numbers
        && !(keeps_exactly(&source_numbers, &edited_numbers) && contains_spoken_number(original))
    {
        return false;
    }
    if !keeps_exactly(&counts(literals(original)), &counts(literals(corrected))) {
        return false;
    }
    let edited_words = counts(matches(&WORD, corrected, true));
    let keeps_names = counts(names(original))
        .iter()
        .all(|(name, &count)| edited_words.get(name).copied().unwrap_or(0) >= count);
    keeps_names && invents_no_frame(original, corrected, allow_framing)
}

/// Whether every value in `source` occurs exactly as often in `edited`.
fn keeps_exactly(source: &Counts, edited: &Counts) -> bool {
    source
        .iter()
        .all(|(value, count)| edited.get(value) == Some(count))
}

/// Formatting must not invent a recipient or sender that was never dictated: each frame in
/// `corrected` must already be in `original`, as a whole word, in any case.
fn invents_no_frame(original: &str, corrected: &str, allow_framing: bool) -> bool {
    let source: Vec<char> = original.chars().collect();
    let subject = (!allow_framing).then(|| &*SUBJECT);
    RECIPIENT_FRAMES.iter().chain(subject).all(|pattern| {
        matches(pattern, corrected, true).iter().all(|frame| {
            let phrase = escaped_pattern(swift::trim_whitespaces_and_newlines(frame));
            let existing = format!(r"(?i)(?<![\p{{L}}\p{{N}}_]){phrase}(?![\p{{L}}\p{{N}}_])");
            Regex::new(&existing).is_some_and(|it| it.is_match(&source))
        })
    })
}

fn polarity(text: &str) -> [usize; 4] {
    let normalized: Vec<char> = swift::foundation_replace(text, "’", "'").chars().collect();
    let count = |regex: &Regex| regex.find_iter(&normalized).len();
    [
        count(&NEGATION),
        count(&POLARITY[0]),
        count(&POLARITY[1]),
        count(&POLARITY[2]),
    ]
}

fn numbers(text: &str) -> Counts {
    let values = matches(&NUMBER, text, true).into_iter().map(|value| {
        let value = swift::foundation_replace(&value, ",", "");
        swift::characters(&value)
            .filter(|it| !swift::is_whitespace(it))
            .collect::<String>()
    });
    counts(values)
}

fn contains_spoken_number(text: &str) -> bool {
    SPOKEN_NUMBER.is_match(&text.chars().collect::<Vec<_>>())
}

fn literals(text: &str) -> Vec<String> {
    let trimmed = matches(&LITERAL, text, false)
        .into_iter()
        .map(|it| it.trim_matches(|c| ".,;:!?)]}".contains(c)).to_owned());
    trimmed.chain(matches(&CODE, text, false)).collect()
}

/// Capitalised words that aren't the first of a sentence, lower-cased.
fn names(text: &str) -> Vec<String> {
    const IGNORED: [&str; 10] = [
        "i", "hi", "hello", "hey", "dear", "um", "uh", "so", "okay", "ok",
    ];
    let scalars: Vec<char> = text.chars().collect();
    NAME.find_iter(&scalars)
        .into_iter()
        .filter_map(|range| {
            // A capitalized first word may just be a normal verb or a typo to fix.
            if starts_sentence(&scalars[..range.start]) {
                return None;
            }
            let name: String = scalars[range].iter().collect();
            let name = swift::foundation_replace(&swift::lowercased(&name), "’", "'");
            let ignored = IGNORED.iter().any(|it| swift::equivalent(it, &name));
            (!ignored).then_some(name)
        })
        .collect()
}

/// Whether the text before a word, trimmed of `.whitespaces`, is empty or ends with a sentence
/// end or a line feed: the Swift's `prefix.trimmingCharacters(in: .whitespaces).last`, then
/// `".!?\n".contains`.
///
/// Only the last two scalars are needed: the last `Character` is one of those four exactly when
/// the last scalar is and doesn't join the one before it, and only a pair decides that (a
/// Prepend scalar before ".", "!" or "?", or "\r" before "\n").
fn starts_sentence(prefix: &[char]) -> bool {
    let kept = |it: &char| !swift::is_foundation_whitespace(*it);
    let (Some(first), Some(last)) = (prefix.iter().position(kept), prefix.iter().rposition(kept))
    else {
        return true;
    };
    let tail: String = prefix[first.max(last.saturating_sub(1))..=last]
        .iter()
        .collect();
    swift::last(&tail).is_none_or(|previous| {
        [".", "!", "?", "\n"]
            .iter()
            .any(|end| swift::equivalent(end, previous))
    })
}

/// The text of every match, with curly apostrophes made straight and, if `lowercase`, lower-cased.
fn matches(regex: &Regex, text: &str, lowercase: bool) -> Vec<String> {
    regex
        .matches(text)
        .into_iter()
        .map(|value| {
            let value = swift::foundation_replace(&value, "’", "'");
            if lowercase {
                swift::lowercased(&value)
            } else {
                value
            }
        })
        .collect()
}

fn counts(values: impl IntoIterator<Item = String>) -> Counts {
    let mut counts = Counts::new();
    for value in values {
        *counts.entry(swift::canonical(&value)).or_default() += 1;
    }
    counts
}
