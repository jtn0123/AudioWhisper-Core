//! Spoken punctuation and filler words: the Mac app's `TranscriptFormatter`. Every rule is
//! conservative: it rewrites only text that cannot plausibly be meant literally, because a wrong
//! rewrite is pasted straight into someone's document.

use std::borrow::Cow;
use std::sync::LazyLock;

use fancy_regex::Regex;

use crate::icu::{self, LINE_END, LINE_START};
use crate::swift;

/// Marks where a new sentence begins after a rewrite; see [`capitalize_marked`].
const SENTENCE_MARK: &str = "\u{E000}";

struct Command {
    words: &'static [&'static str],
    symbol: &'static str,
    /// "period" is an ordinary noun ("a period of time"), so it only acts as a command where a
    /// sentence could end.
    needs_sentence_end: bool,
}

const fn command(words: &'static [&'static str], symbol: &'static str) -> Command {
    Command {
        words,
        symbol,
        needs_sentence_end: false,
    }
}

const COMMANDS: [Command; 8] = [
    command(&["new paragraph"], "\n\n"),
    command(&["new line", "newline"], "\n"),
    command(&["question mark"], "?"),
    command(&["exclamation mark", "exclamation point"], "!"),
    Command {
        words: &["full stop", "period"],
        symbol: ".",
        needs_sentence_end: true,
    },
    command(&["semicolon", "semi colon"], ";"),
    command(&["colon"], ":"),
    command(&["comma"], ","),
];

/// Each command's pattern, and what replaces a match.
static COMMAND_RULES: LazyLock<Vec<(Regex, String)>> =
    LazyLock::new(|| COMMANDS.iter().map(command_rule).collect());

fn command_rule(command: &Command) -> (Regex, String) {
    // `(?i:full\s+stop|period)`, with ICU's folding.
    let words = command
        .words
        .iter()
        .map(|it| {
            it.split(' ')
                .map(icu::literal)
                .collect::<Vec<_>>()
                .join(r"\s+")
        })
        .collect::<Vec<_>>()
        .join("|");
    let is_break = command.symbol.starts_with('\n');
    let mut pattern =
        format!(r"[ \t]*[,.;:!?]*[ \t]*(?<![\p{{L}}\p{{N}}_])(?:{words})(?![\p{{L}}\p{{N}}_])");
    pattern += if is_break {
        r"[,.;:!?]*[ \t]*"
    } else {
        r"[,.;:!?]*"
    };
    if command.needs_sentence_end {
        pattern += &format!(r"(?=[ \t]*{LINE_END}|[ \t]+\p{{Lu}}|[ \t]*\n)");
    }
    let ends_sentence = is_break || ".?!".contains(command.symbol);
    let mark = if ends_sentence { SENTENCE_MARK } else { "" };
    (icu::constant(&pattern), format!("{}{mark}", command.symbol))
}

/// "Hi comma how are you question mark" → "Hi, how are you?". Whisper often punctuates around the
/// spoken word too ("Hi, comma, how"), so adjacent marks are absorbed into the command.
pub fn apply_spoken_punctuation(text: &str) -> String {
    let mut result = Cow::Borrowed(text);
    for (regex, template) in COMMAND_RULES.iter() {
        result = Cow::Owned(icu::replace_literal(regex, &result, template).into_owned());
    }
    swift::trim_whitespaces(&capitalize_marked(&result)).to_owned()
}

/// The filler patterns, which the Swift matches case-insensitively throughout: a filler not
/// joined to a word on either side, and the fillers themselves.
struct Fillers {
    start: String,
    end: String,
    always: String,
    any: String,
    um: String,
}

static FILLERS: LazyLock<Fillers> = LazyLock::new(|| {
    let closure = icu::word_case_closure();
    let [u, h, m, um, er] = ["u", "h", "m", "um", "er"].map(icu::literal);
    // `uh+m*|umm+|erm+`: ICU gives a quantifier only the last character of a literal run.
    let always = format!("{u}{h}+{m}*|{um}{m}+|{er}{m}+");
    Fillers {
        start: format!(r"(?<![\p{{L}}\p{{N}}_'’{closure}-])"),
        end: format!(r"(?![\p{{L}}\p{{N}}_'’{closure}-])"),
        any: format!("(?:{always}|{um})"),
        always: format!("(?:{always})"),
        um,
    }
});

