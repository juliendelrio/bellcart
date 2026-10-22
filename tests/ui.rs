//! `bellcart ui` end to end: the real binary, driven through its HTTP API
//! as the page does.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use bellcart::plan::{Plan, Source};
use common::fake_arbiter::{Reply, start};
use serde_json::{Value, json};

/// A running `bellcart ui`, stopped when dropped.
struct Ui {
    child: Child,
    base: String,
    token: String,
    agent: ureq::Agent,
}

impl Drop for Ui {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Ui {
    fn start(profile: &Path, extra: &[&str]) -> Ui {
        let mut child = Command::new(env!("CARGO_BIN_EXE_bellcart"))
            .args(["--lang", "en", "ui"])
            .arg(profile)
            .args(["--no-browser", "--port", "0"])
            .args(extra)
            // The proposed export folder, where the session is saved, is
            // next to where Bellcart starts: one per test.
            .current_dir(profile.parent().unwrap())
            .env_remove("BELLCART_ARBITER_KEY")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("bellcart starts");
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let url = line
            .split_whitespace()
            .find(|w| w.starts_with("http://"))
            .unwrap_or_else(|| panic!("no URL in {line:?}"));
        let (base, token) = url.split_once("/#").unwrap();
        Ui {
            child,
            base: base.to_owned(),
            token: token.to_owned(),
            agent: ureq::Agent::config_builder()
                .http_status_as_error(false)
                .build()
                .into(),
        }
    }

    fn call(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        token: Option<&str>,
    ) -> (u16, Value) {
        let url = format!("{}{path}", self.base);
        let token = token.unwrap_or(&self.token);
        let mut response = match (method, body) {
            ("GET", _) => self
                .agent
                .get(&url)
                .header("X-Bellcart-Token", token)
                .call(),
            (_, Some(body)) => self
                .agent
                .post(&url)
                .header("X-Bellcart-Token", token)
                .send_json(body),
            (_, None) => self
                .agent
                .post(&url)
                .header("X-Bellcart-Token", token)
                .send_empty(),
        }
        .expect("the server answers");
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap();
        (status, serde_json::from_str(&text).unwrap_or(Value::Null))
    }

    fn get(&self, path: &str) -> Value {
        let (status, body) = self.call("GET", path, None, None);
        assert_eq!(status, 200, "{path}: {body}");
        body
    }

    fn post(&self, path: &str, body: Value) -> (u16, Value) {
        self.call("POST", path, Some(body), None)
    }

    /// Starts the scan and waits for the review.
    fn scan(&self, profile: &Path) -> Value {
        let (status, body) = self.post("/api/scan", json!({ "profile": profile }));
        assert_eq!(status, 200, "{body}");
        self.wait_review()
    }

