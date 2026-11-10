//! `NSRegularExpression.stringByReplacingMatches`, which is ICU's regular expressions, over
//! fancy-regex. The patterns are written in fancy-regex's syntax with ICU's meaning spelled out
//! where the two differ:
//!
//! * `\p{L}`, `\p{N}`, `\p{Lu}` and `\p{Ll}` are ICU's (a newer Unicode than the regex crate's),
//!   substituted by [`compile`].
//! * A case-insensitive literal is [`literal`], which folds as ICU does: in full, so "STRASSE"
//!   matches "Straße".
//! * With `anchorsMatchLines`, `^` and `$` know every line terminator ([`LINE_START`],
//!   [`LINE_END`]).
//!
//! Both libraries' `\s` is White_Space.

use std::borrow::Cow;

use fancy_regex::{Captures, Regex, RegexBuilder};

use crate::tables;

/// ICU's `^` with `anchorsMatchLines` (`URX_CARET_M`): the start of the text, or just after a
/// line terminator that isn't the text's last character, including between `\r` and `\n`.
pub const LINE_START: &str = r"(?:\A|(?<=[\n\x0B\x0C\r\x{85}\x{2028}\x{2029}])(?!\z))";

/// ICU's `$` with `anchorsMatchLines` (`URX_DOLLAR_M`): the end of the text, or just before a
/// line terminator, but not between the two characters of `\r\n`.
pub const LINE_END: &str = r"(?:\z|(?=[\x0B\x0C\r\x{85}\x{2028}\x{2029}])|(?<!\r)(?=\n))";

/// A compiled pattern, with ICU's property classes; `None` if it doesn't compile.
pub fn compile(pattern: &str) -> Option<Regex> {
    let mut pattern = pattern.to_owned();
    for property in ["Lu", "Ll", "L", "N"] {
        let class = tables::icu_class(property)?;
        pattern = pattern.replace(&format!(r"\p{{{property}}}"), &class);
    }
    RegexBuilder::new(&pattern)
        // ICU has no limit; these patterns backtrack at most a few characters a step.
        .backtrack_limit(usize::MAX)
        .build()
        .ok()
}

/// A pattern that is part of this crate, so known to compile.
pub fn constant(pattern: &str) -> Regex {
    compile(pattern).unwrap_or_else(|| panic!("bad pattern {pattern}"))
}

/// What case-insensitive matching adds to a class of letters and numbers, for writing inside the
/// class.
pub fn word_case_closure() -> String {
    tables::word_case_closure().map(escaped).collect()
}

fn escaped(scalar: char) -> String {
    format!(r"\x{{{:X}}}", u32::from(scalar))
}

fn class(scalars: &[char]) -> String {
    format!(
        "[{}]",
        scalars.iter().copied().map(escaped).collect::<String>()
    )
}

/// A run of literal characters matched case-insensitively, as ICU compiles it: folded in full, it
/// is either one scalar, which a text scalar matches if its simple folding is that
/// (`URX_ONECHAR_I`), or several, which the text's full foldings must spell out, without ending
/// inside one (`URX_STRING_I`).
pub fn literal(run: &str) -> String {
    let folded: Vec<char> = run.chars().flat_map(tables::full_fold).collect();
    match folded[..] {
        [single] => class(&tables::simple_preimage(single)),
        _ => spelled(&folded),
    }
}

/// The text whose full foldings are `folded`: each next scalar folds to one, two or three of its
/// scalars.
fn spelled(folded: &[char]) -> String {
    if folded.is_empty() {
        return String::new();
    }
    let alternatives: Vec<String> = (1..=folded.len().min(3))
        .filter_map(|length| {
            let scalars = tables::full_preimage(&folded[..length]);
            (!scalars.is_empty()).then(|| class(&scalars) + &spelled(&folded[length..]))
        })
        .collect();
    match &alternatives[..] {
        [only] => only.clone(),
        _ => format!("(?:{})", alternatives.join("|")),
    }
}

/// Every match of `regex` in `text`, left to right and not overlapping, replaced by what
/// `replacement` makes of it.
pub fn replace_with<'t>(
    regex: &Regex,
    text: &'t str,
    replacement: impl Fn(&Captures<'_>) -> String,
) -> Cow<'t, str> {
    let mut result = String::new();
    let mut copied = 0;
    let mut matched = false;
    for captures in regex.captures_iter(text) {
        // Only a backtracking limit fails a match, and there is none.
        let Some((whole, captures)) = captures
            .ok()
            .and_then(|it| it.get(0).map(|whole| (whole, it)))
        else {
            break;
        };
        matched = true;
        result.push_str(&text[copied..whole.start()]);
        result.push_str(&replacement(&captures));
        copied = whole.end();
    }
    if !matched {
        return Cow::Borrowed(text);
    }
    result.push_str(&text[copied..]);
    Cow::Owned(result)
}

/// Every match replaced by `literal`.
pub fn replace_literal<'t>(regex: &Regex, text: &'t str, literal: &str) -> Cow<'t, str> {
    replace_with(regex, text, |_| literal.to_owned())
}

/// Every match replaced by its group 1 followed by `suffix` (the template `$1` + `suffix`).
pub fn replace_keeping_group<'t>(regex: &Regex, text: &'t str, suffix: &str) -> Cow<'t, str> {
    replace_with(regex, text, |captures| {
        let group = captures.get(1).map_or("", |it| it.as_str());
        format!("{group}{suffix}")
    })
}
