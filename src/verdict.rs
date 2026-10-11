//! Turns an arbiter's typed answers into a decision, under the prudence
//! rules (SPEC-V1 section 5, D10).
//!
//! The answers are combined here, in code, never in the prompt: the model
//! answers small questions, and what Bellcart does with them stays
//! readable, testable and the same for every backend.
//!
//! - `migrate` and `archive` are applied: a mistake costs disk space.
//! - `leave` is applied only from a calibrated backend that is confident
//!   and whose other answers agree. From any other backend it is held: the
//!   folder stays to review with the opinion shown when the answers agree,
//!   and goes to `archive` when they do not, so nothing is lost.
//! - No answer, an unknown option, or `inspect` leaves the folder to review.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::arbiter::Answer;
use crate::plan::Decision;

/// A `noul` answer at or above this probability counts as yes.
pub const YES: f64 = 0.7;
/// A `noul` answer at or below this probability counts as no.
pub const NO: f64 = 0.3;
/// The confidence a calibrated backend needs for its `leave` to apply.
pub const LEAVE_CONFIDENCE: f64 = 0.9;
/// The highest `importance` level (0 = would not notice) a folder can
/// have and still be left.
pub const LEAVE_MAX_IMPORTANCE: f64 = 1.0;

/// What the answers lead to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Verdict {
    /// The decision applied to the folder.
    pub decision: Decision,
    /// What the arbiter suggested, when it differs from or explains
    /// `decision`. `None` when it gave no usable destination.
    pub opinion: Option<Decision>,
    /// Why, from the typed answers, for the interface to word.
    pub reasons: Vec<Reason>,
}

/// One element of an explanation. Front ends turn each into a translated
/// sentence (`verdict-<kind>`); no generated prose is shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Reason {
    /// The `destination` answer, one of its option names.
    Destination { option: String },
    /// `regenerable`, `personal` or `app-data`, in whole percent.
    Likely { question: String, percent: u8 },
    /// The `importance` level, 0 to 3.
    Importance { level: u8 },
    /// A `leave` held back because the backend is not calibrated, or not
    /// confident enough: the user confirms it.
    LeaveToConfirm,
    /// A `leave` the other answers contradict: kept in `archive` instead.
    LeaveDoubted,
}

/// Combines the answers of a backend. `calibrated` is what the backend
/// declares about its confidences.
pub fn combine(answers: &BTreeMap<String, Answer>, calibrated: bool) -> Verdict {
    let mut reasons = Vec::new();
    let destination = match answers.get("destination") {
        Some(Answer::Choice {
            choice, confidence, ..
        }) => {
            reasons.push(Reason::Destination {
                option: choice.clone(),
            });
            Some((choice.as_str(), *confidence))
        }
        _ => None,
    };
    let likely = |question: &str| match answers.get(question) {
        Some(Answer::Noul { noul }) if noul.is_finite() => Some(noul.clamp(0.0, 1.0)),
        _ => None,
    };
    for question in ["regenerable", "personal", "app-data"] {
        if let Some(p) = likely(question) {
            reasons.push(Reason::Likely {
                question: question.to_owned(),
                percent: percent(p),
            });
        }
    }
    let importance = match answers.get("importance") {
        Some(Answer::Score { score, .. }) if score.is_finite() => {
            let level = score.clamp(0.0, 3.0);
            reasons.push(Reason::Importance {
                level: level.round() as u8,
            });
            Some(level)
        }
        _ => None,
    };

    let review = |reasons| Verdict {
        decision: Decision::Review,
        opinion: None,
        reasons,
    };
    let Some((option, confidence)) = destination else {
        return review(reasons);
    };
    let decided = |decision, reasons| Verdict {
        decision,
        opinion: Some(decision),
        reasons,
    };
    match option {
        "migrate" => decided(Decision::Migrate, reasons),
        "archive" => decided(Decision::Archive, reasons),
        "leave" => {
            // Every other answer must point the same way: rebuildable, not
            // personal, of little importance. A missing answer is doubt.
            let agree = likely("regenerable").is_some_and(|p| p >= YES)
                && likely("personal").is_some_and(|p| p <= NO)
                && importance.is_some_and(|level| level <= LEAVE_MAX_IMPORTANCE);
            let sure = calibrated && confidence.is_some_and(|c| c >= LEAVE_CONFIDENCE);
            if agree && sure {
                decided(Decision::Leave, reasons)
            } else if agree {
                reasons.push(Reason::LeaveToConfirm);
                Verdict {
                    decision: Decision::Review,
                    opinion: Some(Decision::Leave),
                    reasons,
                }
            } else {
                reasons.push(Reason::LeaveDoubted);
                Verdict {
                    decision: Decision::Archive,
                    opinion: Some(Decision::Leave),
                    reasons,
                }
            }
        }
        // `inspect`, or an option the question does not have.
        _ => review(reasons),
    }
}

fn percent(p: f64) -> u8 {
    (p * 100.0).round().clamp(0.0, 100.0) as u8
}
