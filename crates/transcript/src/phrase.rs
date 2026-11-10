//! A vocabulary phrase's whole-word, case-insensitive match, as ICU runs the pattern the Swift
//! builds: `(?<![\p{L}\p{N}_])word\s+word[\s-]?Word(?![\p{L}\p{N}_])` with
//! `.caseInsensitive`, each word an escaped (so literal) run. It is matched here directly rather
//! than through a regex because ICU folds a literal in full, which a regex can only spell out by
//! listing every way to write it, and for a phrase like "sssss" there are exponentially many.

use crate::tables;

/// One literal run, folded as ICU compiles it.
#[derive(Debug)]
enum Literal {
    /// It folds to one scalar: a text scalar matches if its simple folding is that.
    One(char),
    /// It folds to several: the text's full foldings must spell them, without ending inside one.
    Many(Vec<char>),
}

impl Literal {
    fn new(run: &str) -> Self {
        let folded: Vec<char> = run.chars().flat_map(tables::full_fold).collect();
        match folded[..] {
            [single] => Self::One(single),
            _ => Self::Many(folded),
        }
    }

    /// Where a match at `at` ends, if there is one.
    fn match_at(&self, text: &str, at: usize) -> Option<usize> {
        match self {
            Self::One(folded) => {
                let scalar = text[at..].chars().next()?;
                (tables::simple_fold(scalar) == *folded).then(|| at + scalar.len_utf8())
            }
            Self::Many(folded) => {
                let mut wanted = folded.iter();
                let mut end = at;
                for scalar in text[at..].chars() {
                    if wanted.len() == 0 {
                        break;
                    }
                    for unit in tables::full_fold(scalar) {
                        // Running out inside a scalar's folding fails, as ICU's `inExpansion`.
                        if wanted.next() != Some(&unit) {
                            return None;
                        }
                    }
                    end += scalar.len_utf8();
                }
                (wanted.len() == 0).then_some(end)
            }
        }
    }
}

#[derive(Debug)]
enum Step {
    Literal(Literal),
    /// `\s+`.
    Spaces,
    /// `[\s-]?`.
    OptionalSpaceOrHyphen,
}

/// A compiled phrase.
#[derive(Debug)]
pub struct Phrase {
    steps: Vec<Step>,
}

impl Phrase {
    /// `parts`' words, the words of a part separated by whitespace, and the parts by an optional
    /// space or hyphen.
    pub fn new<'a>(parts: impl IntoIterator<Item = Vec<&'a str>>) -> Self {
        let mut steps = Vec::new();
        for (index, words) in parts.into_iter().enumerate() {
            if index > 0 {
                steps.push(Step::OptionalSpaceOrHyphen);
            }
            for (index, word) in words.into_iter().enumerate() {
                if index > 0 {
                    steps.push(Step::Spaces);
                }
                steps.push(Step::Literal(Literal::new(word)));
            }
        }
        Self { steps }
    }

    /// Where a match of `steps` at `at` ends, trying the longest whitespace first as ICU's
    /// backtracking does.
    fn match_steps(steps: &[Step], text: &str, at: usize) -> Option<usize> {
        let Some((step, rest)) = steps.split_first() else {
            return Some(at);
        };
        match step {
            Step::Literal(literal) => {
                let end = literal.match_at(text, at)?;
                Self::match_steps(rest, text, end)
            }
            Step::Spaces => {
                let mut ends: Vec<usize> = text[at..]
                    .char_indices()
                    .take_while(|(_, it)| it.is_whitespace())
                    .map(|(offset, it)| at + offset + it.len_utf8())
                    .collect();
                ends.reverse();
                ends.into_iter()
                    .find_map(|end| Self::match_steps(rest, text, end))
            }
            Step::OptionalSpaceOrHyphen => {
                let taken = text[at..]
                    .chars()
                    .next()
                    .filter(|&it| it.is_whitespace() || it == '-')
                    .and_then(|it| Self::match_steps(rest, text, at + it.len_utf8()));
                taken.or_else(|| Self::match_steps(rest, text, at))
            }
        }
    }

    /// Every match in `text`, left to right and not overlapping, replaced by `replacement`. An
    /// empty phrase matches, empty, between any two characters that aren't word characters.
    pub fn replace_all(&self, text: &str, replacement: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut at = 0;
        loop {
            let before = text[..at].chars().next_back();
            let found = if before.is_some_and(is_word) {
                None
            } else {
                Self::match_steps(&self.steps, text, at)
                    .filter(|&end| !text[end..].chars().next().is_some_and(is_word))
            };
            if let Some(end) = found {
                result.push_str(replacement);
                if end > at {
                    at = end;
                    continue;
                }
            }
            // No match here, or an empty one, after which ICU looks on from the next character.
            let Some(next) = text[at..].chars().next() else {
                break;
            };
            result.push(next);
            at += next.len_utf8();
        }
        result
    }
}

/// `[\p{L}\p{N}_]`, matched case-insensitively.
fn is_word(scalar: char) -> bool {
    scalar == '_'
        || tables::is_icu_letter(scalar)
        || tables::is_icu_number(scalar)
        || tables::word_case_closure().any(|it| it == scalar)
}
