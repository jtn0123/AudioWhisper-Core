//! Where Swift, Foundation and ICU differ from the obvious Rust, each answer taken from the Mac
//! app's Swift on macOS (the differential test in the Mac app checks them all the time; these
//! keep the crate honest on its own).

use transcript::cursor::fit;
use transcript::formatter::{apply_spoken_punctuation, remove_fillers, tidy};
use transcript::{Replacement, Vocabulary};

fn terms(terms: &[&str]) -> Vocabulary {
    Vocabulary {
        terms: terms.iter().map(|it| (*it).to_owned()).collect(),
        replacements: Vec::new(),
    }
}

/// ICU folds a case-insensitive literal in full, so a ligature or ß spells its letters.
#[test]
fn literals_fold_in_full() {
    assert_eq!(
        apply_spoken_punctuation("It is done full \u{FB06}op Next"),
        "It is done. Next"
    );
    assert_eq!(
        apply_spoken_punctuation("Hi \u{17F}emicolon there"),
        "Hi; there"
    );
    assert_eq!(
        terms(&["STRASSE"]).apply("die Straße hier"),
        "die STRASSE hier"
    );
    assert_eq!(terms(&["Fuß"]).apply("FUSS und fuss"), "Fuß und Fuß");
    assert_eq!(terms(&["ß"]).apply("ss s ẞ"), "ß s ß");
    assert_eq!(terms(&["\u{212A}"]).apply("k K"), "\u{212A} \u{212A}");
}

/// A long run of letters that fold together is matched without trying every spelling.
#[test]
fn a_long_folding_run_is_quick() {
    let term = "s".repeat(64);
    let text = format!("{} {}", "ß".repeat(32), "ss".repeat(31));
    assert_eq!(
        terms(&[&term]).apply(&text),
        format!("{term} {}", "ss".repeat(31))
    );
}

#[test]
fn nul_and_empty_terms_change_nothing() {
    assert_eq!(terms(&["\u{0}"]).apply("a b"), "a b");
    assert_eq!(terms(&[""]).apply("a, b"), "a, b");
}

/// With anchorsMatchLines, ICU's `^` and `$` know every line terminator.
#[test]
fn lines_end_at_every_terminator() {
    assert_eq!(
        apply_spoken_punctuation("It ended period\r\nNext"),
        "It ended.\r\nNext"
    );
    assert_eq!(remove_fillers("Done.\u{2028}Um, next"), "Done.\u{2028}Next");
    assert_eq!(remove_fillers("Done.\u{0B}uh next"), "Done.\u{0B}Next");
}

/// Matched case-insensitively, `[\p{L}]` takes in U+0345, which folds to a Greek iota.
#[test]
fn a_combining_iota_joins_a_filler_to_its_word() {
    assert_eq!(remove_fillers("a\u{345}uh b"), "a\u{345}uh b");
}

/// `CharacterSet.whitespaces` includes the zero width space.
#[test]
fn trimming_takes_zero_width_spaces() {
    assert_eq!(tidy("\u{200B} , x  .\u{200B}"), ", x.");
    assert_eq!(fit("so", Some("x.\u{200B}")), " So");
}

/// Foundation's replacing doesn't remove a sentence mark a combining character sits on.
#[test]
fn a_mark_with_a_combining_character_stays() {
    assert_eq!(
        apply_spoken_punctuation("Why question mark\u{301}so"),
        "Why?\u{E000}\u{301}so"
    );
}

/// Swift's `range(of:)` finds an arrow only as whole characters.
#[test]
fn an_arrow_inside_a_character_is_not_an_arrow() {
    let vocabulary = Vocabulary::parse("\u{600}-> x\nab->\u{301}c\nq -> \u{200D}r");
    assert_eq!(vocabulary.terms, ["\u{600}-> x", "ab->\u{301}c"]);
    assert_eq!(
        vocabulary.replacements,
        [Replacement {
            heard: "q".to_owned(),
            written: "\u{200D}r".to_owned()
        }]
    );
}
