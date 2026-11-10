//! The C interface as Swift uses it: bytes and lengths in, owned strings out.

use awcore::*;

fn call(text: &str, function: unsafe extern "C" fn(*const u8, usize) -> AwString) -> String {
    // SAFETY: the pointer and length are the string's.
    take(unsafe { function(text.as_ptr(), text.len()) }).unwrap_or_default()
}

/// The string's text, freed; `None` for no string.
fn take(string: AwString) -> Option<String> {
    if string.ptr.is_null() {
        return None;
    }
    // SAFETY: the library returned it, with this length.
    let bytes = unsafe { std::slice::from_raw_parts(string.ptr, string.len) }.to_vec();
    // SAFETY: returned by the library and not freed before.
    unsafe { aw_string_free(string) };
    String::from_utf8(bytes).ok()
}

#[test]
fn stages_run_through_the_interface() {
    assert_eq!(call("Hi comma there", aw_spoken_punctuation), "Hi, there");
    assert_eq!(call("Uh, so", aw_remove_fillers), "So");
    assert_eq!(call("a  ,b", aw_tidy), "a,b");
    // A NUL inside the text survives the trip.
    assert_eq!(call("a\0b comma c", aw_spoken_punctuation), "a\0b, c");
}

#[test]
fn a_vocabulary_parses_reads_back_and_applies() {
    let source = "Kubernetes\npost gress -> Postgres";
    // SAFETY: each handle is live until freed at the end, and every pointer and length match.
    unsafe {
        let vocabulary = aw_vocabulary_parse(source.as_ptr(), source.len());
        assert_eq!(aw_vocabulary_term_count(vocabulary), 1);
        assert_eq!(
            take(aw_vocabulary_term(vocabulary, 0)).as_deref(),
            Some("Kubernetes")
        );
        assert_eq!(take(aw_vocabulary_term(vocabulary, 1)), None);
        assert_eq!(aw_vocabulary_replacement_count(vocabulary), 1);
        let heard = take(aw_vocabulary_replacement(vocabulary, 0, false));
        let written = take(aw_vocabulary_replacement(vocabulary, 0, true));
        assert_eq!(
            (heard.as_deref(), written.as_deref()),
            (Some("post gress"), Some("Postgres"))
        );
        let prompt = take(aw_vocabulary_prompt_text(vocabulary));
        assert_eq!(prompt.as_deref(), Some("Kubernetes, Postgres"));
        let text = "deploy kubernetes on post gress comma ok";
        let applied = take(aw_vocabulary_apply(vocabulary, text.as_ptr(), text.len()));
        assert_eq!(
            applied.as_deref(),
            Some("deploy Kubernetes on Postgres comma ok")
        );
        let formatted = take(aw_format(
            text.as_ptr(),
            text.len(),
            vocabulary,
            false,
            true,
        ));
        assert_eq!(
            formatted.as_deref(),
            Some("deploy Kubernetes on Postgres, ok")
        );
        aw_vocabulary_free(vocabulary);
    }
}

#[test]
fn a_vocabulary_builds_up_and_an_empty_one_prompts_nothing() {
    // SAFETY: the handle is live until freed, and every pointer and length match.
    unsafe {
        let vocabulary = aw_vocabulary_new();
        assert_eq!(take(aw_vocabulary_prompt_text(vocabulary)), None);
        let (term, heard, written) = ("GitHub", "jay son", "JSON");
        aw_vocabulary_add_term(vocabulary, term.as_ptr(), term.len());
        aw_vocabulary_add_replacement(
            vocabulary,
            heard.as_ptr(),
            heard.len(),
            written.as_ptr(),
            written.len(),
        );
        let text = "github and jay son";
        let applied = take(aw_vocabulary_apply(vocabulary, text.as_ptr(), text.len()));
        assert_eq!(applied.as_deref(), Some("GitHub and JSON"));
        aw_vocabulary_free(vocabulary);
        aw_vocabulary_free(std::ptr::null_mut());
        aw_string_free(AwString {
            ptr: std::ptr::null_mut(),
            len: 0,
        });
    }
}

