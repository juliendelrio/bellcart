//! How an arbiter's answers become a decision. A wrong `leave` loses
//! data: every path to it is here.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::collections::BTreeMap;
use std::path::Path;

use bellcart::arbiter::Answer;
use bellcart::plan::{Decision, Plan, Source};
use bellcart::scan::scan;
use bellcart::tree::MemTree;
use bellcart::verdict::{Reason, Verdict, combine};

/// Answers built from short arguments: `None` leaves a question unanswered.
fn answers(
    destination: Option<&str>,
    confidence: Option<f64>,
    regenerable: Option<f64>,
    personal: Option<f64>,
    importance: Option<f64>,
) -> BTreeMap<String, Answer> {
    let mut out = BTreeMap::new();
    if let Some(choice) = destination {
        out.insert(
            "destination".to_owned(),
            Answer::Choice {
                choice: choice.to_owned(),
                probabilities: BTreeMap::new(),
                confidence,
            },
        );
    }
    for (id, value) in [("regenerable", regenerable), ("personal", personal)] {
        if let Some(noul) = value {
            out.insert(id.to_owned(), Answer::Noul { noul });
        }
    }
    if let Some(score) = importance {
        out.insert(
            "importance".to_owned(),
            Answer::Score {
                score,
                probabilities: BTreeMap::new(),
                confidence: None,
            },
        );
    }
    out
}

/// What a confident, consistent "leave" looks like.
fn clear_leave(confidence: Option<f64>) -> BTreeMap<String, Answer> {
    answers(Some("leave"), confidence, Some(0.95), Some(0.05), Some(0.0))
}

#[test]
fn no_answer_leaves_the_folder_to_review() {
    let verdict = combine(&BTreeMap::new(), false);
    assert_eq!(verdict.decision, Decision::Review);
    assert_eq!(verdict.opinion, None);
}

#[test]
fn migrate_and_archive_are_applied() {
    for (option, decision) in [
        ("migrate", Decision::Migrate),
        ("archive", Decision::Archive),
    ] {
        let verdict = combine(&answers(Some(option), None, None, None, None), false);
        assert_eq!(verdict.decision, decision);
        assert_eq!(verdict.opinion, Some(decision));
    }
}

#[test]
fn inspect_or_an_unknown_option_leaves_the_folder_to_review() {
    for option in ["inspect", "delete", ""] {
        let verdict = combine(&answers(Some(option), None, None, None, None), false);
        assert_eq!(verdict.decision, Decision::Review, "{option:?}");
        assert_eq!(verdict.opinion, None, "{option:?}");
    }
}

#[test]
fn an_uncalibrated_leave_the_answers_support_waits_for_the_user() {
    let verdict = combine(&clear_leave(None), false);
    assert_eq!(verdict.decision, Decision::Review);
    assert_eq!(verdict.opinion, Some(Decision::Leave));
    assert!(verdict.reasons.contains(&Reason::LeaveToConfirm));
}

#[test]
fn an_uncalibrated_leave_stays_unapplied_whatever_confidence_it_claims() {
    let verdict = combine(&clear_leave(Some(1.0)), false);
    assert_eq!(verdict.decision, Decision::Review);
}

#[test]
fn a_leave_the_other_answers_contradict_is_archived() {
    for (why, doubted) in [
        (
            "personal",
            answers(Some("leave"), None, Some(0.95), Some(0.8), Some(0.0)),
        ),
        (
            "important",
            answers(Some("leave"), None, Some(0.95), Some(0.05), Some(3.0)),
        ),
        (
            "not rebuildable",
            answers(Some("leave"), None, Some(0.1), Some(0.05), Some(0.0)),
        ),
        (
            "unsure",
            answers(Some("leave"), None, Some(0.5), Some(0.5), Some(1.0)),
        ),
    ] {
        let verdict = combine(&doubted, false);
        assert_eq!(verdict.decision, Decision::Archive, "{why}");
        assert_eq!(verdict.opinion, Some(Decision::Leave), "{why}");
        assert!(verdict.reasons.contains(&Reason::LeaveDoubted), "{why}");
    }
}

#[test]
fn a_leave_with_unanswered_questions_is_archived() {
    let verdict = combine(&answers(Some("leave"), None, None, None, None), false);
    assert_eq!(verdict.decision, Decision::Archive);
}

#[test]
fn a_calibrated_leave_applies_only_when_confident_and_supported() {
    assert_eq!(
        combine(&clear_leave(Some(0.95)), true).decision,
        Decision::Leave
    );
    // Not confident enough: the user confirms.
    assert_eq!(
        combine(&clear_leave(Some(0.6)), true).decision,
        Decision::Review
    );
    // No confidence reported: the user confirms.
    assert_eq!(combine(&clear_leave(None), true).decision, Decision::Review);
    // Confident, but the other answers disagree: kept.
    let doubted = answers(Some("leave"), Some(0.99), Some(0.95), Some(0.9), Some(0.0));
    assert_eq!(combine(&doubted, true).decision, Decision::Archive);
}

#[test]
fn an_uncalibrated_arbiter_can_never_cause_a_leave() {
    // Every combination of answers, including absurd values.
    let options = [
        None,
        Some("migrate"),
        Some("archive"),
        Some("leave"),
        Some("inspect"),
        Some("x"),
    ];
    let confidences = [
        None,
        Some(0.0),
        Some(0.95),
        Some(1.0),
        Some(f64::NAN),
        Some(7.0),
    ];
    let probabilities = [
        None,
        Some(0.0),
        Some(0.3),
        Some(0.5),
        Some(0.7),
        Some(1.0),
        Some(-1.0),
        Some(f64::INFINITY),
    ];
    let levels = [
        None,
        Some(0.0),
        Some(1.0),
        Some(1.4),
        Some(2.0),
        Some(3.0),
        Some(-5.0),
        Some(f64::NAN),
    ];
    let mut checked = 0;
    for option in options {
        for confidence in confidences {
            for regenerable in probabilities {
                for personal in probabilities {
                    for importance in levels {
                        let set = answers(option, confidence, regenerable, personal, importance);
                        let verdict = combine(&set, false);
                        assert_ne!(verdict.decision, Decision::Leave, "{set:?}");
                        checked += 1;
                    }
                }
            }
        }
    }
    assert_eq!(checked, 6 * 6 * 8 * 8 * 8);
}

