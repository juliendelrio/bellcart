//! `bellcart ui`: the review interface, served on this machine only and
//! opened in the default browser (D7).
//!
//! One thread answers requests; the scan and the arbiter run on their own
//! threads and share their state with it behind a mutex. The page and its
//! script are embedded in the binary: nothing is fetched from anywhere.
//!
//! Only this machine can reach the server (it listens on 127.0.0.1), and
//! only the page Bellcart opened can use its API: every call must carry a
//! random token the page gets in the URL fragment, which never leaves the
//! browser, and a `Host` of 127.0.0.1, which stops DNS rebinding.

use std::collections::BTreeMap;
use std::hash::{BuildHasher, Hasher};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use anyhow::{Context, Result};
use bellcart::arbiter::Arbiter;
use bellcart::decision_log::{self, Entry};
use bellcart::openai_chat::OpenAiChat;
use bellcart::pack::{Library, QuestionSet, RuleSet};
use bellcart::plan::{Decision, PlanNode, Source, SplitBudget};
use bellcart::recipe::{Recipes, Variables};
use bellcart::scan::{Progress, Scan, scan_with};
use bellcart::signature::{Privacy, Signature, signatures};
use bellcart::tree::OsTree;
use bellcart::verdict::combine;
use serde_json::{Value, json};
use tiny_http::{Header, Method, Request, Response, Server};

use crate::i18n::Localizer;
use crate::saved::{self, Choice, Saved};

const INDEX: &str = include_str!("../../../web/index.html");
const SCRIPT: &str = include_str!("../../../web/app.js");
const STYLE: &str = include_str!("../../../web/style.css");

/// The largest request body accepted: requests carry a path or a choice.
const MAX_BODY: u64 = 64 * 1024;

pub struct UiOptions {
    /// The profile proposed on the first screen.
    pub profile: PathBuf,
    /// 0 picks a free port.
    pub port: u16,
    pub open_browser: bool,
    /// An arbiter the user configured, with how much it may be sent.
    pub arbiter: Option<(OpenAiChat, Privacy, String)>,
}

/// Where the session stands.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Phase {
    Choose,
    Scanning,
    Review,
    Failed(String),
}

/// The state the request thread and the worker threads share.
struct Session {
    phase: Phase,
    profile: Option<PathBuf>,
    progress: Arc<Progress>,
    scan: Option<Scan>,
    /// Entries to append to the decision log on export: arbiter answers
    /// and the user's decisions.
    log: Vec<Entry>,
    arbiter_done: usize,
    arbiter_total: usize,
    arbiter_running: bool,
    arbiter_error: Option<String>,
    /// Where this session's decisions are saved as they are made, and what
    /// is saved there. `None` when the export folder is inside the profile.
    saved: Option<(PathBuf, Saved)>,
    /// How many saved decisions the last scan replayed.
    resumed: usize,
}

pub fn run(
    options: UiOptions,
    library: Library,
    questions: Option<QuestionSet>,
    text: &Localizer,
) -> Result<()> {
    let server = Server::http(("127.0.0.1", options.port))
        .map_err(|e| anyhow::anyhow!("cannot listen on 127.0.0.1: {e}"))?;
    let port = server
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .context("the server has no port")?;
    let token = token();
    let url = format!("http://127.0.0.1:{port}/#{token}");
    crate::say(&format!(
        "{}\n",
        text.with("ui-listening", &[("url", url.as_str().into())])
    ))?;
    if options.open_browser {
        open_in_browser(&url);
    }

    let library = Arc::new(library);
    let questions = Arc::new(questions);
    let arbiter = options.arbiter.map(Arc::new);
    let session = Arc::new(Mutex::new(Session {
        phase: Phase::Choose,
        profile: None,
        progress: Arc::new(Progress::default()),
        scan: None,
        log: Vec::new(),
        arbiter_done: 0,
        arbiter_total: 0,
        arbiter_running: false,
        arbiter_error: None,
        saved: None,
        resumed: 0,
    }));
    let app = App {
        port,
        token,
        default_profile: options.profile,
        library,
        questions,
        arbiter,
        session,
    };

    for request in server.incoming_requests() {
        let quit = request.url() == "/api/quit" && *request.method() == Method::Post;
        let allowed = quit && app.allowed(&request);
        app.handle(request, text);
        if allowed {
            break;
        }
    }
    Ok(())
}

