//! A parsed pattern compiled to a backtracking program, run as ICU's matcher runs one: depth
//! first, the first alternative and the longest repetition tried first, lookarounds atomic. The
//! backtracking stack is a vector, so a long text can't overflow the thread's stack.

use crate::tables;

use super::parse::{Assertion, Node};
use super::set::Set;

#[derive(Clone, Debug)]
enum Instruction {
    Set(Set),
    Literal(Vec<char>),
    Folded(Vec<char>),
    /// A greedy run of one class, backtracked a scalar at a time (ICU's `URX_LOOP_SR_I`).
    Loop {
        set: Set,
        min: usize,
        max: Option<usize>,
    },
    /// Go on at the first; on failure, at the second.
    Split(usize, usize),
    Jump(usize),
    Assertion(Assertion),
    Look {
        behind: bool,
        negate: bool,
        program: Program,
        /// A lookbehind's fixed length in scalars.
        length: usize,
    },
    Match,
}

/// A compiled pattern.
#[derive(Clone, Debug)]
pub struct Program(Vec<Instruction>);

/// Where to resume after a failure.
enum Retry {
    At(usize, usize),
    /// A loop at `position` that may give a scalar back, down to `floor`.
    Shorter {
        next: usize,
        floor: usize,
        position: usize,
    },
}

impl Program {
    /// `None` for a lookbehind without a fixed length, which ICU would bound but isn't needed.
    pub fn compile(node: &Node) -> Option<Self> {
        let mut instructions = Vec::new();
        emit(node, &mut instructions)?;
        instructions.push(Instruction::Match);
        Some(Self(instructions))
    }

    /// The end of the first match that starts at `start` and that `accept` takes.
    pub fn run(
        &self,
        text: &[char],
        start: usize,
        accept: &dyn Fn(usize) -> bool,
    ) -> Option<usize> {
        let mut retries: Vec<Retry> = Vec::new();
        let (mut pc, mut at) = (0, start);
        loop {
            if matches!(self.0[pc], Instruction::Match) {
                if accept(at) {
                    return Some(at);
                }
            } else if self.step(text, &mut pc, &mut at, &mut retries) {
                continue;
            }
            match retries.pop()? {
                Retry::At(next, position) => (pc, at) = (next, position),
                Retry::Shorter {
                    next,
                    floor,
                    position,
                } => {
                    let shorter = position - 1;
                    if shorter > floor {
                        retries.push(Retry::Shorter {
                            next,
                            floor,
                            position: shorter,
                        });
                    }
                    (pc, at) = (next, shorter);
                }
            }
        }
    }

    /// Runs the instruction at `pc`, which isn't `Match`: true if it succeeded (and moved on),
    /// false to backtrack.
    fn step(
        &self,
        text: &[char],
        pc: &mut usize,
        at: &mut usize,
        retries: &mut Vec<Retry>,
    ) -> bool {
        let advanced = match &self.0[*pc] {
            Instruction::Set(set) => text
                .get(*at)
                .is_some_and(|&it| set.contains(it))
                .then_some(1),
            Instruction::Literal(scalars) => {
                text[*at..].starts_with(scalars).then_some(scalars.len())
            }
            Instruction::Folded(folded) => matched_folded(text, *at, folded).map(|end| end - *at),
            Instruction::Loop { set, min, max } => {
                let limit = max.map_or(text.len() - *at, |max| max.min(text.len() - *at));
                let count = text[*at..*at + limit]
                    .iter()
                    .take_while(|&&it| set.contains(it))
                    .count();
                if count < *min {
                    return false;
                }
                if count > *min {
                    retries.push(Retry::Shorter {
                        next: *pc + 1,
                        floor: *at + min,
                        position: *at + count,
                    });
                }
                Some(count)
            }
            Instruction::Split(first, second) => {
                retries.push(Retry::At(*second, *at));
                *pc = *first;
                return true;
            }
            Instruction::Jump(target) => {
                *pc = *target;
                return true;
            }
            Instruction::Assertion(assertion) => holds(*assertion, text, *at).then_some(0),
            Instruction::Look {
                behind,
                negate,
                program,
                length,
            } => {
                let found = if *behind {
                    *at >= *length && program.run(text, *at - length, &|end| end == *at).is_some()
                } else {
                    program.run(text, *at, &|_| true).is_some()
                };
                (found != *negate).then_some(0)
            }
            Instruction::Match => None,
        };
        let Some(advanced) = advanced else {
            return false;
        };
        *at += advanced;
        *pc += 1;
        true
    }
}

/// ICU's `URX_STRING_I`: the text from `at`, folded in full scalar by scalar, spells `folded`,
/// and doesn't end inside a scalar's folding. The end of the match.
fn matched_folded(text: &[char], at: usize, folded: &[char]) -> Option<usize> {
    let mut rest = folded;
    let mut end = at;
    while !rest.is_empty() {
        let scalar = *text.get(end)?;
        for piece in tables::full_fold(scalar) {
            let (&first, others) = rest.split_first()?;
            if first != piece {
                return None;
            }
            rest = others;
        }
        end += 1;
    }
    Some(end)
}

