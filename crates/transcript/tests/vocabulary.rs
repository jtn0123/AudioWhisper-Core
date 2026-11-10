//! The Mac app's `CustomVocabularyTests`, ported.

use transcript::vocabulary::{MAX_PROMPT_CHARACTERS, camel_case_parts};
use transcript::{Replacement, Vocabulary};

fn replacement(heard: &str, written: &str) -> Replacement {
    Replacement {
        heard: heard.to_owned(),
        written: written.to_owned(),
    }
}

fn terms(terms: &[&str]) -> Vocabulary {
    Vocabulary {
        terms: terms.iter().map(|it| (*it).to_owned()).collect(),
        replacements: Vec::new(),
    }
}

#[test]
fn parsing_separates_terms_replacements_and_comments() {
    let vocabulary = Vocabulary::parse(
        "# my words\nKubernetes\n  post gress -> Postgres\njay son => JSON\n\nKubernetes\n -> nothing",
    );
    assert_eq!(vocabulary.terms, ["Kubernetes"]);
    assert_eq!(
        vocabulary.replacements,
        [
            replacement("post gress", "Postgres"),
            replacement("jay son", "JSON")
        ]
    );
}

#[test]
fn terms_fix_case_on_whole_words_only() {
    let vocabulary = terms(&["GitHub", "AI"]);
    assert_eq!(
        vocabulary.apply("push to github and ask the ai"),
        "push to GitHub and ask the AI"
    );
    assert_eq!(vocabulary.apply("said aim and plain"), "said aim and plain");
}

#[test]
fn camel_case_terms_match_their_spoken_split() {
    let vocabulary = terms(&["AudioWhisper", "PostgreSQL"]);
    assert_eq!(vocabulary.apply("open audio whisper"), "open AudioWhisper");
    assert_eq!(
        vocabulary.apply("audio-whisper on postgre sql"),
        "AudioWhisper on PostgreSQL"
    );
}

#[test]
fn replacements_are_literal_and_case_insensitive() {
    let vocabulary = Vocabulary {
        terms: Vec::new(),
        replacements: vec![replacement("dollar sign", r"$1 (\n)")],
    };
    assert_eq!(vocabulary.apply("Dollar  sign here"), r"$1 (\n) here");
}

#[test]
fn multi_word_terms_tolerate_spacing() {
    let vocabulary = terms(&["Visual Studio Code"]);
    assert_eq!(
        vocabulary.apply("open visual  studio code now"),
        "open Visual Studio Code now"
    );
}

#[test]
fn prompt_lists_spellings_within_the_budget() {
    let vocabulary = Vocabulary {
        terms: vec!["Kubernetes".to_owned(), "AudioWhisper".to_owned()],
        replacements: vec![
            replacement("post gress", "Postgres"),
            replacement("skip", ""),
        ],
    };
    assert_eq!(
        vocabulary.prompt_text().as_deref(),
        Some("Kubernetes, AudioWhisper, Postgres")
    );
    assert_eq!(Vocabulary::default().prompt_text(), None);

    let many: Vec<String> = (0..200).map(|it| format!("Term{it}")).collect();
    let long = terms(&many.iter().map(String::as_str).collect::<Vec<_>>());
    let prompt = long.prompt_text().unwrap_or_default();
    assert!(prompt.chars().count() <= MAX_PROMPT_CHARACTERS);
    assert!(prompt.starts_with("Term0, Term1"));
}

#[test]
fn camel_case_splitting() {
    assert_eq!(camel_case_parts("iPhone"), ["i", "Phone"]);
    assert_eq!(camel_case_parts("NASA"), ["NASA"]);
    assert_eq!(camel_case_parts("ChatGPT"), ["Chat", "GPT"]);
}