    /// Waits for the scan started to end, and returns the review.
    fn wait_review(&self) -> Value {
        let started = Instant::now();
        loop {
            let progress = self.get("/api/progress");
            match progress["phase"].as_str() {
                Some("review") => return self.get("/api/review"),
                Some("failed") => panic!("the scan failed: {progress}"),
                _ => {}
            }
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "the scan is too slow"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// A small profile with a rule-settled folder, a recipe, and a folder
/// whose name is personal.
fn profile(name: &str) -> (PathBuf, PathBuf) {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&base);
    let profile = base.join("profile");
    for dir in ["Wedding Smith", "AppData/Local/Temp", ".ssh"] {
        std::fs::create_dir_all(profile.join(dir)).unwrap();
    }
    std::fs::write(profile.join("Wedding Smith/guests.xlsx"), "x".repeat(300)).unwrap();
    std::fs::write(profile.join("AppData/Local/Temp/t.tmp"), "x".repeat(900)).unwrap();
    std::fs::write(profile.join(".ssh/id_ed25519"), "key").unwrap();
    std::fs::write(profile.join(".gitconfig"), "[user]").unwrap();
    (profile, base.join("export"))
}

#[test]
fn the_page_is_served_with_headers_that_keep_it_to_itself() {
    let (profile, _) = profile("ui-page");
    let ui = Ui::start(&profile, &[]);
    for (path, kind) in [
        ("/", "text/html"),
        ("/app.js", "text/javascript"),
        ("/style.css", "text/css"),
    ] {
        let response = ui.agent.get(format!("{}{path}", ui.base)).call().unwrap();
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .map(|v| v.to_str().unwrap().to_owned())
                .unwrap_or_default()
        };
        assert!(header("content-type").starts_with(kind), "{path}");
        assert!(
            header("content-security-policy").contains("default-src 'none'"),
            "{path}"
        );
        assert_eq!(header("x-frame-options"), "DENY");
        assert_eq!(header("cache-control"), "no-store");
    }
}

#[test]
fn the_api_answers_only_the_page_that_has_the_token() {
    let (profile, _) = profile("ui-token");
    let ui = Ui::start(&profile, &[]);
    assert_eq!(ui.call("GET", "/api/session", None, Some("")).0, 403);
    assert_eq!(ui.call("GET", "/api/session", None, Some("0123")).0, 403);
    let (status, _) = ui.call(
        "POST",
        "/api/scan",
        Some(json!({ "profile": profile })),
        Some("x"),
    );
    assert_eq!(status, 403);
    assert_eq!(ui.call("GET", "/api/session", None, None).0, 200);
}

#[test]
fn the_api_refuses_a_request_made_under_another_host_name() {
    // What a page on another site would do after pointing its own domain
    // name at 127.0.0.1 (DNS rebinding).
    let (profile, _) = profile("ui-host");
    let ui = Ui::start(&profile, &[]);
    let address = ui.base.trim_start_matches("http://");
    let mut stream = std::net::TcpStream::connect(address).unwrap();
    let port = address.rsplit(':').next().unwrap();
    write!(
        stream,
        "GET /api/session HTTP/1.1\r\nHost: evil.example:{port}\r\nX-Bellcart-Token: {}\r\nConnection: close\r\n\r\n",
        ui.token
    )
    .unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
}

#[test]
fn the_session_gives_the_strings_and_the_proposed_profile() {
    let (profile, _) = profile("ui-session");
    let ui = Ui::start(&profile, &[]);
    let session = ui.get("/api/session");
    assert_eq!(session["lang"], "en");
    assert_eq!(session["strings"]["decision-leave"], "Leave behind");
    assert_eq!(session["strings"]["ui-step-review"], "Review");
    assert_eq!(session["phase"], "choose");
    assert_eq!(session["profile"], profile.display().to_string());
    assert_eq!(session["arbiter"], Value::Null);
}

#[test]
fn a_whole_session_from_scan_to_export() {
    let (profile, export) = profile("ui-session-full");
    let ui = Ui::start(&profile, &[]);
    let review = ui.scan(&profile);

    // One unit to decide: the profile itself, small enough to judge whole.
    assert_eq!(review["queue"].as_array().unwrap().len(), 1);
    let card = &review["queue"][0];
    assert_eq!(card["path"], "");
    assert!(
        card["folders"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["name"] == "Wedding Smith")
    );
    let settled: Vec<&str> = review["settled"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["path"].as_str().unwrap())
        .collect();
    assert!(settled.contains(&"AppData/Local/Temp"), "{settled:?}");
    assert!(settled.contains(&".gitconfig"), "{settled:?}");

    // Decide the unit, and correct a rule.
    let (status, review) = ui.post("/api/decide", json!({ "path": "", "decision": "archive" }));
    assert_eq!(status, 200);
    assert!(review["queue"].as_array().unwrap().is_empty());
    assert!(review["totals"]["archive"]["bytes"].as_u64().unwrap() > 0);
    let (status, review) = ui.post(
        "/api/decide",
        json!({ "path": "AppData/Local/Temp", "decision": "migrate" }),
    );
    assert_eq!(status, 200);
    assert_eq!(review["totals"]["leave"]["bytes"], 0);

    let (status, body) = ui.post("/api/export", json!({ "folder": export }));
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["files"].as_array().unwrap().len(), 4);

