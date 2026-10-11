//! The `openai-chat` backend against a fake local endpoint.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::collections::BTreeMap;
use std::time::Duration;

use bellcart::arbiter::{Answer, Arbiter, Question};
use bellcart::openai_chat::{ChatConfig, OpenAiChat};
use bellcart::plan::Decision;
use bellcart::verdict::combine;
use common::fake_arbiter::{Reply, start};
use serde_json::json;

fn questions() -> BTreeMap<String, Question> {
    common::core_pack()
        .questions
        .expect("core questions")
        .questions
}

fn backend(base: &str, key: Option<&str>) -> OpenAiChat {
    OpenAiChat::new(ChatConfig {
        endpoint: base.to_owned(),
        model: "test-model".to_owned(),
        key: key.map(str::to_owned),
        timeout: Duration::from_secs(5),
    })
    .expect("a valid endpoint")
}

fn all_answers() -> serde_json::Value {
    json!({
        "destination": "archive",
        "regenerable": "yes",
        "personal": "no",
        "app-data": "unsure",
        "importance": "Minor inconvenience.",
    })
}

#[test]
fn the_backend_sends_the_signature_and_one_enum_per_question() {
    let fake = start(vec![Reply::Answers(all_answers())]);
    backend(&fake.base, Some("secret"))
        .ask("path: Documents/<1>", &questions())
        .expect("an answer");
    let received = fake.received.recv().unwrap();
    assert_eq!(received.url, "/v1/chat/completions");
    assert_eq!(received.authorization.as_deref(), Some("Bearer secret"));
    let body = &received.body;
    assert_eq!(body["model"], "test-model");
    assert_eq!(body["messages"][1]["content"], "path: Documents/<1>");
    let schema = &body["response_format"]["json_schema"]["schema"];
    assert_eq!(body["response_format"]["json_schema"]["strict"], true);
    assert_eq!(
        schema["properties"]["destination"]["enum"],
        json!(["archive", "inspect", "leave", "migrate"])
    );
    assert_eq!(
        schema["properties"]["regenerable"]["enum"],
        json!(["yes", "no", "unsure"])
    );
    assert_eq!(schema["required"].as_array().unwrap().len(), 5);
    assert_eq!(schema["additionalProperties"], false);
}

#[test]
fn without_a_key_no_authorization_is_sent() {
    let fake = start(vec![Reply::Answers(all_answers())]);
    backend(&fake.base, None)
        .ask("path: .", &questions())
        .unwrap();
    assert_eq!(fake.received.recv().unwrap().authorization, None);
}

#[test]
fn answers_come_back_typed() {
    let fake = start(vec![Reply::Answers(all_answers())]);
    let answers = backend(&fake.base, None)
        .ask("path: .", &questions())
        .unwrap();
    assert_eq!(
        answers["destination"],
        Answer::Choice {
            choice: "archive".to_owned(),
            probabilities: BTreeMap::new(),
            confidence: None
        }
    );
    assert_eq!(answers["regenerable"], Answer::Noul { noul: 1.0 });
    assert_eq!(answers["personal"], Answer::Noul { noul: 0.0 });
    assert_eq!(answers["app-data"], Answer::Noul { noul: 0.5 });
    assert_eq!(
        answers["importance"],
        Answer::Score {
            score: 1.0,
            probabilities: BTreeMap::new(),
            confidence: None
        }
    );
}

#[test]
fn values_outside_the_allowed_ones_are_dropped() {
    let fake = start(vec![Reply::Answers(json!({
        "destination": "delete",
        "regenerable": "probably",
        "personal": 0.2,
        "importance": "Huge.",
    }))]);
    let answers = backend(&fake.base, None)
        .ask("path: .", &questions())
        .unwrap();
    assert!(answers.is_empty(), "{answers:?}");
}

#[test]
fn the_backend_reports_no_calibration() {
    let chat = backend("http://localhost:1/v1", None);
    assert!(!chat.calibrated());
    assert_eq!(chat.name(), "openai-chat");
}

#[test]
fn a_confident_leave_from_this_backend_still_waits_for_the_user() {
    let fake = start(vec![Reply::Answers(json!({
        "destination": "leave",
        "regenerable": "yes",
        "personal": "no",
        "app-data": "yes",
        "importance": "Would not notice.",
    }))]);
    let chat = backend(&fake.base, None);
    let answers = chat.ask("path: AppData/Local/Thing", &questions()).unwrap();
    let verdict = combine(&answers, chat.calibrated());
    assert_eq!(verdict.decision, Decision::Review);
    assert_eq!(verdict.opinion, Some(Decision::Leave));
}