/// What every request needs.
struct App {
    port: u16,
    token: String,
    default_profile: PathBuf,
    library: Arc<Library>,
    questions: Arc<Option<QuestionSet>>,
    arbiter: Option<Arc<(OpenAiChat, Privacy, String)>>,
    session: Arc<Mutex<Session>>,
}

/// An answer to a request: a status and a JSON body.
type Reply = (u16, Value);

fn error(status: u16, message: impl Into<String>) -> Reply {
    (status, json!({ "error": message.into() }))
}

impl App {
    fn session(&self) -> MutexGuard<'_, Session> {
        // A worker that panicked leaves the data as it was: still usable.
        self.session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The request comes from the page Bellcart opened, through this
    /// machine's address.
    fn allowed(&self, request: &Request) -> bool {
        let token_ok = header_value(request, "X-Bellcart-Token").is_some_and(|t| t == self.token);
        self.host_ok(request) && token_ok
    }

    /// The request was addressed to this machine by its own name, not to
    /// another name that happens to point here (DNS rebinding).
    fn host_ok(&self, request: &Request) -> bool {
        header_value(request, "Host").is_some_and(|host| {
            host == format!("127.0.0.1:{}", self.port) || host == format!("localhost:{}", self.port)
        })
    }

    fn handle(&self, mut request: Request, text: &Localizer) {
        let path = request
            .url()
            .split('?')
            .next()
            .unwrap_or_default()
            .to_owned();
        let method = request.method().clone();
        if !path.starts_with("/api/") {
            let asset = match (self.host_ok(&request), path.as_str()) {
                (true, "/") => Some((INDEX, "text/html; charset=utf-8")),
                (true, "/app.js") => Some((SCRIPT, "text/javascript; charset=utf-8")),
                (true, "/style.css") => Some((STYLE, "text/css; charset=utf-8")),
                _ => None,
            };
            let response = match asset {
                Some((body, kind)) => {
                    with_headers(Response::from_string(body), &[("Content-Type", kind)])
                }
                None => Response::from_string("not found").with_status_code(404),
            };
            let _ = request.respond(secured(response));
            return;
        }
        let (status, body) = if !self.allowed(&request) {
            error(403, "forbidden")
        } else {
            let mut raw = String::new();
            let read = request.as_reader().take(MAX_BODY).read_to_string(&mut raw);
            let input: Value = if raw.trim().is_empty() {
                Value::Null
            } else {
                serde_json::from_str(&raw).unwrap_or(Value::String(String::new()))
            };
            match (read, method, path.as_str()) {
                (Err(_), ..) => error(400, "cannot read the request"),
                (_, Method::Get, "/api/session") => self.session_info(text),
                (_, Method::Post, "/api/scan") => self.start_scan(&input, text),
                (_, Method::Get, "/api/progress") => self.progress(),
                (_, Method::Get, "/api/review") => self.review(text),
                (_, Method::Post, "/api/decide") => self.decide(&input, text),
                (_, Method::Post, "/api/export") => self.export(&input, text),
                (_, Method::Get, "/api/arbiter") => self.arbiter_preview(),
                (_, Method::Post, "/api/arbiter") => self.start_arbiter(),
                (_, Method::Post, "/api/quit") => (200, json!({ "ok": true })),
                _ => error(404, "no such call"),
            }
        };
        let response = with_headers(
            Response::from_string(body.to_string()),
            &[("Content-Type", "application/json; charset=utf-8")],
        )
        .with_status_code(status);
        let _ = request.respond(secured(response));
    }

    fn session_info(&self, text: &Localizer) -> Reply {
        let session = self.session();
        let profile = session.profile.as_ref().unwrap_or(&self.default_profile);
        let resumable = saved::location(profile, &default_export(profile))
            .and_then(|file| Saved::load(&file, profile).map(|s| (file, s)))
            .filter(|(_, s)| !s.decisions.is_empty())
            .map(|(file, s)| {
                json!({
                    "profile": s.profile,
                    "count": s.decisions.len(),
                    "file": file.display().to_string(),
                })
            });
        (
            200,
            json!({
                "saved": resumable,
                "lang": text.lang(),
                "strings": text.patterns(),
                "profile": session
                    .profile
                    .as_ref()
                    .unwrap_or(&self.default_profile)
                    .display()
                    .to_string(),
                "phase": phase_name(&session.phase),
                "arbiter": self.arbiter.as_ref().map(|a| a.0.url().to_owned()),
                "export": default_export(&self.default_profile).display().to_string(),
            }),
        )
    }