#[test]
fn fit_tells_unknown_context_from_an_empty_one() {
    let text = "hello";
    // SAFETY: pointers and lengths match; a null pointer has length 0.
    unsafe {
        let unknown = take(aw_fit(
            text.as_ptr(),
            text.len(),
            false,
            std::ptr::null(),
            0,
        ));
        let empty = take(aw_fit(text.as_ptr(), text.len(), true, std::ptr::null(), 0));
        assert_eq!(
            (unknown.as_deref(), empty.as_deref()),
            (Some("hello"), Some("Hello"))
        );
        let phrase = (b"github".as_ptr(), 6);
        let replaced = take(aw_replace_whole_phrase(
            phrase.0,
            phrase.1,
            b"GitHub".as_ptr(),
            6,
            b"a github".as_ptr(),
            8,
            true,
        ));
        assert_eq!(replaced.as_deref(), Some("a GitHub"));
    }
}

#[test]
fn camel_case_parts_come_back_as_a_list() {
    let term = "PostgreSQL";
    // SAFETY: the list is live until freed, and the pointer and length match.
    unsafe {
        let parts = aw_camel_case_parts(term.as_ptr(), term.len());
        let read: Vec<Option<String>> = (0..=aw_strings_count(parts))
            .map(|index| take(aw_strings_get(parts, index)))
            .collect();
        assert_eq!(
            read,
            [Some("Postgre".to_owned()), Some("SQL".to_owned()), None]
        );
        aw_strings_free(parts);
        aw_strings_free(std::ptr::null_mut());
    }
}

/// Calls a gate function that takes two strings.
fn pair<R>(a: &str, b: &str, function: impl FnOnce(*const u8, usize, *const u8, usize) -> R) -> R {
    function(a.as_ptr(), a.len(), b.as_ptr(), b.len())
}

#[test]
fn the_correction_gate_runs_through_the_interface() {
    // SAFETY: every pointer and length are a live string's, and `ratio` a live double.
    unsafe {
        assert!(pair("i cant go", "I can't go.", |a, b, c, d| {
            aw_correction_allows(a, b, c, d, false)
        }));
        assert!(!pair("i cant go", "I can go.", |a, b, c, d| {
            aw_correction_allows(a, b, c, d, false)
        }));
        let subject = "Subject: Go\n\ngo";
        assert!(pair("go", subject, |a, b, c, d| aw_correction_allows(
            a, b, c, d, true
        )));
        assert!(!pair("go", subject, |a, b, c, d| aw_correction_allows(
            a, b, c, d, false
        )));

        let mut ratio = -1.0;
        assert!(pair("hello", "hallo", |a, b, c, d| {
            aw_correction_ratio(a, b, c, d, &mut ratio)
        }));
        assert_eq!(ratio, 0.2);
        let long = "x".repeat(250_001);
        assert!(!pair(&long, "x", |a, b, c, d| aw_correction_ratio(
            a, b, c, d, &mut ratio
        )));
        assert_eq!(ratio, 0.2, "untouched when there is no ratio");

        let distance = pair("hello", "helo", |a, b, c, d| {
            aw_correction_edit_distance(a, b, c, d)
        });
        assert_eq!(distance, 0.2);
        let merged = pair("hello world", " Hello world\n", |a, b, c, d| {
            aw_correction_safe_merge(a, b, c, d, 0.25)
        });
        assert_eq!(take(merged).as_deref(), Some("Hello world"));
        let kept = pair("hello world", "", |a, b, c, d| {
            aw_correction_safe_merge(a, b, c, d, 0.25)
        });
        assert_eq!(take(kept).as_deref(), Some("hello world"));

        let output = "Subject: Files\n\nSend Maria the 42 files.";
        let source = "Send Maria the 42 files.";
        let rewrite = pair(source, output, |a, b, c, d| {
            aw_correction_accepted(a, b, c, d, true)
        });
        assert_eq!(take(rewrite).as_deref(), Some(output));
        let cleanup = pair(source, output, |a, b, c, d| {
            aw_correction_accepted(a, b, c, d, false)
        });
        assert_eq!(take(cleanup).as_deref(), Some(source));
    }
}
