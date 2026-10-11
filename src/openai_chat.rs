//! The `openai-chat` arbiter backend: any endpoint that speaks the OpenAI
//! `/v1/chat/completions` API with structured output (OpenAI, a gateway,
//! or a local server such as llama.cpp, Ollama or vLLM).
//!
//! Each question becomes one property of a strict JSON schema whose values
//! are an `enum`, so the model can only pick an option. The backend is not
//! calibrated: it reports no confidence, and its `leave` answers are held
//! for the user (see [`crate::verdict`]).
//!
//! It only ever sends requests to the endpoint the user configured: no
//! redirect is followed, and no proxy is taken from the environment.

use std::collections::BTreeMap;
use std::time::Duration;

use serde_json::{Value, json};

use crate::arbiter::{Answer, Arbiter, Question};
use crate::{Error, Result};

/// What the user configures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatConfig {
    /// The API base (`https://api.openai.com/v1`) or the full
    /// `.../chat/completions` URL.
    pub endpoint: String,
    pub model: String,
    /// Sent as a bearer token. Local servers usually need none.
    pub key: Option<String>,
    pub timeout: Duration,
}

/// Answers for `noul` questions: the model picks a word, mapped to a
/// probability. These are not calibrated probabilities.
const NOUL_WORDS: [(&str, f64); 3] = [("yes", 1.0), ("no", 0.0), ("unsure", 0.5)];

pub struct OpenAiChat {
    agent: ureq::Agent,
    url: String,
    model: String,
    key: Option<String>,
}

impl OpenAiChat {
    pub fn new(config: ChatConfig) -> Result<Self> {
        let url = completions_url(&config.endpoint)?;
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(config.timeout))
            .max_redirects(0)
            .proxy(None)
            .http_status_as_error(false)
            .user_agent(concat!("bellcart/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        Ok(Self {
            agent,
            url,
            model: config.model,
            key: config.key,
        })
    }

    /// The URL requests go to.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The body sent for `state`: what the preview shows.
    pub fn body(&self, state: &str, questions: &BTreeMap<String, Question>) -> Value {
        json!({
            "model": self.model,
            "temperature": 0,
            "messages": [
                {"role": "system", "content": system_prompt(questions)},
                {"role": "user", "content": state},
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "answers",
                    "strict": true,
                    "schema": schema(questions),
                },
            },
        })
    }
}

impl Arbiter for OpenAiChat {
    fn name(&self) -> &str {
        "openai-chat"
    }

    fn calibrated(&self) -> bool {
        false
    }

    fn ask(
        &self,
        state: &str,
        questions: &BTreeMap<String, Question>,
    ) -> Result<BTreeMap<String, Answer>> {
        let mut request = self.agent.post(&self.url);
        if let Some(key) = &self.key {
            request = request.header("Authorization", &format!("Bearer {key}"));
        }
        let mut response = request
            .send_json(self.body(state, questions))
            .map_err(|e| Error::Arbiter(format!("cannot reach {}: {e}", self.url)))?;
        let status = response.status();
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|e| Error::Arbiter(format!("cannot read the answer: {e}")))?;
        if !status.is_success() {
            let start: String = text.chars().take(300).collect();
            return Err(Error::Arbiter(format!(
                "{} answered {status}: {start}",
                self.url
            )));
        }
        parse(&text, questions)
    }
}

/// Accepts a base URL or a full one, and refuses plain `http` beyond this
/// machine: the key and the signatures would cross the network in clear.
fn completions_url(endpoint: &str) -> Result<String> {
    let endpoint = endpoint.trim().trim_end_matches('/');
    let rest = if let Some(rest) = endpoint.strip_prefix("https://") {
        rest
    } else if let Some(rest) = endpoint.strip_prefix("http://") {
        let host = rest.split(['/', ':']).next().unwrap_or_default();
        let local = matches!(host, "localhost" | "127.0.0.1") || rest.starts_with("[::1]");
        if !local {
            return Err(Error::Arbiter(format!(
                "`{endpoint}` must use https: only an endpoint on this machine may use http"
            )));
        }
        rest
    } else {
        return Err(Error::Arbiter(format!(
            "`{endpoint}` is not an http or https URL"
        )));
    };
    if rest.split('/').next().is_none_or(str::is_empty) {
        return Err(Error::Arbiter(format!("`{endpoint}` has no host")));
    }
    Ok(if endpoint.ends_with("/chat/completions") {
        endpoint.to_owned()
    } else {
        format!("{endpoint}/chat/completions")
    })
}