    let plan: Plan =
        serde_json::from_slice(&std::fs::read(export.join("plan.json")).unwrap()).unwrap();
    assert_eq!(plan.tree.source, Source::User);
    let temp = plan
        .nodes()
        .into_iter()
        .find(|n| n.path == "AppData/Local/Temp")
        .unwrap();
    assert_eq!(temp.source, Source::User);
    let log = bellcart::decision_log::read(&export.join("bellcart-decisions.jsonl")).unwrap();
    assert_eq!(log.len(), 2);
    let notebook = std::fs::read_to_string(export.join("notebook.html")).unwrap();
    assert!(notebook.contains("<h3>Git</h3>"), "{notebook}");
}

#[test]
fn a_decision_from_a_parent_folder_keeps_what_rules_and_recipes_settled() {
    let (profile, export) = profile("ui-below");
    let ui = Ui::start(&profile, &[]);
    let before = ui.scan(&profile);

    let (status, review) = ui.post(
        "/api/decide",
        json!({ "path": "", "decision": "migrate", "below": true }),
    );
    assert_eq!(status, 200, "{review}");
    assert_eq!(review["changed"], 1);
    assert!(review["queue"].as_array().unwrap().is_empty());
    assert_eq!(review["totals"]["leave"], before["totals"]["leave"]);

    let (status, _) = ui.post(
        "/api/decide",
        json!({ "path": "Nowhere", "decision": "leave", "below": true }),
    );
    assert_eq!(status, 404);

    ui.post("/api/export", json!({ "folder": export }));
    let log = bellcart::decision_log::read(&export.join("bellcart-decisions.jsonl")).unwrap();
    assert_eq!(log.len(), 1);
}

fn saved_session(profile: &Path) -> Value {
    let file = profile
        .parent()
        .unwrap()
        .join("bellcart-export/bellcart-session.json");
    serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap()
}

#[test]
fn decisions_are_saved_as_they_are_made_and_resumed_by_the_next_session() {
    let (profile, _) = profile("ui-resume");
    {
        let ui = Ui::start(&profile, &[]);
        assert_eq!(ui.get("/api/session")["saved"], Value::Null);
        let review = ui.scan(&profile);
        assert!(
            review["saved_to"]
                .as_str()
                .unwrap()
                .ends_with("bellcart-session.json")
        );
        let (status, _) = ui.post("/api/decide", json!({ "path": "", "decision": "archive" }));
        assert_eq!(status, 200);
        assert_eq!(
            saved_session(&profile)["decisions"][0]["decision"],
            "archive"
        );
    }

    let ui = Ui::start(&profile, &[]);
    assert_eq!(ui.get("/api/session")["saved"]["count"], 1);
    let (status, _) = ui.post("/api/scan", json!({ "profile": profile, "resume": true }));
    assert_eq!(status, 200);
    let review = ui.wait_review();
    assert_eq!(review["resumed"], 1);
    assert!(review["queue"].as_array().unwrap().is_empty());
    assert!(review["totals"]["archive"]["bytes"].as_u64().unwrap() > 0);
}

#[test]
fn starting_over_ignores_the_saved_decisions_until_a_new_one_replaces_them() {
    let (profile, _) = profile("ui-start-over");
    {
        let ui = Ui::start(&profile, &[]);
        ui.scan(&profile);
        ui.post("/api/decide", json!({ "path": "", "decision": "archive" }));
    }
    let ui = Ui::start(&profile, &[]);
    let review = ui.scan(&profile);
    assert_eq!(review["resumed"], 0);
    assert_eq!(review["queue"].as_array().unwrap().len(), 1);
    assert_eq!(
        saved_session(&profile)["decisions"][0]["decision"],
        "archive"
    );

    ui.post("/api/decide", json!({ "path": "", "decision": "migrate" }));
    let decisions = saved_session(&profile)["decisions"].clone();
    assert_eq!(decisions.as_array().unwrap().len(), 1);
    assert_eq!(decisions[0]["decision"], "migrate");
}

#[test]
fn decisions_saved_for_another_profile_are_not_offered() {
    let (profile, _) = profile("ui-other-profile");
    let other = profile.parent().unwrap().join("other");
    std::fs::create_dir_all(&other).unwrap();
    let file = profile
        .parent()
        .unwrap()
        .join("bellcart-export/bellcart-session.json");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let saved = json!({
        "format": 1,
        "profile": other,
        "decisions": [{ "path": "", "decision": "leave" }],
    });
    std::fs::write(&file, saved.to_string()).unwrap();

    let ui = Ui::start(&profile, &[]);
    assert_eq!(ui.get("/api/session")["saved"], Value::Null);
    ui.post("/api/scan", json!({ "profile": profile, "resume": true }));
    let review = ui.wait_review();
    assert_eq!(review["resumed"], 0);
    assert_eq!(review["queue"].as_array().unwrap().len(), 1);
}

#[test]
fn the_export_is_refused_inside_the_profile() {
    let (profile, _) = profile("ui-export-inside");
    let ui = Ui::start(&profile, &[]);
    ui.scan(&profile);
    let inside = profile.join("Export");
    let (status, body) = ui.post("/api/export", json!({ "folder": inside }));
    assert_eq!(status, 400);
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("inside the scanned folder")
    );
    assert!(!inside.exists());
}