    fn start_scan(&self, input: &Value, text: &Localizer) -> Reply {
        let Some(profile) = input.get("profile").and_then(Value::as_str) else {
            return error(400, "a profile is needed");
        };
        let profile = PathBuf::from(profile.trim());
        if !profile.is_dir() {
            return error(
                400,
                text.with(
                    "ui-not-a-folder",
                    &[("path", profile.display().to_string().into())],
                ),
            );
        }
        let progress = Arc::new(Progress::default());
        // Resume the decisions saved for this profile, or start over: the
        // file is then rewritten at the first new decision.
        let resume = input.get("resume").and_then(Value::as_bool) == Some(true);
        let saved = saved::location(&profile, &default_export(&profile)).map(|file| {
            let kept = resume.then(|| Saved::load(&file, &profile)).flatten();
            let saved = kept.unwrap_or_else(|| Saved::new(&profile));
            (file, saved)
        });
        let replay: Vec<Choice> = saved
            .as_ref()
            .map(|(_, s)| s.decisions.clone())
            .unwrap_or_default();
        {
            let mut session = self.session();
            if session.phase == Phase::Scanning {
                return error(409, "a scan is running");
            }
            session.phase = Phase::Scanning;
            session.profile = Some(profile.clone());
            session.progress = Arc::clone(&progress);
            session.scan = None;
            session.log.clear();
            session.saved = saved;
            session.resumed = 0;
        }
        let shared = Arc::clone(&self.session);
        let library = Arc::clone(&self.library);
        thread::spawn(move || {
            let result = RuleSet::compile(&library.rules)
                .map_err(anyhow::Error::from)
                .and_then(|rules| {
                    let found =
                        Recipes::detect(&OsTree, &profile, &library.recipes, &Variables::profile());
                    Ok(scan_with(
                        &OsTree,
                        &profile,
                        &rules,
                        &found,
                        &SplitBudget::default(),
                        &progress,
                    )?)
                });
            let mut session = shared
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            match result {
                Ok(mut scan) => {
                    let time = decision_log::now();
                    let mut resumed = 0;
                    for choice in &replay {
                        let changed = saved::apply(&mut scan.plan, choice);
                        resumed += usize::from(!changed.is_empty());
                        session
                            .log
                            .extend(changed.into_iter().map(|path| Entry::Corrected {
                                time,
                                path,
                                decision: choice.decision,
                            }));
                    }
                    session.resumed = resumed;
                    session.scan = Some(scan);
                    session.phase = Phase::Review;
                }
                Err(e) => session.phase = Phase::Failed(format!("{e:#}")),
            }
        });
        (200, json!({ "ok": true }))
    }

    fn progress(&self) -> Reply {
        let session = self.session();
        let counts = session.progress.counts();
        let failed = match &session.phase {
            Phase::Failed(message) => Some(message.clone()),
            _ => None,
        };
        (
            200,
            json!({
                "phase": phase_name(&session.phase),
                "folders": counts.folders,
                "files": counts.files,
                "bytes": counts.bytes,
                "error": failed,
                "arbiter": {
                    "running": session.arbiter_running,
                    "done": session.arbiter_done,
                    "total": session.arbiter_total,
                    "error": session.arbiter_error,
                },
            }),
        )
    }

    fn review(&self, text: &Localizer) -> Reply {
        let session = self.session();
        let Some(scan) = &session.scan else {
            return error(409, "nothing scanned yet");
        };
        let mut model = review_model(scan, &self.library, text);
        model["resumed"] = json!(session.resumed);
        model["saved_to"] = json!(session.saved.as_ref().map(|(f, _)| f.display().to_string()));
        (200, model)
    }

    fn decide(&self, input: &Value, text: &Localizer) -> Reply {
        let path = input.get("path").and_then(Value::as_str);
        let decision = input
            .get("decision")
            .and_then(|d| serde_json::from_value::<Decision>(d.clone()).ok());
        let (Some(path), Some(decision)) = (path, decision) else {
            return error(400, "a path and a decision are needed");
        };
        // `below`: the decision holds for the folder and all it contains,
        // chosen from a parent in the card's path.
        let below = input.get("below").and_then(Value::as_bool) == Some(true);
        let mut session = self.session();
        let Some(scan) = session.scan.as_mut() else {
            return error(409, "nothing scanned yet");
        };
        let changed = if below {
            scan.plan.apply_user_below(path, decision)
        } else if scan.plan.apply_user(path, decision) {
            vec![path.to_owned()]
        } else {
            Vec::new()
        };
        if changed.is_empty() {
            return error(404, "no such folder in the plan");
        }
        let count = changed.len();
        // Saved at once, so that closing the page loses nothing.
        let save_error = session.saved.as_mut().and_then(|(file, saved)| {
            saved.decisions.push(Choice {
                path: path.to_owned(),
                decision,
                below,
            });
            saved.write(file).err().map(|e| format!("{e:#}"))
        });
        let time = decision_log::now();
        session
            .log
            .extend(changed.into_iter().map(|path| Entry::Corrected {
                time,
                path,
                decision,
            }));
        let Some(scan) = &session.scan else {
            return error(409, "nothing scanned yet");
        };
        let mut model = review_model(scan, &self.library, text);
        model["changed"] = json!(count);
        model["save_error"] = json!(save_error);
        (200, model)
    }

