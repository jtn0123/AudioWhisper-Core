//! The Mac app's `CursorContextFormatterTests`, ported.

use transcript::cursor::fit;

#[test]
fn start_of_field_capitalises_without_a_space() {
    assert_eq!(fit("hello there", Some("")), "Hello there");
    assert_eq!(fit("iPhone works", Some("")), "iPhone works");
}

#[test]
fn after_a_sentence_end_adds_a_space_and_keeps_the_capital() {
    assert_eq!(fit("We should go", Some("It is late.")), " We should go");
    assert_eq!(
        fit("we should go", Some("He said \"stop.\"")),
        " We should go"
    );
    assert_eq!(fit("We should go", Some("It is late. ")), "We should go");
}

#[test]
fn mid_sentence_lowercases_only_function_words() {
    assert_eq!(
        fit("We should go", Some("I told her that")),
        " we should go"
    );
    assert_eq!(fit("It’s fine", Some("and")), " it’s fine");
    assert_eq!(fit("Paris is nice", Some("I think")), " Paris is nice");
    assert_eq!(fit("I agree", Some("and")), " I agree");
    assert_eq!(fit("NASA launched", Some("then")), " NASA launched");
}

#[test]
fn new_line_and_openers_need_no_space() {
    assert_eq!(fit("and then", Some("Done\n")), "And then");
    assert_eq!(fit("The note", Some("see (")), "The note");
    // An opening quote takes no space.
    assert_eq!(fit("hello", Some("say \"")), "hello");
}

#[test]
fn punctuation_attaches_to_the_previous_word() {
    assert_eq!(fit(", and more", Some("apples")), ", and more");
}

#[test]
fn unknown_context_or_leading_space_is_left_alone() {
    assert_eq!(fit("We go", None), "We go");
    assert_eq!(fit(" We go", Some("word")), " We go");
    assert_eq!(fit("", Some("word")), "");
}
