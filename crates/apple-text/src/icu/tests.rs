//! Each expectation is what `NSRegularExpression` returns on macOS for the same pattern and text.

use super::*;

fn found(pattern: &str, text: &str) -> Vec<String> {
    Regex::new(pattern)
        .unwrap_or_else(|| panic!("{pattern} doesn't compile"))
        .matches(text)
}

#[test]
fn case_insensitive_literals_fold_in_full() {
    assert_eq!(
        found("(?i)subject", "SUBJECT ſubject Subject"),
        ["SUBJECT", "ſubject", "Subject"]
    );
    assert_eq!(found("(?i)mustnt", "MUﬆNT"), ["MUﬆNT"]);
    assert_eq!(found("(?i)strasse", "Straße"), ["Straße"]);
    // A match can't end inside a scalar's folding: "ß" is "ss", so "s" alone isn't in it.
    assert!(found("(?i)stras", "Straße").is_empty());
    // A single scalar matches by simple folding, so "ß" (which folds to "ss" in full) is not "ss".
    assert_eq!(found("(?i)k", "k K \u{212A}"), ["k", "K", "\u{212A}"]);
    assert_eq!(found("(?i)ss", "ß ẞ SS"), ["ß", "ẞ", "SS"]);
}

#[test]
fn case_insensitive_classes_take_icus_closure() {
    assert_eq!(found("(?i)5[ap]m", "5AM 5pm"), ["5AM", "5pm"]);
    assert_eq!(found(r"(?i)[\p{L}]+", "a\u{345}b"), ["a\u{345}b"]);
    assert_eq!(found(r"[\p{L}]+", "a\u{345}b"), ["a", "b"]);
    // A class the tables have no closure for won't compile case-insensitively.
    assert!(Regex::new("(?i)[xyz]").is_none());
}

#[test]
fn word_boundaries_look_through_combining_and_format_characters() {
    assert_eq!(
        found(r"\bno\b", "no\u{301} no\u{AD}x"),
        Vec::<String>::new()
    );
    assert_eq!(found(r"\bx", "\u{301}x a\u{301}x"), ["x"]);
    assert_eq!(found(r"\bab\b", "ab\u{903}"), Vec::<String>::new());
    assert_eq!(found(r"\b\w+\b", "x\u{200D}y ½"), ["x\u{200D}y"]);
}

#[test]
fn icus_classes() {
    assert_eq!(found(r"\d+", "12 ١٢ ½ Ⅻ ²"), ["12", "١٢"]);
    assert_eq!(found(r"\s+", "a\u{85}\u{200B}b"), ["\u{85}"]);
    assert_eq!(
        found(r"[\p{L}\p{M}]+", "e\u{301}\u{903}5"),
        ["e\u{301}\u{903}"]
    );
    assert_eq!(found(r#"[^\s`\"]+"#, "a`b c"), ["a", "b", "c"]);
    assert_eq!(found(r"[\w.+%-]+", "a.b-c%d+e"), ["a.b-c%d+e"]);
}

#[test]
fn a_line_anchored_search_starts_after_crlf_not_inside_it() {
    assert_eq!(found(r"(?m)^\s*hi", "a\r\nhi"), ["hi"]);
    assert_eq!(found(r"(?m)^\s*hi", "a\n\nhi \u{2029} hi"), ["\nhi", " hi"]);
    assert_eq!(found(r"(?m)^\s*x", "x\nx\r"), ["x", "x"]);
    // Not at the start of the pattern, `^` does match between "\r" and "\n".
    assert_eq!(found(r"(?m)\r^\n", "\r\n"), ["\r\n"]);
}

#[test]
fn alternation_and_quantifiers_backtrack_like_icu() {
    assert_eq!(found(r"(?i)\b(?:no|not)\b", "not no"), ["not", "no"]);
    assert_eq!(
        found(r"(?i)\b[\p{L}]+n't\b", "don't can't n't"),
        ["don't", "can't"]
    );
    assert_eq!(found(r"-{1,2}[a-z]+", "---v --x"), ["--v", "--x"]);
    assert_eq!(
        found(r"https?://\w+", "http://a https://b"),
        ["http://a", "https://b"]
    );
    assert_eq!(found(r"\d+(?:[.,]\d+)*", "1,250.50, 7."), ["1,250.50", "7"]);
    assert_eq!(
        found(r"(?i)best(?: regards)?", "Best Regards best"),
        ["Best Regards", "best"]
    );
}

#[test]
fn lookarounds() {
    let pattern = r"(?i)(?<![\p{L}\p{N}_])hi(?![\p{L}\p{N}_])";
    assert_eq!(found(pattern, "hi xhi hix _hi \u{345}hi Hi"), ["hi", "Hi"]);
    assert!(Regex::new(r"(?<=a+)b").is_none());
}

#[test]
fn escaped_patterns_match_themselves() {
    let text = "[your name] (a.b) $5 ^x|y? {z}+* a\\b/c";
    assert_eq!(
        escaped_pattern(text),
        r"\[your name] \(a\.b\) \$5 \^x\|y\? \{z\}\+\* a\\b\/c"
    );
    assert_eq!(found(&escaped_pattern(text), text), [text]);
}

#[test]
fn what_icu_refuses_or_this_doesnt_support_fails() {
    for pattern in [
        "a}b",
        "(a",
        "a)",
        "[a",
        "*a",
        "a**",
        "a*?",
        r"\q",
        r"\p{Greek}",
        "a$",
        "(?<n>a)",
    ] {
        assert!(Regex::new(pattern).is_none(), "{pattern}");
    }
}

#[test]
fn an_empty_match_moves_the_search_on() {
    let regex = Regex::new("x*").unwrap_or_else(|| panic!("x* doesn't compile"));
    let text: Vec<char> = "axxb".chars().collect();
    assert_eq!(regex.find_iter(&text), [0..0, 1..3, 3..3, 4..4]);
}
