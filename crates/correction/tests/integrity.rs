//! The Mac app's `CorrectionIntegrityTests`, one for one.

use std::time::Instant;

use correction::safe_merge;

fn merge(original: &str, edited: &str) -> String {
    merge_at(original, edited, 0.6)
}

fn merge_at(original: &str, edited: &str, ratio: f64) -> String {
    safe_merge(original, edited, ratio)
}

#[test]
fn rejects_lost_negation_and_changed_amounts() {
    for (original, edited) in [
        ("Do not delete the backup.", "Delete the backup."),
        ("I can't approve Thursday.", "I can approve Thursday."),
        ("The balance is negative 125.", "The balance is 125."),
        ("Refund $125 to Jordan.", "Refund $1250 to Jordan."),
        (
            "Please tell Jordan it is ready.",
            "Please tell Logan it is ready.",
        ),
    ] {
        assert_eq!(merge(original, edited), original);
    }
}

#[test]
fn rejects_observed_instruction_loss_and_invented_email() {
    let command = "Run `grep -v error app.log | less`. Do not replace -v with -d.";
    assert_eq!(
        merge_at(command, "Run `grep -v error app.log | less`.", 0.85),
        command
    );
    let relay = "Please let Siobhan know that the refund is $125. No attachment is included.";
    assert_eq!(
        merge(relay, &format!("{relay}\n\nBest regards,\n\n[Your Name]")),
        relay
    );
    let invented = concat!(
        "Hi Siobhan,\n\nPlease note that the refund amount is $125. ",
        "No attachments are included with this message.\n\nBest regards,"
    );
    assert_eq!(merge(relay, invented), relay);
}

#[test]
fn preserves_literal_flags_and_identifiers() {
    for (original, edited) in [
        ("Run grep -v error app.log.", "Run grep -d error app.log."),
        (
            "Send it to editor@example.com.",
            "Send it to owner@example.com.",
        ),
        ("Open /tmp/report.txt.", "Open /tmp/reports.txt."),
    ] {
        assert_eq!(merge_at(original, edited, 0.85), original);
    }
}

#[test]
fn accepts_harmless_grammar_and_existing_email_structure() {
    for (original, edited) in [
        (
            "hello how are you doing today",
            "Hello, how are you doing today?",
        ),
        (
            "she dont have the files yet",
            "She doesn't have the files yet.",
        ),
        (
            "i would of called sooner but i didnt have your number",
            "I would have called sooner, but I didn't have your number.",
        ),
        (
            "no thanks i dont want to join this time",
            "No thanks, I don't want to join this time.",
        ),
        (
            "hi sarah um please sand the attach meant by friday best regards morgan",
            "Hi Sarah,\n\nPlease send the attachment by Friday.\n\nBest regards,\nMorgan",
        ),
        ("I can't approve Thursday.", "I cannot approve Thursday."),
        (
            "The balance is negative 125.",
            "The balance is negative 125.",
        ),
        ("Refund $1250.50 to Jordan.", "Refund $1,250.50 to Jordan."),
        (
            "Hi Siobhan, Tuesday is fine.\nBest regards, Justin",
            "Hi Siobhan,\nTuesday is fine.\nBest regards, Justin",
        ),
    ] {
        assert_eq!(merge(original, edited), edited);
    }
}

#[test]
fn long_unrelated_edits_are_rejected_at_every_category_threshold() {
    let original = "alpha ".repeat(900);
    let edited = "bravo ".repeat(900);
    for ratio in [0.6, 0.85] {
        assert_eq!(merge_at(&original, &edited, ratio), original);
    }
}

#[test]
fn long_small_correction_survives_and_truncation_does_not() {
    let sentence = "The release is ready. ";
    let original = sentence.repeat(450);
    let edited = format!(
        "The release is nearly ready. {}",
        &original[sentence.len()..]
    );
    assert_eq!(merge(&original, &edited), edited.trim());
    let quarter: String = original
        .chars()
        .take(original.chars().count() / 4)
        .collect();
    assert_eq!(merge(&original, &quarter), original);
}

#[test]
fn large_adversarial_comparison_fails_closed_within_budget() {
    let original = "alpha ".repeat(20000);
    let edited = "bravo ".repeat(20000);
    let start = Instant::now();
    assert_eq!(merge(&original, &edited), original);
    assert!(
        start.elapsed().as_secs_f64() < 3.0,
        "Guard must not run an unbounded quadratic comparison"
    );
}
