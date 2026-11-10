//! ICU's pattern syntax, the part of it the ports use, parsed the way ICU's compiler reads it:
//! consecutive literal characters form one run, which a quantifier splits before its last
//! character, and case-insensitive runs and classes are folded and closed as ICU folds and closes
//! them.

use crate::tables;

use super::set::{Item, Set};

/// A zero-width test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assertion {
    /// `\b`, as ICU defines it without `UREGEX_UWORD`.
    WordBoundary,
    /// `^` with `anchorsMatchLines`.
    LineStart,
    /// `^` without it.
    TextStart,
}

/// A parsed pattern.
#[derive(Clone, Debug)]
pub enum Node {
    Set(Set),
    /// Case-sensitive literal scalars (`URX_STRING`).
    Literal(Vec<char>),
    /// A case-insensitive literal run, folded in full (`URX_STRING_I`).
    Folded(Vec<char>),
    Concat(Vec<Node>),
    Alternation(Vec<Node>),
    /// Greedy; `max` `None` is unbounded.
    Repeat {
        node: Box<Node>,
        min: usize,
        max: Option<usize>,
    },
    Assertion(Assertion),
    Look {
        behind: bool,
        negate: bool,
        node: Box<Node>,
    },
}

#[derive(Clone, Copy, Debug, Default)]
struct Flags {
    case_insensitive: bool,
    multiline: bool,
}

/// `pattern` parsed, or `None` for what ICU wouldn't compile or this parser doesn't know.
pub fn parse(pattern: &str) -> Option<Node> {
    let mut parser = Parser {
        chars: pattern.chars().collect(),
        at: 0,
        flags: Flags::default(),
    };
    let node = parser.alternation()?;
    (parser.at == parser.chars.len()).then_some(node)
}

struct Parser {
    chars: Vec<char>,
    at: usize,
    flags: Flags,
}