    fn export(&self, input: &Value, text: &Localizer) -> Reply {
        let Some(folder) = input.get("folder").and_then(Value::as_str) else {
            return error(400, "a folder is needed");
        };
        let folder = PathBuf::from(folder.trim());
        let mut session = self.session();
        let (Some(scan), Some(profile)) = (&session.scan, &session.profile) else {
            return error(409, "nothing scanned yet");
        };
        if crate::outside(profile, &folder.join("plan.json")).is_err() {
            return error(
                400,
                text.with(
                    "ui-export-inside",
                    &[("profile", profile.display().to_string().into())],
                ),
            );
        }
        let written = write_exports(&folder, scan, &session.log, &self.library, text);
        match written {
            Ok(files) => {
                session.log.clear();
                (200, json!({ "files": files }))
            }
            Err(e) => error(500, format!("{e:#}")),
        }
    }

    fn arbiter_preview(&self) -> Reply {
        let Some(arbiter) = &self.arbiter else {
            return error(404, "no arbiter is configured");
        };
        let Some(questions) = self.questions.as_ref() else {
            return error(500, "no questions");
        };
        let session = self.session();
        let Some(scan) = &session.scan else {
            return error(409, "nothing scanned yet");
        };
        let pending = pending_units(scan);
        let Some(first) = pending.first() else {
            return (200, json!({ "count": 0 }));
        };
        let (chat, privacy, model) = arbiter.as_ref();
        let body = chat.body(&first.for_privacy(*privacy).to_text(), &questions.questions);
        (
            200,
            json!({
                "count": pending.len(),
                "url": chat.url(),
                "model": model,
                "privacy": if *privacy == Privacy::Full { "full" } else { "redacted" },
                "request": serde_json::to_string_pretty(&body).unwrap_or_default(),
            }),
        )
    }

    fn start_arbiter(&self) -> Reply {
        let Some(arbiter) = self.arbiter.clone() else {
            return error(404, "no arbiter is configured");
        };
        let Some(questions) = self.questions.as_ref().clone() else {
            return error(500, "no questions");
        };
        let units: Vec<(String, String)> = {
            let mut session = self.session();
            if session.arbiter_running {
                return error(409, "the arbiter is running");
            }
            let Some(scan) = &session.scan else {
                return error(409, "nothing scanned yet");
            };
            let units: Vec<(String, String)> = pending_units(scan)
                .iter()
                .map(|s| (s.path.clone(), s.for_privacy(arbiter.1).to_text()))
                .collect();
            session.arbiter_running = true;
            session.arbiter_done = 0;
            session.arbiter_total = units.len();
            session.arbiter_error = None;
            units
        };
        let shared = Arc::clone(&self.session);
        thread::spawn(move || {
            let (chat, privacy, model) = arbiter.as_ref();
            let lock = || {
                shared
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
            };
            for (path, state) in units {
                let answers = match chat.ask(&state, &questions.questions) {
                    Ok(answers) => answers,
                    Err(e) => {
                        let mut session = lock();
                        session.arbiter_error = Some(e.to_string());
                        session.arbiter_running = false;
                        return;
                    }
                };
                let verdict = combine(&answers, chat.calibrated());
                let mut session = lock();
                if let Some(scan) = session.scan.as_mut() {
                    scan.plan.apply_verdict(&path, &verdict, chat.name());
                }
                session.log.push(Entry::Answered {
                    time: decision_log::now(),
                    path,
                    state,
                    privacy: (*privacy).into(),
                    questions_version: questions.version,
                    backend: chat.name().to_owned(),
                    model: model.clone(),
                    answers,
                    verdict,
                });
                session.arbiter_done += 1;
            }
            lock().arbiter_running = false;
        });
        (200, json!({ "ok": true }))
    }
}

fn header_value(request: &Request, name: &str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str().to_owned())
}

