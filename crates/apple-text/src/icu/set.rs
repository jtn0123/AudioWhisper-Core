//! Character classes, as ICU's compiled `UnicodeSet`s match them.

use crate::tables;

/// One member of a class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    /// The scalars from the first to the last, inclusive.
    Range(char, char),
    /// The scalars whose simple folding is this one (a case-insensitive literal scalar).
    Folds(char),
    /// `\p{L}`.
    Letter,
    /// `\p{N}`.
    Number,
    /// `\p{Lu}`.
    UppercaseLetter,
    /// `\p{M}`.
    Mark,
    /// `\w`.
    Word,
    /// `\s`.
    Space,
    /// `\d`.
    Digit,
}

impl Item {
    /// The property `\p{name}` names, of those the ports use.
    pub fn property(name: &str) -> Option<Self> {
        match name {
            "L" => Some(Self::Letter),
            "N" => Some(Self::Number),
            "Lu" => Some(Self::UppercaseLetter),
            "M" => Some(Self::Mark),
            _ => None,
        }
    }

    fn contains(self, scalar: char) -> bool {
        match self {
            Self::Range(first, last) => (first..=last).contains(&scalar),
            Self::Folds(folded) => tables::simple_fold(scalar) == folded,
            Self::Letter => tables::is_icu_letter(scalar),
            Self::Number => tables::is_icu_number(scalar),
            Self::UppercaseLetter => tables::is_icu_uppercase_letter(scalar),
            Self::Mark => tables::is_icu_mark(scalar),
            Self::Word => tables::is_icu_word(scalar),
            Self::Space => tables::is_icu_space(scalar),
            Self::Digit => tables::is_icu_digit(scalar),
        }
    }
}

/// A class: its members, perhaps negated, then what case-insensitive matching adds (ICU closes a
/// class over case after negating it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Set {
    items: Vec<Item>,
    negated: bool,
    closure: Vec<char>,
}

impl Set {
    pub fn new(items: Vec<Item>, negated: bool, closure: Vec<char>) -> Self {
        Self {
            items,
            negated,
            closure,
        }
    }

    /// The plain class of these members.
    pub fn of(items: Vec<Item>) -> Self {
        Self::new(items, false, Vec::new())
    }

    pub fn contains(&self, scalar: char) -> bool {
        self.items.iter().any(|it| it.contains(scalar)) != self.negated
            || self.closure.contains(&scalar)
    }
}