#[test]
fn an_error_status_is_an_error_that_says_so() {
    let fake = start(vec![Reply::Raw(401, "{\"error\": \"bad key\"}".to_owned())]);
    let error = backend(&fake.base, Some("wrong"))
        .ask("path: .", &questions())
        .expect_err("a 401 is an error");
    let message = error.to_string();
    assert!(
        message.contains("401") && message.contains("bad key"),
        "{message}"
    );
}

#[test]
fn an_answer_that_is_not_the_expected_json_is_an_error() {
    let fake = start(vec![
        Reply::Raw(200, "not json".to_owned()),
        Reply::Raw(200, "{\"choices\": []}".to_owned()),
    ]);
    let chat = backend(&fake.base, None);
    assert!(chat.ask("path: .", &questions()).is_err());
    assert!(chat.ask("path: .", &questions()).is_err());
}

#[test]
fn a_redirect_is_never_followed() {
    let elsewhere = start(vec![Reply::Answers(all_answers())]);
    let fake = start(vec![Reply::Redirect(format!(
        "{}/chat/completions",
        elsewhere.base
    ))]);
    let result = backend(&fake.base, Some("secret")).ask("path: .", &questions());
    assert!(result.is_err());
    // The other server never heard of the request, nor of the key.
    assert!(
        elsewhere
            .received
            .recv_timeout(Duration::from_millis(300))
            .is_err()
    );
}

#[test]
fn a_silent_endpoint_times_out() {
    let fake = start(vec![Reply::Slow(Duration::from_secs(3))]);
    let chat = OpenAiChat::new(ChatConfig {
        endpoint: fake.base.clone(),
        model: "m".to_owned(),
        key: None,
        timeout: Duration::from_millis(300),
    })
    .unwrap();
    let started = std::time::Instant::now();
    assert!(chat.ask("path: .", &questions()).is_err());
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn plain_http_to_another_machine_is_refused_before_any_request() {
    let result = OpenAiChat::new(ChatConfig {
        endpoint: "http://api.example.com/v1".to_owned(),
        model: "m".to_owned(),
        key: Some("secret".to_owned()),
        timeout: Duration::from_secs(1),
    });
    assert!(result.is_err());
}

// `bellcart arbitrate`, end to end on the real binary.

/// A small profile in a fresh folder, and a folder for outputs.
fn profile(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let base = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&base);
    let profile = base.join("profile");
    std::fs::create_dir_all(profile.join("Wedding Smith")).unwrap();
    std::fs::write(profile.join("Wedding Smith/guests.xlsx"), "x").unwrap();
    let outputs = base.join("outputs");
    std::fs::create_dir_all(&outputs).unwrap();
    (profile, outputs)
}

fn arbitrate(args: &[&str], input: &str) -> std::process::Output {
    use std::io::Write as _;
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_bellcart"))
        .args(args)
        .env_remove("BELLCART_ARBITER_KEY")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("bellcart starts");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().expect("bellcart ends")
}

fn base_args<'a>(profile: &'a str, endpoint: &'a str, log: &'a str) -> Vec<&'a str> {
    vec![
        "--lang",
        "en",
        "arbitrate",
        profile,
        "--endpoint",
        endpoint,
        "--model",
        "m",
        "--log",
        log,
    ]
}

#[test]
fn arbitrate_shows_the_request_and_sends_nothing_without_a_yes() {
    let (profile, outputs) = profile("arbitrate-no");
    let fake = start(vec![Reply::Answers(all_answers())]);
    let log = outputs.join("log.jsonl");
    let args = base_args(profile.to_str().unwrap(), &fake.base, log.to_str().unwrap());
    let output = arbitrate(&args, "n\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("\"response_format\""), "{stdout}");
    assert!(stdout.contains("Nothing was sent."), "{stdout}");
    // Redacted by default: the preview holds no personal name.
    assert!(!stdout.contains("Wedding"), "{stdout}");
    assert!(
        fake.received
            .recv_timeout(Duration::from_millis(300))
            .is_err()
    );
    assert!(!log.exists());
}

