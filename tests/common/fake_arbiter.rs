//! A local HTTP server that plays an OpenAI-compatible endpoint, so the
//! arbiter backend is tested without any network.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// What the fake endpoint received.
#[derive(Debug, Clone)]
pub struct Received {
    pub url: String,
    pub authorization: Option<String>,
    pub body: serde_json::Value,
}

/// How the fake endpoint answers.
#[derive(Debug, Clone)]
pub enum Reply {
    /// A chat completion whose message content is this JSON.
    Answers(serde_json::Value),
    /// A raw status and body.
    Raw(u16, String),
    /// A redirect to another URL.
    Redirect(String),
    /// Wait this long before answering.
    Slow(Duration),
}

pub struct FakeArbiter {
    pub base: String,
    pub received: mpsc::Receiver<Received>,
}

/// Starts a server on a free local port that gives `replies` in turn, then
/// stops.
pub fn start(replies: Vec<Reply>) -> FakeArbiter {
    let server = tiny_http::Server::http("127.0.0.1:0").expect("a local port");
    let port = server.server_addr().to_ip().expect("an IP address").port();
    let (sender, received) = mpsc::channel();
    thread::spawn(move || {
        for reply in replies {
            let Ok(mut request) = server.recv() else {
                return;
            };
            let mut body = String::new();
            let _ = request.as_reader().read_to_string(&mut body);
            let authorization = request
                .headers()
                .iter()
                .find(|h| h.field.equiv("Authorization"))
                .map(|h| h.value.as_str().to_owned());
            let _ = sender.send(Received {
                url: request.url().to_owned(),
                authorization,
                body: serde_json::from_str(&body).unwrap_or(serde_json::Value::Null),
            });
            let json = tiny_http::Header::from_bytes("Content-Type", "application/json")
                .expect("a header");
            let response = match reply {
                Reply::Answers(content) => {
                    let completion = serde_json::json!({
                        "choices": [{"message": {"role": "assistant", "content": content.to_string()}}]
                    });
                    tiny_http::Response::from_string(completion.to_string()).with_header(json)
                }
                Reply::Raw(status, text) => {
                    tiny_http::Response::from_string(text).with_status_code(status)
                }
                Reply::Redirect(to) => tiny_http::Response::from_string("")
                    .with_status_code(302)
                    .with_header(tiny_http::Header::from_bytes("Location", to).expect("a header")),
                Reply::Slow(wait) => {
                    thread::sleep(wait);
                    tiny_http::Response::from_string("{}").with_header(json)
                }
            };
            let _ = request.respond(response);
        }
    });
    FakeArbiter {
        base: format!("http://127.0.0.1:{port}/v1"),
        received,
    }
}