/// A filler that opens a sentence; it hands its capital to the next word.
static OPENING_FILLER: LazyLock<Regex> = LazyLock::new(|| {
    let Fillers {
        start,
        end,
        always,
        um,
        ..
    } = &*FILLERS;
    icu::constant(&format!(
        r"({LINE_START}|[.!?][ \t]+){start}(?:{always}|{um}(?=[,.…])){end}[,.…]*[ \t]*"
    ))
});

/// Commas that only bracketed the filler go with it: "I was, uh, thinking".
static BRACKETED_FILLER: LazyLock<Regex> = LazyLock::new(|| {
    let Fillers {
        start, end, any, ..
    } = &*FILLERS;
    icu::constant(&format!(r"[ \t]*,[ \t]*{start}{any}{end}[ \t]*,"))
});

static FILLER_BEFORE_END: LazyLock<Regex> = LazyLock::new(|| {
    let Fillers {
        start, end, any, ..
    } = &*FILLERS;
    icu::constant(&format!(r"[ \t]*,[ \t]*{start}{any}{end}(?=[.!?…])"))
});

/// A comma the speaker dictated before an unbracketed filler stays.
static BARE_FILLER: LazyLock<Regex> = LazyLock::new(|| {
    let Fillers {
        start, end, always, ..
    } = &*FILLERS;
    icu::constant(&format!(r"[ \t]*{start}{always}{end}[,…]?"))
});

/// "uh" and "uhm" are never words, so they go wherever they appear. "um" is a real word in German
/// ("um 5 Uhr") and Portuguese ("um carro"), so it is removed only when punctuated as an
/// interjection: "Um, so" / "so, um, I".
pub fn remove_fillers(text: &str) -> String {
    let result = icu::replace_keeping_group(&OPENING_FILLER, text, SENTENCE_MARK);
    let result = icu::replace_literal(&BRACKETED_FILLER, &result, "").into_owned();
    let result = icu::replace_literal(&FILLER_BEFORE_END, &result, "").into_owned();
    let result = icu::replace_literal(&BARE_FILLER, &result, "").into_owned();
    tidy(&capitalize_marked(&result))
}

static TIDY_RULES: LazyLock<[(Regex, &'static str); 3]> = LazyLock::new(|| {
    [
        (icu::constant(r"[ \t]{2,}"), " "),
        (icu::constant(r",{2,}"), ","),
        (
            icu::constant(&format!(r"{LINE_START}[ \t]*[,;:][ \t]*")),
            "",
        ),
    ]
});

static SPACE_BEFORE_PUNCTUATION: LazyLock<Regex> =
    LazyLock::new(|| icu::constant(r"[ \t]+([,.;:!?])"));

/// Collapses the gaps a removal leaves: doubled spaces, a space before punctuation, a doubled
/// comma, a comma left at the very start.
pub fn tidy(text: &str) -> String {
    let [spaces, commas, leading] = &*TIDY_RULES;
    let result = icu::replace_literal(&spaces.0, text, spaces.1);
    let result = icu::replace_keeping_group(&SPACE_BEFORE_PUNCTUATION, &result, "").into_owned();
    let result = icu::replace_literal(&commas.0, &result, commas.1).into_owned();
    let result = icu::replace_literal(&leading.0, &result, leading.1);
    swift::trim_whitespaces(&result).to_owned()
}

static MARKED_WORD: LazyLock<Regex> = LazyLock::new(|| {
    icu::constant(&format!(
        r"{SENTENCE_MARK}([ \t\n]*)(\p{{Ll}}[\p{{Ll}}'’]*)(?![\p{{L}}])"
    ))
});

/// Uppercases the all-lowercase word after each sentence mark, then drops the marks. Only
/// positions a rule rewrote are touched, so existing text such as "e.g. this" keeps its case; a
/// word with an internal capital ("iPhone") is left alone.
pub fn capitalize_marked(text: &str) -> String {
    let result = icu::replace_with(&MARKED_WORD, text, |captures| {
        let whole = captures.get(0).map_or("", |it| it.as_str());
        let Some(word) = captures.get(2) else {
            return whole.to_owned();
        };
        let word = word.as_str();
        let first = swift::first(word).unwrap_or("");
        let start = whole.len() - word.len();
        format!(
            "{}{}{}",
            &whole[..start],
            swift::uppercased(first),
            &word[first.len()..]
        )
    });
    swift::foundation_replace(&result, SENTENCE_MARK, "")
}
