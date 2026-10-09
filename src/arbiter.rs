//! The arbiter contract: how an undecided folder is put to a model.
//!
//! The shapes follow the System One request format (`state`, `model`,
//! `questions`) so that a decision model, a gateway route or a classic LLM
//! constrained to the same schema can all sit behind [`Arbiter`].
//!
//! V1 ships no model backend in this crate yet: [`NoArbiter`] answers
//! nothing, and undecided folders stay `review` for the user.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A typed question about a folder.
///
/// Keep each question atomic: one judgement a knowledgeable person could
/// make in a few seconds. Combine the answers in code, not in the prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Question {
    /// Pick one option. `criteria` maps each option to its description.
    Choice {
        instructions: String,
        criteria: BTreeMap<String, String>,
    },
    /// Rate on an ordered scale. `criteria` lists the levels from low to high.
    Score {
        instructions: String,
        criteria: Vec<String>,
    },
    /// Is this statement true? Answered with a probability.
    Noul {
        instructions: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        criteria: Option<NoulCriteria>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoulCriteria {
    #[serde(rename = "true")]
    pub when_true: String,
    #[serde(rename = "false")]
    pub when_false: String,
}

/// The body sent to a System One compatible endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub state: String,
    pub model: String,
    pub questions: BTreeMap<String, Question>,
}

/// One answer. `confidence` is `None` when the backend cannot provide a
/// calibrated value: never invent one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Answer {
    Choice {
        choice: String,
        #[serde(default)]
        probabilities: BTreeMap<String, f64>,
        #[serde(default)]
        confidence: Option<f64>,
    },
    Score {
        score: f64,
        #[serde(default)]
        probabilities: BTreeMap<String, f64>,
        #[serde(default)]
        confidence: Option<f64>,
    },
    Noul {
        noul: f64,
    },
}

/// Something that answers questions about a folder signature.
pub trait Arbiter {
    /// Identifies the backend in decision logs.
    fn name(&self) -> &str;

    /// Whether the confidences this backend reports can drive automation.
    /// When `false`, a `leave` answer must never be applied without a rule
    /// or the user confirming it.
    fn calibrated(&self) -> bool;

    /// Answers the questions it can. A missing id means "no opinion".
    fn ask(
        &self,
        state: &str,
        questions: &BTreeMap<String, Question>,
    ) -> crate::Result<BTreeMap<String, Answer>>;
}

/// The `none` backend: rules and recipes only.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoArbiter;

impl Arbiter for NoArbiter {
    fn name(&self) -> &str {
        "none"
    }

    fn calibrated(&self) -> bool {
        false
    }

    fn ask(
        &self,
        _state: &str,
        _questions: &BTreeMap<String, Question>,
    ) -> crate::Result<BTreeMap<String, Answer>> {
        Ok(BTreeMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serializes_to_the_system_one_shape() {
        let mut questions = BTreeMap::new();
        questions.insert(
            "regenerable".to_owned(),
            Question::Noul {
                instructions: "Can this be rebuilt?".to_owned(),
                criteria: None,
            },
        );
        let request = Request {
            state: "path: a/b".to_owned(),
            model: "any".to_owned(),
            questions,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "state": "path: a/b",
                "model": "any",
                "questions": {
                    "regenerable": {"type": "noul", "instructions": "Can this be rebuilt?"}
                }
            })
        );
    }

    #[test]
    fn answers_parse_without_confidence() {
        let answer: Answer =
            serde_json::from_str(r#"{"type": "choice", "choice": "archive"}"#).unwrap();
        assert_eq!(
            answer,
            Answer::Choice {
                choice: "archive".to_owned(),
                probabilities: BTreeMap::new(),
                confidence: None
            }
        );
    }
}