/// Tells the model what it is doing and what each question means.
fn system_prompt(questions: &BTreeMap<String, Question>) -> String {
    let mut out = String::from(
        "You help someone who is replacing their PC decide what to do with each \
         folder of the old one. You are given a summary of one folder: its path, \
         size, dates, file types and largest entries. You never see file contents. \
         Answer every question with one of the allowed values.\n",
    );
    for (id, question) in questions {
        match question {
            Question::Choice {
                instructions,
                criteria,
            } => {
                out.push_str(&format!("\n{id}: {instructions}\n"));
                for (option, meaning) in criteria {
                    out.push_str(&format!("- {option}: {meaning}\n"));
                }
            }
            Question::Score {
                instructions,
                criteria,
            } => {
                out.push_str(&format!(
                    "\n{id}: {instructions} Answer with the level that fits, from lowest to highest:\n"
                ));
                for level in criteria {
                    out.push_str(&format!("- {level}\n"));
                }
            }
            Question::Noul {
                instructions,
                criteria,
            } => {
                out.push_str(&format!(
                    "\n{id}: {instructions} Answer yes, no or unsure.\n"
                ));
                if let Some(c) = criteria {
                    out.push_str(&format!("- yes: {}\n- no: {}\n", c.when_true, c.when_false));
                }
            }
        }
    }
    out
}

/// One `enum` property per question, all required.
fn schema(questions: &BTreeMap<String, Question>) -> Value {
    let mut properties = serde_json::Map::new();
    for (id, question) in questions {
        let values: Vec<String> = match question {
            Question::Choice { criteria, .. } => criteria.keys().cloned().collect(),
            Question::Score { criteria, .. } => criteria.clone(),
            Question::Noul { .. } => NOUL_WORDS.iter().map(|(w, _)| (*w).to_owned()).collect(),
        };
        properties.insert(id.clone(), json!({"type": "string", "enum": values}));
    }
    let required: Vec<&String> = questions.keys().collect();
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

/// Reads the answers out of a chat completion. A value outside the allowed
/// ones is dropped: no opinion is better than a made-up one.
fn parse(text: &str, questions: &BTreeMap<String, Question>) -> Result<BTreeMap<String, Answer>> {
    let completion: Value = serde_json::from_str(text)
        .map_err(|e| Error::Arbiter(format!("the answer is not JSON: {e}")))?;
    let content = completion
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Arbiter("the answer has no message content".to_owned()))?;
    let values: BTreeMap<String, Value> = serde_json::from_str(content)
        .map_err(|e| Error::Arbiter(format!("the message is not the expected JSON: {e}")))?;

    let mut answers = BTreeMap::new();
    for (id, question) in questions {
        let Some(value) = values.get(id).and_then(Value::as_str) else {
            continue;
        };
        let answer = match question {
            Question::Choice { criteria, .. } => {
                criteria.contains_key(value).then(|| Answer::Choice {
                    choice: value.to_owned(),
                    probabilities: BTreeMap::new(),
                    confidence: None,
                })
            }
            Question::Score { criteria, .. } => {
                criteria
                    .iter()
                    .position(|l| l == value)
                    .map(|level| Answer::Score {
                        score: level as f64,
                        probabilities: BTreeMap::new(),
                        confidence: None,
                    })
            }
            Question::Noul { .. } => NOUL_WORDS
                .iter()
                .find(|(word, _)| *word == value)
                .map(|(_, noul)| Answer::Noul { noul: *noul }),
        };
        if let Some(answer) = answer {
            answers.insert(id.clone(), answer);
        }
    }
    Ok(answers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_get_the_completions_path() {
        assert_eq!(
            completions_url("https://api.openai.com/v1").unwrap(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            completions_url("https://gw.example/v1/chat/completions/").unwrap(),
            "https://gw.example/v1/chat/completions"
        );
        assert_eq!(
            completions_url("http://localhost:11434/v1").unwrap(),
            "http://localhost:11434/v1/chat/completions"
        );
        assert!(completions_url("http://127.0.0.1:8080/v1").is_ok());
        assert!(completions_url("http://[::1]:8080/v1").is_ok());
    }

    #[test]
    fn plain_http_beyond_this_machine_is_refused() {
        for endpoint in [
            "http://api.example.com/v1",
            "http://localhost.example.com/v1",
            "http://192.168.1.10:8080/v1",
            "http://[2001:db8::1]:8080/v1",
            "ftp://example.com",
            "api.example.com/v1",
            "https://",
        ] {
            assert!(completions_url(endpoint).is_err(), "{endpoint}");
        }
    }
}