#[test]
fn bad_requests_are_refused_cleanly() {
    let (profile, _) = profile("ui-bad");
    let ui = Ui::start(&profile, &[]);
    assert_eq!(ui.call("GET", "/api/review", None, None).0, 409);
    let (status, body) = ui.post("/api/scan", json!({ "profile": profile.join("missing") }));
    assert_eq!(status, 400);
    assert!(body["error"].as_str().unwrap().contains("is not a folder"));
    ui.scan(&profile);
    assert_eq!(
        ui.post(
            "/api/decide",
            json!({ "path": "Nowhere", "decision": "leave" })
        )
        .0,
        404
    );
    assert_eq!(
        ui.post("/api/decide", json!({ "path": "", "decision": "delete" }))
            .0,
        400
    );
    assert_eq!(ui.post("/api/decide", json!("not an object")).0, 400);
    assert_eq!(ui.call("GET", "/api/nothing", None, None).0, 404);
    assert_eq!(ui.call("GET", "/api/arbiter", None, None).0, 404);
}

#[test]
fn closing_the_interface_stops_the_server() {
    let (profile, _) = profile("ui-quit");
    let mut ui = Ui::start(&profile, &[]);
    assert_eq!(ui.post("/api/quit", json!({})).0, 200);
    let started = Instant::now();
    loop {
        if let Some(status) = ui.child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the server is still running"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn the_arbiter_shows_its_request_then_decides_on_a_click() {
    let fake = start(vec![Reply::Answers(json!({
        "destination": "archive",
        "regenerable": "no",
        "personal": "yes",
        "app-data": "no",
        "importance": "Real loss of time or work.",
    }))]);
    let (profile, _) = profile("ui-arbiter");
    let ui = Ui::start(&profile, &["--endpoint", &fake.base, "--model", "m"]);
    assert!(
        ui.get("/api/session")["arbiter"]
            .as_str()
            .unwrap()
            .ends_with("/chat/completions")
    );
    ui.scan(&profile);

    let preview = ui.get("/api/arbiter");
    assert_eq!(preview["count"], 1);
    assert_eq!(preview["privacy"], "redacted");
    let request = preview["request"].as_str().unwrap();
    assert!(request.contains("response_format"));
    assert!(!request.contains("Wedding"), "{request}");
    // Seeing the preview sends nothing.
    assert!(
        fake.received
            .recv_timeout(Duration::from_millis(300))
            .is_err()
    );

    assert_eq!(ui.post("/api/arbiter", json!({})).0, 200);
    let started = Instant::now();
    while ui.get("/api/progress")["arbiter"]["running"] == true {
        assert!(started.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(50));
    }
    let review = ui.get("/api/review");
    assert!(review["queue"].as_array().unwrap().is_empty());
    let decided = review["settled"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["path"] == "")
        .unwrap();
    assert_eq!(decided["by"], "arbiter");
    assert_eq!(decided["decision"], "archive");
}