#[test]
fn answers_of_the_wrong_type_are_ignored() {
    let mut set = BTreeMap::new();
    set.insert("destination".to_owned(), Answer::Noul { noul: 1.0 });
    set.insert(
        "regenerable".to_owned(),
        Answer::Choice {
            choice: "yes".to_owned(),
            probabilities: BTreeMap::new(),
            confidence: None,
        },
    );
    let verdict = combine(&set, true);
    assert_eq!(verdict.decision, Decision::Review);
    assert!(verdict.reasons.is_empty());
}

#[test]
fn reasons_come_from_the_typed_answers() {
    let verdict = combine(
        &answers(Some("archive"), None, Some(0.42), Some(0.9), Some(2.0)),
        false,
    );
    assert_eq!(
        verdict.reasons,
        [
            Reason::Destination {
                option: "archive".to_owned()
            },
            Reason::Likely {
                question: "regenerable".to_owned(),
                percent: 42
            },
            Reason::Likely {
                question: "personal".to_owned(),
                percent: 90
            },
            Reason::Importance { level: 2 },
        ]
    );
}

// Applying a verdict to the plan.

fn plan_of(tree: &MemTree) -> Plan {
    scan(
        tree,
        Path::new(""),
        &common::core_rules(),
        &common::small_budget(),
    )
    .expect("the scan succeeds")
    .plan
}

fn verdict(decision: Decision, opinion: Option<Decision>) -> Verdict {
    Verdict {
        decision,
        opinion,
        reasons: Vec::new(),
    }
}

#[test]
fn a_verdict_decides_a_review_unit_and_moves_its_totals() {
    let mut plan = plan_of(
        &MemTree::new()
            .file("Downloads/old/a.zip", 700)
            .file("Pictures/b.jpg", 600),
    );
    assert!(plan.apply_verdict(
        "Downloads",
        &verdict(Decision::Archive, Some(Decision::Archive)),
        "test"
    ));
    let node = plan
        .nodes()
        .into_iter()
        .find(|n| n.path == "Downloads")
        .unwrap();
    assert_eq!(node.decision, Decision::Archive);
    assert_eq!(
        node.source,
        Source::Arbiter {
            backend: "test".to_owned(),
            opinion: Some(Decision::Archive)
        }
    );
    assert_eq!(plan.totals[&Decision::Archive].bytes, 700);
    assert_eq!(plan.totals[&Decision::Review].bytes, 600);
}

#[test]
fn a_verdict_never_overrides_a_rule() {
    let mut plan = plan_of(
        &MemTree::new()
            .file("app/node_modules/x.js", 5_000)
            .file("app/a.rs", 10),
    );
    let before = plan.clone();
    assert!(!plan.apply_verdict(
        "app/node_modules",
        &verdict(Decision::Migrate, None),
        "test"
    ));
    assert_eq!(plan, before);
}

#[test]
fn a_verdict_only_applies_to_a_unit() {
    // The root holds no files of its own here: it is cut, not a unit.
    let mut plan = plan_of(
        &MemTree::new()
            .file("Downloads/a.zip", 700)
            .file("Pictures/b.jpg", 600),
    );
    assert_eq!(plan.tree.unit, None);
    assert!(!plan.apply_verdict("", &verdict(Decision::Archive, None), "test"));
    assert!(!plan.apply_verdict("Nowhere", &verdict(Decision::Archive, None), "test"));
}

#[test]
fn a_held_leave_stays_to_review_with_its_opinion_in_the_plan() {
    let mut plan = plan_of(
        &MemTree::new()
            .file("Downloads/a.zip", 700)
            .file("Pictures/b.jpg", 600),
    );
    let held = combine(&clear_leave(None), false);
    assert!(plan.apply_verdict("Downloads", &held, "openai-chat"));
    let json = serde_json::to_value(&plan).unwrap();
    let back: Plan = serde_json::from_value(json).unwrap();
    let node = back
        .nodes()
        .into_iter()
        .find(|n| n.path == "Downloads")
        .unwrap();
    assert_eq!(node.decision, Decision::Review);
    assert_eq!(
        node.source,
        Source::Arbiter {
            backend: "openai-chat".to_owned(),
            opinion: Some(Decision::Leave)
        }
    );
    assert_eq!(back.totals[&Decision::Leave].bytes, 0);
}

#[test]
fn a_version_3_plan_still_parses() {
    let v3 = r#"{
        "version": 3,
        "root": "C:\\Users\\me",
        "totals": {"migrate": {"bytes": 2, "files": 1}},
        "tree": {
            "path": "",
            "decision": "review",
            "source": {"by": "default"},
            "stats": {"bytes": 0, "files": 0},
            "overrides": [{
                "path": ".gitconfig",
                "decision": "migrate",
                "source": {"by": "recipe", "recipe": "git"},
                "stats": {"bytes": 2, "files": 1}
            }]
        },
        "applications": [{"recipe": "git", "name": "Git"}]
    }"#;
    let plan: Plan = serde_json::from_str(v3).expect("a v3 plan parses");
    assert_eq!(plan.version, 3);
}