pub fn is_line_terminator(scalar: char) -> bool {
    matches!(scalar, '\n'..='\r' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

fn holds(assertion: Assertion, text: &[char], at: usize) -> bool {
    match assertion {
        Assertion::TextStart => at == 0,
        // `URX_CARET_M`: never at the very end, even after a line terminator.
        Assertion::LineStart => at == 0 || (at < text.len() && is_line_terminator(text[at - 1])),
        Assertion::WordBoundary => is_word_boundary(text, at),
    }
}

/// ICU's `isChunkWordBoundary`: whether the scalar at `at` and the last one before it that
/// isn't Grapheme_Extend or a format character differ in being word characters. There is no
/// boundary before a Grapheme_Extend or format character.
fn is_word_boundary(text: &[char], at: usize) -> bool {
    let current = match text.get(at) {
        Some(&scalar) if tables::is_boundary_transparent(scalar) => return false,
        Some(&scalar) => tables::is_icu_word(scalar),
        None => false,
    };
    let previous = text[..at]
        .iter()
        .rev()
        .find(|&&it| !tables::is_boundary_transparent(it))
        .is_some_and(|&it| tables::is_icu_word(it));
    current != previous
}

fn emit(node: &Node, out: &mut Vec<Instruction>) -> Option<()> {
    match node {
        Node::Set(set) => out.push(Instruction::Set(set.clone())),
        Node::Literal(scalars) => out.push(Instruction::Literal(scalars.clone())),
        Node::Folded(folded) => out.push(Instruction::Folded(folded.clone())),
        Node::Concat(nodes) => {
            for node in nodes {
                emit(node, out)?;
            }
        }
        Node::Alternation(branches) => emit_alternation(branches, out)?,
        Node::Repeat { node, min, max } => emit_repeat(node, *min, *max, out)?,
        Node::Assertion(assertion) => out.push(Instruction::Assertion(*assertion)),
        Node::Look {
            behind,
            negate,
            node,
        } => {
            let length = if *behind { fixed_length(node)? } else { 0 };
            out.push(Instruction::Look {
                behind: *behind,
                negate: *negate,
                program: Program::compile(node)?,
                length,
            });
        }
    }
    Some(())
}

fn emit_alternation(branches: &[Node], out: &mut Vec<Instruction>) -> Option<()> {
    let mut jumps = Vec::new();
    for (index, branch) in branches.iter().enumerate() {
        let last = index + 1 == branches.len();
        let split = out.len();
        if !last {
            out.push(Instruction::Split(split + 1, 0));
        }
        emit(branch, out)?;
        if !last {
            jumps.push(out.len());
            out.push(Instruction::Jump(0));
            let next = out.len();
            out[split] = Instruction::Split(split + 1, next);
        }
    }
    let end = out.len();
    for jump in jumps {
        out[jump] = Instruction::Jump(end);
    }
    Some(())
}

fn emit_repeat(
    node: &Node,
    min: usize,
    max: Option<usize>,
    out: &mut Vec<Instruction>,
) -> Option<()> {
    if let Node::Set(set) = node {
        out.push(Instruction::Loop {
            set: set.clone(),
            min,
            max,
        });
        return Some(());
    }
    for _ in 0..min {
        emit(node, out)?;
    }
    match max {
        None => {
            let split = out.len();
            out.push(Instruction::Split(split + 1, 0));
            emit(node, out)?;
            out.push(Instruction::Jump(split));
            let end = out.len();
            out[split] = Instruction::Split(split + 1, end);
        }
        Some(max) => {
            // Each optional copy is tried only after the one before matched.
            let mut splits = Vec::new();
            for _ in min..max {
                splits.push(out.len());
                out.push(Instruction::Split(0, 0));
                emit(node, out)?;
            }
            let end = out.len();
            for split in splits {
                out[split] = Instruction::Split(split + 1, end);
            }
        }
    }
    Some(())
}

/// The scalars every match of `node` spans, if that is fixed.
fn fixed_length(node: &Node) -> Option<usize> {
    match node {
        Node::Set(_) => Some(1),
        Node::Literal(scalars) => Some(scalars.len()),
        Node::Folded(_) => None,
        Node::Concat(nodes) => nodes.iter().map(fixed_length).sum(),
        Node::Alternation(branches) => {
            let lengths: Option<Vec<usize>> = branches.iter().map(fixed_length).collect();
            let lengths = lengths?;
            let first = *lengths.first()?;
            lengths.iter().all(|&it| it == first).then_some(first)
        }
        Node::Repeat { node, min, max } => {
            (Some(*min) == *max).then_some(())?;
            Some(fixed_length(node)? * min)
        }
        Node::Assertion(_) | Node::Look { .. } => Some(0),
    }
}