#[test]
fn arbitrate_preview_prints_the_signatures_and_sends_nothing() {
    let (profile, outputs) = profile("arbitrate-preview");
    let fake = start(vec![Reply::Answers(all_answers())]);
    let log = outputs.join("log.jsonl");
    let mut args = base_args(profile.to_str().unwrap(), &fake.base, log.to_str().unwrap());
    args.push("--preview");
    let output = arbitrate(&args, "");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("path: "), "{stdout}");
    assert!(
        fake.received
            .recv_timeout(Duration::from_millis(300))
            .is_err()
    );
}

#[test]
fn arbitrate_with_a_yes_decides_logs_and_writes_the_plan() {
    let (profile, outputs) = profile("arbitrate-yes");
    let fake = start(vec![Reply::Answers(all_answers())]);
    let log = outputs.join("log.jsonl");
    let plan_file = outputs.join("plan.json");
    let mut args = base_args(profile.to_str().unwrap(), &fake.base, log.to_str().unwrap());
    args.extend(["--out", plan_file.to_str().unwrap()]);
    let output = arbitrate(&args, "y\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let sent = fake.received.recv().unwrap();
    let state = sent.body["messages"][1]["content"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(!state.contains("Wedding"), "{state}");

    let plan: bellcart::plan::Plan =
        serde_json::from_slice(&std::fs::read(&plan_file).unwrap()).unwrap();
    assert_eq!(plan.tree.decision, Decision::Archive);
    assert_eq!(
        plan.tree.source,
        bellcart::plan::Source::Arbiter {
            backend: "openai-chat".to_owned(),
            opinion: Some(Decision::Archive)
        }
    );

    let entries = bellcart::decision_log::read(&log).unwrap();
    assert_eq!(entries.len(), 1);
    let bellcart::decision_log::Entry::Answered {
        state: logged,
        model,
        verdict,
        ..
    } = &entries[0]
    else {
        panic!("an answered entry");
    };
    assert_eq!(logged, &state);
    assert_eq!(model, "m");
    assert_eq!(verdict.decision, Decision::Archive);
}

#[test]
fn arbitrate_says_when_a_leave_waits_for_the_user() {
    let (profile, outputs) = profile("arbitrate-leave");
    let fake = start(vec![Reply::Answers(json!({
        "destination": "leave",
        "regenerable": "yes",
        "personal": "no",
        "app-data": "yes",
        "importance": "Would not notice.",
    }))]);
    let log = outputs.join("log.jsonl");
    let args = base_args(profile.to_str().unwrap(), &fake.base, log.to_str().unwrap());
    let output = arbitrate(&args, "y\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("the arbiter suggests leaving it: to confirm"),
        "{stdout}"
    );
    assert!(stdout.contains("  Leave behind         0 B"), "{stdout}");
}

#[test]
fn a_failed_request_is_reported_and_the_run_goes_on() {
    let (profile, outputs) = profile("arbitrate-fail");
    let fake = start(vec![Reply::Raw(500, "boom".to_owned())]);
    let log = outputs.join("log.jsonl");
    let args = base_args(profile.to_str().unwrap(), &fake.base, log.to_str().unwrap());
    let output = arbitrate(&args, "y\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("500"), "{stdout}");
}

#[test]
fn arbitrate_refuses_a_log_inside_the_profile() {
    let (profile, _) = profile("arbitrate-inside");
    let log = profile.join("log.jsonl");
    let args = base_args(
        profile.to_str().unwrap(),
        "http://localhost:1/v1",
        log.to_str().unwrap(),
    );
    let output = arbitrate(&args, "y\n");
    assert!(!output.status.success());
    assert!(!log.exists());
}

#[test]
fn the_decision_log_reads_back_what_was_appended() {
    let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("log-roundtrip.jsonl");
    let _ = std::fs::remove_file(&file);
    let corrected = bellcart::decision_log::Entry::Corrected {
        time: 1,
        path: "Downloads".to_owned(),
        decision: Decision::Archive,
    };
    bellcart::decision_log::append(&file, &corrected).unwrap();
    bellcart::decision_log::append(&file, &corrected).unwrap();
    assert_eq!(
        bellcart::decision_log::read(&file).unwrap(),
        [corrected.clone(), corrected]
    );
}

#[test]
fn a_line_cut_short_does_not_spoil_the_log() {
    let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("log-cut.jsonl");
    let good = "{\"event\":\"corrected\",\"time\":1,\"path\":\"a\",\"decision\":\"migrate\"}";
    std::fs::write(&file, format!("{good}\n{{\"event\":\"answ\n{good}\n")).unwrap();
    assert_eq!(bellcart::decision_log::read(&file).unwrap().len(), 2);
}
