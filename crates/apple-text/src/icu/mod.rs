//! ICU's regular expressions, which `NSRegularExpression` runs, for the patterns the ports use:
//! literals, classes with `\p{L}`, `\p{N}`, `\p{Lu}`, `\p{M}`, `\w`, `\s` and `\d`, groups,
//! alternation, greedy quantifiers, `\b`, `^`, lookahead and fixed-length lookbehind, and the
//! flags `(?i)` and `(?m)`. Where ICU differs from Perl or the regex crate, this is ICU:
//!
//! * A case-insensitive literal run is folded in full, and the text is folded scalar by scalar
//!   to match it, so "ﬆ" is "st" and "ß" is "ss"; a single scalar matches by simple folding.
//! * A case-insensitive class is closed over case with ICU's own closure (`[ap]` takes "A" and
//!   "P", `[\p{L}]` takes U+0345), generated from macOS.
//! * `\w` is Alphabetic, M, Nd, Pc and the zero width (non-)joiners; `\b` compares the scalar at
//!   the position with the last one before it that isn't Grapheme_Extend or a format character,
//!   and is never true just before one.
//! * With `(?m)`, `^` matches after any line terminator, between `\r` and `\n` too, but never at
//!   the end of the text. A search for a pattern that starts with it, though, only tries where a
//!   line starts, and to ICU a line starts after "\r\n", not inside it.
//!
//! The data (ICU's Unicode 17, newer than the regex crate's) comes from [`crate::tables`].
//! Positions are scalar offsets: ICU counts UTF-16 units, but a match never splits a scalar.

mod parse;
mod program;
mod set;

use std::ops::Range;

use program::Program;

/// A compiled pattern.
#[derive(Clone, Debug)]
pub struct Regex {
    program: Program,
    /// Whether the pattern starts with a multiline `^` (ICU's `START_LINE`).
    starts_lines: bool,
}

impl Regex {
    /// `pattern` compiled, or `None` where `NSRegularExpression(pattern:)` would throw or the
    /// pattern goes beyond what is supported.
    pub fn new(pattern: &str) -> Option<Self> {
        let node = parse::parse(pattern)?;
        Some(Self {
            program: Program::compile(&node)?,
            starts_lines: parse::starts_with_line_start(&node),
        })
    }

    /// `matches(in:range:)` over the whole text: every match, left to right and not
    /// overlapping, as scalar offsets. After an empty match the search moves on a scalar.
    pub fn find_iter(&self, text: &[char]) -> Vec<Range<usize>> {
        let mut found = Vec::new();
        let mut from = 0;
        while let Some(range) = self.find_at(text, from) {
            from = if range.is_empty() {
                range.end + 1
            } else {
                range.end
            };
            found.push(range);
        }
        found
    }

    /// The first match that starts at or after `from`.
    pub fn find_at(&self, text: &[char], from: usize) -> Option<Range<usize>> {
        (from..=text.len())
            .filter(|&start| !self.starts_lines || starts_line(text, from, start))
            .find_map(|start| {
                self.program
                    .run(text, start, &|_| true)
                    .map(|end| start..end)
            })
    }

    /// Whether there is any match.
    pub fn is_match(&self, text: &[char]) -> bool {
        self.find_at(text, 0).is_some()
    }

    /// Every match's text.
    pub fn matches(&self, text: &str) -> Vec<String> {
        let scalars: Vec<char> = text.chars().collect();
        self.find_iter(&scalars)
            .into_iter()
            .map(|range| scalars[range].iter().collect())
            .collect()
    }
}

/// Where ICU's search (`RegexMatcher::find`, `START_LINE`) tries a pattern that starts with a
/// multiline `^`, searching from `from`: the start of the text, and after each line terminator at
/// or past `from`, "\r\n" counting as one.
fn starts_line(text: &[char], from: usize, start: usize) -> bool {
    if start == 0 {
        return true;
    }
    let previous = text[start - 1];
    start > from
        && program::is_line_terminator(previous)
        && !(previous == '\r' && text.get(start) == Some(&'\n'))
}

/// `NSRegularExpression.escapedPattern(for:)`: a backslash before each of `$()*+./?[\^{|}`.
pub fn escaped_pattern(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for scalar in text.chars() {
        if "$()*+./?[\\^{|}".contains(scalar) {
            escaped.push('\\');
        }
        escaped.push(scalar);
    }
    escaped
}

#[cfg(test)]
mod tests;
