//! The Mac app's `TranscriptFormatterTests`, ported.

use transcript::formatter::{apply_spoken_punctuation, remove_fillers};
use transcript::{Options, Vocabulary, format};

#[test]
fn default_options_leave_text_untouched() {
    let text = "Um, so  e.g. this , stays exactly. period";
    assert_eq!(format(text, &Options::default()), text);
}

#[test]
fn sentence_opening_filler_hands_its_capital_on() {
    assert_eq!(
        remove_fillers("Um, so I think it works."),
        "So I think it works."
    );
    assert_eq!(
        remove_fillers("It works. Uh, maybe not."),
        "It works. Maybe not."
    );
    assert_eq!(remove_fillers("Uh I agree."), "I agree.");
}

#[test]
fn mid_sentence_fillers_are_removed_with_their_commas() {
    assert_eq!(
        remove_fillers("I was, uh, thinking about it."),
        "I was thinking about it."
    );
    assert_eq!(remove_fillers("I was uhm thinking."), "I was thinking.");
    assert_eq!(remove_fillers("So, um, I left."), "So I left.");
    assert_eq!(remove_fillers("Deploy it, uh now."), "Deploy it, now.");
    assert_eq!(
        remove_fillers("I said uh, never mind."),
        "I said never mind."
    );
    assert_eq!(remove_fillers("I agree, um."), "I agree.");
}

#[test]
fn um_is_kept_where_it_is_a_word() {
    assert_eq!(
        remove_fillers("Wir treffen uns um 5 Uhr."),
        "Wir treffen uns um 5 Uhr."
    );
    assert_eq!(remove_fillers("Um carro azul."), "Um carro azul.");
    assert_eq!(
        remove_fillers("Ich gehe, um zu essen."),
        "Ich gehe, um zu essen."
    );
}

#[test]
fn meaningful_lookalikes_survive() {
    assert_eq!(remove_fillers("Uh-oh, that broke."), "Uh-oh, that broke.");
    assert_eq!(
        remove_fillers("The umbrella is here."),
        "The umbrella is here."
    );
    assert_eq!(remove_fillers("Use e.g. this one."), "Use e.g. this one.");
}

#[test]
fn fillers_on_a_line_after_a_break_are_removed() {
    assert_eq!(remove_fillers("First.\nUm, second."), "First.\nSecond.");
}

#[test]
fn spoken_marks_become_punctuation() {
    assert_eq!(
        apply_spoken_punctuation("Hi comma how are you question mark"),
        "Hi, how are you?"
    );
    assert_eq!(apply_spoken_punctuation("Wait exclamation point"), "Wait!");
    assert_eq!(
        apply_spoken_punctuation("Note colon buy milk"),
        "Note: buy milk"
    );
}

#[test]
fn whisper_punctuation_around_the_command_is_absorbed() {
    assert_eq!(
        apply_spoken_punctuation("Hello, comma, how are you?"),
        "Hello, how are you?"
    );
    assert_eq!(apply_spoken_punctuation("I'm done. Period."), "I'm done.");
}

#[test]
fn line_breaks_and_capitalisation() {
    assert_eq!(
        apply_spoken_punctuation("first line new line second line"),
        "first line\nSecond line"
    );
    assert_eq!(
        apply_spoken_punctuation("Dear Sam, new paragraph. thanks for writing"),
        "Dear Sam\n\nThanks for writing"
    );
}

#[test]
fn period_as_a_noun_is_kept() {
    let text = "Over a period of three weeks the period ended";
    assert_eq!(apply_spoken_punctuation(text), text);
    assert_eq!(
        apply_spoken_punctuation("It is done period Next one"),
        "It is done. Next one"
    );
}

#[test]
fn options_compose_in_order() {
    let options = Options {
        vocabulary: Vocabulary::parse("Kubernetes"),
        remove_fillers: true,
        spoken_punctuation: true,
    };
    assert_eq!(
        format("Um, deploy kubernetes comma uh now period", &options),
        "Deploy Kubernetes, now."
    );
}
