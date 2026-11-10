//! The pure parts of the Mac app's `SemanticCorrectionServiceTests`,
//! `SemanticCorrectionServiceCoverageTests` and `ErrorPropagationIntegrationTests` (edit distance
//! and `safeMerge`), and `DictationWorkflowTests`' check of `DictationWriting.accepted`, one for
//! one.

use correction::{MAX_CHANGE_RATIO, Operation, accepted, normalized_edit_distance, safe_merge};

fn assert_close(value: f64, expected: f64, accuracy: f64) {
    assert!(
        (value - expected).abs() <= accuracy,
        "{value} is not {expected}"
    );
}

// SemanticCorrectionServiceTests: normalized edit distance.

#[test]
fn normalized_edit_distance_identical_strings() {
    assert_eq!(normalized_edit_distance("hello", "hello"), 0.0);
}

#[test]
fn normalized_edit_distance_completely_different() {
    assert_eq!(normalized_edit_distance("abc", "xyz"), 1.0);
}

#[test]
fn normalized_edit_distance_one_char_difference() {
    assert_eq!(normalized_edit_distance("hello", "hallo"), 0.2);
}

#[test]
fn normalized_edit_distance_empty_original() {
    assert_eq!(normalized_edit_distance("", "hello"), 1.0);
}

#[test]
fn normalized_edit_distance_empty_corrected() {
    assert_eq!(normalized_edit_distance("hello", ""), 1.0);
}

#[test]
fn normalized_edit_distance_both_empty() {
    assert_eq!(normalized_edit_distance("", ""), 0.0);
}

#[test]
fn normalized_edit_distance_insertion() {
    assert_eq!(normalized_edit_distance("helo", "hello"), 0.2);
}

#[test]
fn normalized_edit_distance_deletion() {
    assert_eq!(normalized_edit_distance("hello", "helo"), 0.2);
}

#[test]
fn normalized_edit_distance_unicode() {
    assert_eq!(normalized_edit_distance("café", "cafe"), 0.25);
}

#[test]
fn normalized_edit_distance_emoji() {
    // "hello 👋" is 7 characters (the emoji is one Character).
    assert_close(
        normalized_edit_distance("hello 👋", "hello 🙋"),
        1.0 / 7.0,
        0.001,
    );
}

// SemanticCorrectionServiceTests: safe merge.

#[test]
fn safe_merge_accepts_small_changes() {
    assert_eq!(
        safe_merge("hello world", "Hello world", 0.25),
        "Hello world"
    );
}

#[test]
fn safe_merge_rejects_large_changes() {
    let result = safe_merge("hello world", "completely different text here", 0.25);
    assert_eq!(result, "hello world");
}

#[test]
fn safe_merge_rejects_empty_corrected() {
    assert_eq!(safe_merge("hello world", "", 0.25), "hello world");
}

#[test]
fn safe_merge_trims_whitespace() {
    assert_eq!(
        safe_merge("hello world", "  hello world  \n", 0.25),
        "hello world"
    );
}

#[test]
fn safe_merge_identical_strings() {
    assert_eq!(
        safe_merge("hello world", "hello world", 0.25),
        "hello world"
    );
}

#[test]
fn safe_merge_at_exact_threshold() {
    // 0.2 is not more than 0.2, so the boundary is accepted.
    assert_eq!(safe_merge("hello", "hallo", 0.2), "hallo");
}

#[test]
fn safe_merge_just_under_threshold() {
    assert_eq!(safe_merge("hello", "hallo", 0.21), "hallo");
}

#[test]
fn safe_merge_with_filler_word_removal() {
    let original = "So um I was like uh thinking about it";
    let corrected = "So I was thinking about it";
    assert_eq!(safe_merge(original, corrected, 0.4), corrected);
}

#[test]
fn safe_merge_with_punctuation_fix() {
    let original = "hello how are you doing today";
    let corrected = "Hello, how are you doing today?";
    assert_eq!(safe_merge(original, corrected, 0.25), corrected);
}

#[test]
fn safe_merge_preserves_original_on_hallucination() {
    let original = "The meeting is at 3pm";
    let corrected = "The meeting is at 3pm. Please bring your laptop and prepare the quarterly report for discussion.";
    assert_eq!(safe_merge(original, corrected, 0.25), original);
}

#[test]
fn safe_merge_with_newlines() {
    let result = safe_merge("line one\nline two", "Line one.\nLine two.", 0.25);
    assert_eq!(result, "Line one.\nLine two.");
}

#[test]
fn normalized_edit_distance_long_strings() {
    let original = "a".repeat(1000);
    let corrected = "a".repeat(990) + &"b".repeat(10);
    assert_close(normalized_edit_distance(&original, &corrected), 0.01, 0.001);
}

#[test]
fn safe_merge_with_zero_threshold() {
    assert_eq!(safe_merge("hello", "Hello", 0.0), "hello");
}

#[test]
fn safe_merge_with_full_threshold() {
    let result = safe_merge("hello", "completely different", 1.0);
    assert_eq!(result, "completely different");
}

#[test]
fn mlx_max_change_ratio_is_permissive() {
    let original = "so um I was like thinking about this";
    let corrected = "So I was thinking about this";
    assert_eq!(safe_merge(original, corrected, MAX_CHANGE_RATIO), corrected);
}

// SemanticCorrectionServiceCoverageTests.

#[test]
fn safe_merge_rejects_whitespace_corrected_at_low_threshold() {
    assert_eq!(safe_merge("keep me", "   \n  ", 0.25), "keep me");
}

#[test]
fn normalized_edit_distance_longer_first_arg() {
    let distance = normalized_edit_distance("hello there", "hello");
    assert!(distance > 0.0 && distance <= 1.0);
}

// ErrorPropagationIntegrationTests.

#[test]
fn semantic_correction_safe_merge_with_empty_correction() {
    assert_eq!(safe_merge("Original text", "", 0.6), "Original text");
}

#[test]
fn semantic_correction_safe_merge_exceeds_ratio() {
    let corrected = "Completely different text that bears no resemblance";
    assert_eq!(safe_merge("Hello world", corrected, 0.25), "Hello world");
}

#[test]
fn semantic_correction_safe_merge_accepts_minor_changes() {
    assert_eq!(
        safe_merge("hello world", "Hello world.", 0.6),
        "Hello world."
    );
}

#[test]
fn propagation_normalized_edit_distance_identical_strings() {
    assert_eq!(normalized_edit_distance("Hello world", "Hello world"), 0.0);
}

#[test]
fn propagation_normalized_edit_distance_completely_different() {
    assert_eq!(normalized_edit_distance("abc", "xyz"), 1.0);
}

#[test]
fn propagation_normalized_edit_distance_empty_string() {
    assert_eq!(normalized_edit_distance("", "hello"), 1.0);
}

// DictationWorkflowTests.

#[test]
fn rewrite_allows_formatting_but_still_protects_numbers_and_negation() {
    let source = "Send Maria the 42 files, but do not upload them.";
    let output = "Subject: Files\n\nPlease send Maria the 42 files. Do not upload them.";
    assert_eq!(accepted(source, output, Operation::Rewrite), output);
    assert_eq!(accepted(source, output, Operation::Cleanup), source);
    let changed = output.replace("42", "43");
    assert_eq!(accepted(source, &changed, Operation::Rewrite), source);
    let unnegated = output.replace("not ", "");
    assert_eq!(accepted(source, &unnegated, Operation::Rewrite), source);
    let recipients = format!("Hi Alice,\n{source}\nBest,\nBob");
    assert_eq!(accepted(source, &recipients, Operation::Rewrite), source);
}