/// Literal characters waiting to be emitted as one run.
#[derive(Default)]
struct Run(Vec<char>);

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn eat(&mut self, expected: char) -> bool {
        let found = self.peek() == Some(expected);
        if found {
            self.at += 1;
        }
        found
    }

    fn alternation(&mut self) -> Option<Node> {
        let mut branches = vec![self.concat()?];
        while self.eat('|') {
            branches.push(self.concat()?);
        }
        Some(match branches.len() {
            1 => branches.pop()?,
            _ => Node::Alternation(branches),
        })
    }

    fn concat(&mut self) -> Option<Node> {
        let mut nodes = Vec::new();
        let mut run = Run::default();
        while let Some(next) = self.peek() {
            if next == '|' || next == ')' {
                break;
            }
            if let Some(literal) = self.literal()? {
                if self.at_quantifier() {
                    self.flush(&mut run, &mut nodes);
                    let single = self.run_node(&[literal]);
                    nodes.push(self.quantified(single)?);
                } else {
                    run.0.push(literal);
                }
                continue;
            }
            self.flush(&mut run, &mut nodes);
            if let Some(atom) = self.atom()? {
                let atom = if self.at_quantifier() {
                    self.quantified(atom)?
                } else {
                    atom
                };
                nodes.push(atom);
            }
        }
        self.flush(&mut run, &mut nodes);
        Some(Node::Concat(nodes))
    }

    /// The next character if it is a literal one (consumed), `Some(None)` if something else comes
    /// next, `None` for a bad escape.
    fn literal(&mut self) -> Option<Option<char>> {
        let next = self.peek()?;
        match next {
            // ICU won't compile a `}` that closes nothing, which `atom` refuses too.
            '(' | '[' | '^' | '$' | '.' | '*' | '+' | '?' | '{' | '}' => Some(None),
            '\\' => {
                let escaped = *self.chars.get(self.at + 1)?;
                let literal = if escaped.is_ascii_alphanumeric() {
                    control_escape(escaped)
                } else {
                    Some(escaped)
                };
                if literal.is_some() {
                    self.at += 2;
                }
                Some(literal)
            }
            _ => {
                self.at += 1;
                Some(Some(next))
            }
        }
    }

    fn flush(&self, run: &mut Run, nodes: &mut Vec<Node>) {
        if !run.0.is_empty() {
            nodes.push(self.run_node(&run.0));
            run.0.clear();
        }
    }

    /// A literal run as ICU's `fixLiterals` emits it.
    fn run_node(&self, run: &[char]) -> Node {
        if !self.flags.case_insensitive {
            return match run {
                [single] => Node::Set(Set::of(vec![Item::Range(*single, *single)])),
                _ => Node::Literal(run.to_vec()),
            };
        }
        let folded: Vec<char> = run.iter().copied().flat_map(tables::full_fold).collect();
        match folded[..] {
            // `URX_ONECHAR_I`: a text scalar matches if its simple folding is this.
            [single] => Node::Set(Set::of(vec![Item::Folds(single)])),
            _ => Node::Folded(folded),
        }
    }

    fn at_quantifier(&self) -> bool {
        matches!(self.peek(), Some('*' | '+' | '?' | '{'))
    }

    fn quantified(&mut self, node: Node) -> Option<Node> {
        let (min, max) = match self.peek()? {
            '*' => (0, None),
            '+' => (1, None),
            '?' => (0, Some(1)),
            _ => return self.interval(node),
        };
        self.at += 1;
        self.greedy(node, min, max)
    }

    /// `{n}`, `{n,}` or `{n,m}`.
    fn interval(&mut self, node: Node) -> Option<Node> {
        self.at += 1;
        let min = self.number()?;
        let max = if self.eat(',') {
            if self.peek() == Some('}') {
                None
            } else {
                Some(self.number()?)
            }
        } else {
            Some(min)
        };
        if !self.eat('}') || max.is_some_and(|max| max < min) {
            return None;
        }
        self.greedy(node, min, max)
    }

    fn number(&mut self) -> Option<usize> {
        let start = self.at;
        while self.peek().is_some_and(|it| it.is_ascii_digit()) {
            self.at += 1;
        }
        let digits: String = self.chars[start..self.at].iter().collect();
        digits.parse().ok()
    }

    /// Only greedy quantifiers are supported; a lazy or possessive one fails the parse.
    fn greedy(&mut self, node: Node, min: usize, max: Option<usize>) -> Option<Node> {
        if matches!(self.peek(), Some('?' | '+')) || self.at_quantifier() {
            return None;
        }
        Some(Node::Repeat {
            node: Box::new(node),
            min,
            max,
        })
    }

    /// The atom at the cursor that isn't a literal character; `Some(None)` for a flag setting.
    fn atom(&mut self) -> Option<Option<Node>> {
        let next = self.peek()?;
        self.at += 1;
        let node = match next {
            '(' => return self.group(),
            '[' => Node::Set(self.class()?),
            '^' => Node::Assertion(if self.flags.multiline {
                Assertion::LineStart
            } else {
                Assertion::TextStart
            }),
            '\\' => self.escape()?,
            _ => return None,
        };
        Some(Some(node))
    }

    /// An escape outside a class: `\b`, `\d`, `\s`, `\w`, their negations, or `\p{…}`.
    fn escape(&mut self) -> Option<Node> {
        let letter = self.peek()?;
        self.at += 1;
        if letter == 'b' {
            return Some(Node::Assertion(Assertion::WordBoundary));
        }
        // ICU closes a property over case when it matches case-insensitively; unsupported here.
        if matches!(letter, 'p' | 'P') && self.flags.case_insensitive {
            return None;
        }
        let item = self.class_escape(letter)?;
        let negated = letter.is_ascii_uppercase();
        Some(Node::Set(Set::new(vec![item], negated, Vec::new())))
    }

    /// The set an escape names (its negation is the caller's): `\d`, `\s`, `\w` or `\p{…}`.
    fn class_escape(&mut self, letter: char) -> Option<Item> {
        match letter.to_ascii_lowercase() {
            'd' => Some(Item::Digit),
            's' => Some(Item::Space),
            'w' => Some(Item::Word),
            'p' => {
                if !self.eat('{') {
                    return None;
                }
                let start = self.at;
                while self.peek()? != '}' {
                    self.at += 1;
                }
                let name: String = self.chars[start..self.at].iter().collect();
                self.at += 1;
                Item::property(&name)
            }
            _ => None,
        }
    }

    fn group(&mut self) -> Option<Option<Node>> {
        let saved = self.flags;
        let mut look = None;
        if self.eat('?') {
            match self.peek()? {
                ':' => self.at += 1,
                '=' | '!' => look = Some((false, self.chars[self.at] == '!')),
                '<' => {
                    self.at += 1;
                    look = Some((true, self.peek()? == '!'));
                    // Named groups aren't supported.
                    if !matches!(self.peek()?, '=' | '!') {
                        return None;
                    }
                }
                _ => {
                    if self.flag_group()? {
                        return Some(None);
                    }
                }
            }
            if look.is_some() {
                self.at += 1;
            }
        }
        let body = self.alternation()?;
        if !self.eat(')') {
            return None;
        }
        self.flags = saved;
        Some(Some(match look {
            Some((behind, negate)) => Node::Look {
                behind,
                negate,
                node: Box::new(body),
            },
            None => body,
        }))
    }

    /// `(?im)` (true: the flags hold to the end of the enclosing group) or `(?im:` (false: a
    /// group with them follows).
    fn flag_group(&mut self) -> Option<bool> {
        loop {
            match self.peek()? {
                'i' => self.flags.case_insensitive = true,
                'm' => self.flags.multiline = true,
                ')' => {
                    self.at += 1;
                    return Some(true);
                }
                ':' => {
                    self.at += 1;
                    return Some(false);
                }
                _ => return None,
            }
            self.at += 1;
        }
    }

    /// A class, after its `[`.
    fn class(&mut self) -> Option<Set> {
        let start = self.at - 1;
        let negated = self.eat('^');
        let mut items = Vec::new();
        let mut first = true;
        loop {
            let next = self.peek()?;
            self.at += 1;
            match next {
                ']' if !first => break,
                '[' => return None,
                '\\' => {
                    let escaped = self.peek()?;
                    self.at += 1;
                    if let Some(control) = control_escape(escaped) {
                        items.push(self.range_from(control)?);
                    } else if escaped.is_ascii_alphanumeric() {
                        let item = self.class_escape(escaped)?;
                        if escaped.is_ascii_uppercase() {
                            return None;
                        }
                        items.push(item);
                    } else {
                        items.push(self.range_from(escaped)?);
                    }
                }
                _ => items.push(self.range_from(next)?),
            }
            first = false;
        }
        let extra = if self.flags.case_insensitive {
            let source: String = self.chars[start..self.at].iter().collect();
            tables::case_closure(&source)?.collect()
        } else {
            Vec::new()
        };
        Some(Set::new(items, negated, extra))
    }

    /// `first`, or the range from it if a `-` and an end follow (a `-` before `]` is literal).
    fn range_from(&mut self, first: char) -> Option<Item> {
        if self.peek() != Some('-') || self.chars.get(self.at + 1) == Some(&']') {
            return Some(Item::Range(first, first));
        }
        self.at += 1;
        let mut last = self.peek()?;
        self.at += 1;
        if last == '\\' {
            last = self.peek().filter(|it| !it.is_ascii_alphanumeric())?;
            self.at += 1;
        }
        (first <= last).then_some(Item::Range(first, last))
    }
}

/// The character `\n`, `\r` or `\t` stands for.
fn control_escape(letter: char) -> Option<char> {
    match letter {
        'n' => Some('\n'),
        'r' => Some('\r'),
        't' => Some('\t'),
        _ => None,
    }
}

/// Whether every match of `node` begins with a multiline `^`, so that ICU's search only tries it
/// where lines start (`matchStartType`'s `START_LINE`).
pub fn starts_with_line_start(node: &Node) -> bool {
    match node {
        Node::Assertion(Assertion::LineStart) => true,
        Node::Concat(nodes) => nodes.first().is_some_and(starts_with_line_start),
        _ => false,
    }
}