/// Review units still waiting for a decision, largest first.
fn pending_units(scan: &Scan) -> Vec<Signature> {
    let pending: Vec<&PlanNode> = scan
        .plan
        .nodes()
        .into_iter()
        .filter(|n| n.unit.is_some() && n.source == Source::Default)
        .collect();
    signatures(scan)
        .into_iter()
        .filter(|s| pending.iter().any(|n| n.path == s.path))
        .collect()
}

/// What the review screen shows.
fn review_model(scan: &Scan, library: &Library, text: &Localizer) -> Value {
    let plan = &scan.plan;
    let totals: BTreeMap<&str, Value> = plan
        .totals
        .iter()
        .map(|(d, t)| (d.as_str(), json!({ "bytes": t.bytes, "files": t.files })))
        .collect();

    let by_path: BTreeMap<String, Signature> = signatures(scan)
        .into_iter()
        .map(|s| (s.path.clone(), s))
        .collect();

    // To decide: review units nobody settled, and units an arbiter could
    // only give an opinion on. Ordered by size times uncertainty: a unit
    // with an opinion is half as uncertain as one without.
    let mut queue: Vec<(u128, Value)> = Vec::new();
    let mut settled = Vec::new();
    for node in plan.nodes() {
        let opinion = match &node.source {
            Source::Arbiter { opinion, .. } => *opinion,
            _ => None,
        };
        let waiting = node.decision == Decision::Review
            && node.unit.is_some()
            && matches!(node.source, Source::Default | Source::Arbiter { .. });
        if waiting {
            let weight = if opinion.is_some() { 1 } else { 2 };
            let card = card(node, by_path.get(&node.path), opinion, library, text);
            queue.push((u128::from(node.stats.bytes) * weight, card));
        } else if let Some(why) = crate::render::why(node, plan, &library.recipes, text) {
            settled.push(json!({
                "path": node.path,
                "decision": node.decision.as_str(),
                "bytes": node.stats.bytes,
                "files": node.stats.files,
                "by": source_kind(&node.source),
                "why": why,
            }));
        }
    }
    queue.sort_by_key(|a| std::cmp::Reverse(a.0));
    settled.sort_by_key(|n| std::cmp::Reverse(n["bytes"].as_u64().unwrap_or(0)));
    let applications: Vec<String> = plan
        .applications
        .iter()
        .map(|a| crate::render::application_name(&a.recipe, plan, &library.recipes, text))
        .collect();
    let unreadable: u64 = plan.nodes().iter().map(|n| n.stats.unreadable).sum();
    json!({
        "root": plan.root,
        "totals": totals,
        "queue": queue.into_iter().map(|(_, card)| card).collect::<Vec<_>>(),
        "settled": settled,
        "applications": applications,
        "unreadable": unreadable,
    })
}

/// One review card.
fn card(
    node: &PlanNode,
    signature: Option<&Signature>,
    opinion: Option<Decision>,
    library: &Library,
    text: &Localizer,
) -> Value {
    let stats = &node.stats;
    // What a rule recognised the folder as, and what it proposes.
    let (app, propose) = match &node.unit {
        Some(bellcart::plan::UnitReason::Rule { rule, propose, .. }) => (
            library
                .rules
                .iter()
                .find(|r| &r.id == rule)
                .and_then(|r| r.name.as_ref())
                .map(|n| n.get(text.lang()).to_owned()),
            *propose,
        ),
        _ => (None, None),
    };
    let mut extensions: Vec<(&String, &u64)> = stats.extensions.iter().collect();
    extensions.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    let extensions: Vec<Value> = extensions
        .into_iter()
        .take(6)
        .map(|(ext, bytes)| json!({ "ext": ext, "bytes": bytes }))
        .collect();
    let named = |list: &[bellcart::signature::Named]| -> Vec<Value> {
        list.iter()
            .take(8)
            .map(|n| json!({ "name": n.name, "bytes": n.bytes }))
            .collect()
    };
    let (folders, files, folder_count, file_count) = signature.map_or_else(
        || (Vec::new(), Vec::new(), 0, 0),
        |s| {
            (
                named(&s.sample.folders),
                named(&s.sample.files),
                s.sample.folder_count,
                s.sample.file_count,
            )
        },
    );
    let settled: Vec<Value> = node
        .overrides
        .iter()
        .filter(|o| o.decision != Decision::Review)
        .map(|o| json!({ "path": o.path, "decision": o.decision.as_str(), "bytes": o.stats.bytes }))
        .collect();
    json!({
        "path": node.path,
        "bytes": stats.bytes,
        "files": stats.files,
        "oldest": stats.oldest,
        "newest": stats.newest,
        "unreadable": stats.unreadable,
        "reason": node.unit.as_ref().map(|u| crate::render::unit_reason(u, text)),
        "extensions": extensions,
        "folders": folders,
        "folder_count": folder_count,
        "sample_files": files,
        "file_count": file_count,
        "settled": settled,
        "opinion": opinion.map(Decision::as_str),
        "app": app,
        "propose": propose.map(Decision::as_str),
    })
}

