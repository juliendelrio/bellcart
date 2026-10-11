//! What `bellcart eval` reports about a backend on a labelled set.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::collections::BTreeMap;

use bellcart::arbiter::{Answer, Arbiter, Question};
use bellcart::eval::{Labelled, Tally, append_label, evaluate, read_labels};
use bellcart::plan::Decision;
use bellcart::signature::Privacy;
use bellcart::{Error, Result};

/// Answers `destination` with the word found after `says:` in the state,
/// and supports a `leave` fully. `calibrated` and confident when asked.
struct Stub {
    calibrated: bool,
}

impl Arbiter for Stub {
    fn name(&self) -> &str {
        "stub"
    }

    fn calibrated(&self) -> bool {
        self.calibrated
    }

    fn ask(&self, state: &str, _: &BTreeMap<String, Question>) -> Result<BTreeMap<String, Answer>> {
        let Some(choice) = state.split("says: ").nth(1) else {
            return Err(Error::Arbiter("no answer".to_owned()));
        };
        let mut answers = BTreeMap::new();
        answers.insert(
            "destination".to_owned(),
            Answer::Choice {
                choice: choice.trim().to_owned(),
                probabilities: BTreeMap::new(),
                confidence: Some(0.99),
            },
        );
        answers.insert("regenerable".to_owned(), Answer::Noul { noul: 1.0 });
        answers.insert("personal".to_owned(), Answer::Noul { noul: 0.0 });
        answers.insert(
            "importance".to_owned(),
            Answer::Score {
                score: 0.0,
                probabilities: BTreeMap::new(),
                confidence: None,
            },
        );
        Ok(answers)
    }
}

fn labelled(path: &str, says: &str, label: Decision) -> Labelled {
    Labelled {
        path: path.to_owned(),
        full: format!("path: {path}\nsays: {says}"),
        redacted: format!("path: <1>\nsays: {says}"),
        label,
    }
}

fn questions() -> BTreeMap<String, Question> {
    common::core_pack().questions.unwrap().questions
}

#[test]
fn agreement_is_counted_per_label() {
    let labels = [
        labelled("a", "migrate", Decision::Migrate),
        labelled("b", "archive", Decision::Migrate),
        labelled("c", "archive", Decision::Archive),
    ];
    let report = evaluate(
        &Stub { calibrated: false },
        &questions(),
        &labels,
        Privacy::Full,
    );
    assert_eq!(
        report.per_label[&Decision::Migrate],
        Tally {
            total: 2,
            agreed: 1
        }
    );
    assert_eq!(
        report.per_label[&Decision::Archive],
        Tally {
            total: 1,
            agreed: 1
        }
    );
    assert_eq!((report.total(), report.agreed()), (3, 2));
    assert!(report.lost.is_empty());
}

#[test]
fn a_held_leave_counts_as_what_the_backend_thought() {
    let labels = [
        labelled("cache", "leave", Decision::Leave),
        labelled("thesis", "leave", Decision::Migrate),
    ];
    let report = evaluate(
        &Stub { calibrated: false },
        &questions(),
        &labels,
        Privacy::Full,
    );
    assert_eq!(report.per_label[&Decision::Leave].agreed, 1);
    // Held for the user, so nothing is lost, but it is flagged.
    assert!(report.lost.is_empty());
    assert_eq!(report.leave_suggested_on_migrate, ["thesis"]);
}

#[test]
fn a_kept_folder_that_would_be_left_is_reported_as_lost() {
    let labels = [
        labelled("thesis", "leave", Decision::Migrate),
        labelled("old photos", "leave", Decision::Archive),
        labelled("cache", "leave", Decision::Leave),
    ];
    let report = evaluate(
        &Stub { calibrated: true },
        &questions(),
        &labels,
        Privacy::Full,
    );
    assert_eq!(report.lost, ["thesis", "old photos"]);
}

#[test]
fn folders_without_an_answer_are_listed_as_failed() {
    let broken = Labelled {
        path: "x".to_owned(),
        full: "no answer here".to_owned(),
        redacted: "no answer here".to_owned(),
        label: Decision::Migrate,
    };
    let report = evaluate(
        &Stub { calibrated: false },
        &questions(),
        &[broken],
        Privacy::Full,
    );
    assert_eq!(report.failed, ["x"]);
    assert_eq!(
        report.per_label[&Decision::Migrate],
        Tally {
            total: 1,
            agreed: 0
        }
    );
}

#[test]
fn the_privacy_mode_picks_which_signature_is_sent() {
    let label = labelled("Wedding", "migrate", Decision::Migrate);
    assert!(label.state(Privacy::Full).contains("Wedding"));
    assert!(!label.state(Privacy::Redacted).contains("Wedding"));
}

#[test]
fn labels_read_back_what_was_appended_and_a_bad_line_is_an_error() {
    let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("labels.jsonl");
    let _ = std::fs::remove_file(&file);
    let one = labelled("a", "migrate", Decision::Migrate);
    append_label(&file, &one).unwrap();
    append_label(&file, &one).unwrap();
    assert_eq!(read_labels(&file).unwrap(), [one.clone(), one]);

    std::fs::write(&file, "{\"path\": \"a\"}\n").unwrap();
    let error = read_labels(&file).expect_err("a bad line is an error");
    assert!(error.to_string().contains("line 1"), "{error}");
}
