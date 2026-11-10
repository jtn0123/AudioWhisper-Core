use super::*;

fn folded(scalar: char) -> String {
    full_fold(scalar).collect()
}

#[test]
fn folding_expands_and_keeps_nul() {
    assert_eq!(folded('ß'), "ss");
    assert_eq!(folded('ẞ'), "ss");
    assert_eq!(folded('\u{FB03}'), "ffi");
    assert_eq!(folded('İ'), "i\u{307}");
    assert_eq!(folded('A'), "a");
    assert_eq!(folded('\0'), "\0");
    assert_eq!(simple_fold('ẞ'), 'ß');
    assert_eq!(simple_fold('\u{212A}'), 'k');
}

#[test]
fn preimages_collect_every_spelling() {
    assert_eq!(simple_preimage('k'), ['K', 'k', '\u{212A}']);
    assert_eq!(simple_preimage('ß'), ['ß', 'ẞ']);
    assert_eq!(full_preimage(&['s', 's']), ['ß', 'ẞ']);
    assert_eq!(full_preimage(&['s', 't']), ['\u{FB05}', '\u{FB06}']);
    // ß folds in full to "ss", so it is not a spelling of itself.
    assert!(full_preimage(&['ß']).is_empty());
}

#[test]
fn ranges_hold_their_ends_and_nothing_past_them() {
    assert!(is_icu_letter('A') && is_icu_letter('Z') && !is_icu_letter('@') && !is_icu_letter('['));
    // A letter new in Unicode 17, which the regex crate doesn't know.
    assert!(is_icu_letter('\u{A7CE}'));
    assert!(is_icu_number('0') && is_icu_number('Ⅻ') && !is_icu_number('/'));
    assert!(is_foundation_whitespace('\u{200B}') && !is_foundation_whitespace('\n'));
    assert!(is_foundation_newline('\u{2029}') && !is_foundation_newline(' '));
    assert!(is_foundation_extend('\u{301}') && !is_foundation_extend('\u{200D}'));
    assert!(has_numeric_type('一') && !has_numeric_type('丁'));
    assert!(is_titlecase('ǅ') && !is_titlecase('Ǆ'));
    assert_eq!(word_case_closure().collect::<Vec<_>>(), ['\u{345}']);
}

#[test]
fn classes_compile_with_icu_data() {
    let class = icu_class("Lu").unwrap_or_default();
    assert!(class.starts_with(r"[\x{41}-\x{5A}"));
    assert!(icu_class("Zs").is_none());
}