fn source_kind(source: &Source) -> &'static str {
    match source {
        Source::Rule { .. } => "rule",
        Source::Recipe { .. } => "recipe",
        Source::Arbiter { .. } => "arbiter",
        Source::User => "user",
        Source::Default => "default",
    }
}

fn phase_name(phase: &Phase) -> &'static str {
    match phase {
        Phase::Choose => "choose",
        Phase::Scanning => "scanning",
        Phase::Review => "review",
        Phase::Failed(_) => "failed",
    }
}

/// Writes the plan, the notebook and the log into `folder`.
fn write_exports(
    folder: &Path,
    scan: &Scan,
    log: &[Entry],
    library: &Library,
    text: &Localizer,
) -> Result<Vec<String>> {
    std::fs::create_dir_all(folder)
        .with_context(|| format!("cannot create {}", folder.display()))?;
    let mut files = Vec::new();
    let plan_file = folder.join("plan.json");
    std::fs::write(&plan_file, serde_json::to_string_pretty(&scan.plan)?)
        .with_context(|| format!("cannot write {}", plan_file.display()))?;
    files.push(plan_file.display().to_string());

    let book = bellcart::notebook::notebook(&scan.plan.applications, &library.recipes);
    for (name, format) in [
        ("notebook.html", crate::notebook::Format::Html),
        ("notebook.md", crate::notebook::Format::Markdown),
    ] {
        let file = folder.join(name);
        std::fs::write(
            &file,
            crate::notebook::render(&book, &scan.plan.root, text, format),
        )
        .with_context(|| format!("cannot write {}", file.display()))?;
        files.push(file.display().to_string());
    }

    if !log.is_empty() {
        let file = folder.join("bellcart-decisions.jsonl");
        for entry in log {
            decision_log::append(&file, entry)?;
        }
        files.push(file.display().to_string());
    }
    Ok(files)
}

/// A folder outside the profile to propose for the exports: next to where
/// Bellcart was started when that is outside the profile, else at the root
/// of the profile's drive.
fn default_export(profile: &Path) -> PathBuf {
    let here = std::env::current_dir().unwrap_or_default();
    let probe = here.join("bellcart-export");
    if crate::outside(profile, &probe).is_ok() {
        return probe;
    }
    let root = profile
        .ancestors()
        .last()
        .map_or_else(|| PathBuf::from("/"), Path::to_path_buf);
    root.join("bellcart-export")
}

/// Adds headers. They are fixed strings in this file, so each one builds;
/// one that did not would be skipped rather than fail the response.
fn with_headers<R: Read>(mut response: Response<R>, headers: &[(&str, &str)]) -> Response<R> {
    for (name, value) in headers {
        if let Ok(header) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response.add_header(header);
        }
    }
    response
}

/// Headers that keep the page to itself: nothing loaded from elsewhere,
/// no framing, no caching of personal data, no referrer.
fn secured<R: Read>(response: Response<R>) -> Response<R> {
    with_headers(
        response,
        &[
            (
                "Content-Security-Policy",
                "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; \
                 img-src 'self' data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
            ),
            ("X-Content-Type-Options", "nosniff"),
            ("X-Frame-Options", "DENY"),
            ("Referrer-Policy", "no-referrer"),
            ("Cache-Control", "no-store"),
        ],
    )
}

/// 128 random bits as hex. The standard library's hasher is keyed with
/// randomness from the operating system, so no dependency is needed.
fn token() -> String {
    let mut out = String::new();
    for round in 0..2u8 {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u8(round);
        hasher.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos()),
        );
        out.push_str(&format!("{:016x}", hasher.finish()));
    }
    out
}

/// Opens the URL in the default browser, without a shell. If it fails, the
/// URL printed in the terminal is still there to click.
fn open_in_browser(url: &str) {
    let mut command = if cfg!(windows) {
        let mut c = std::process::Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url]);
        c
    } else if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    } else {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    let _ = command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}
