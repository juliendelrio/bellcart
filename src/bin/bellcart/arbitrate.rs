//! `bellcart arbitrate`: puts the review units to an arbiter, after showing
//! what will be sent and getting a yes.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use bellcart::arbiter::Arbiter;
use bellcart::decision_log::{self, Entry};
use bellcart::openai_chat::{ChatConfig, OpenAiChat};
use bellcart::pack::{Library, QuestionSet, RuleSet};
use bellcart::plan::{Decision, SplitBudget, human_bytes};
use bellcart::signature::{Privacy, signatures};
use bellcart::verdict::combine;

use crate::i18n::Localizer;

/// The environment variable that holds the API key. A key is never taken
/// from the command line, where it would stay in the shell history.
pub const KEY_VARIABLE: &str = "BELLCART_ARBITER_KEY";
/// After this many failures in a row, the endpoint is taken to be down.
const MAX_FAILURES_IN_A_ROW: usize = 3;

pub struct Options {
    pub profile: PathBuf,
    pub endpoint: String,
    pub model: String,
    pub privacy: Privacy,
    pub out: Option<PathBuf>,
    pub log: PathBuf,
    pub limit: Option<usize>,
    pub preview: bool,
    pub timeout: Duration,
}

pub fn run(
    options: &Options,
    library: &Library,
    questions: &QuestionSet,
    text: &Localizer,
) -> Result<()> {
    for file in options.out.iter().chain([&options.log]) {
        crate::outside(&options.profile, file)?;
    }
    let chat = OpenAiChat::new(ChatConfig {
        endpoint: options.endpoint.clone(),
        model: options.model.clone(),
        key: std::env::var(KEY_VARIABLE).ok().filter(|k| !k.is_empty()),
        timeout: options.timeout,
    })?;

    let rules = RuleSet::compile(&library.rules)?;
    let mut scan = crate::progress::scan(
        &options.profile,
        &rules,
        &library.recipes,
        &SplitBudget::default(),
        None,
        text,
    )?;
    let mut units = signatures(&scan);
    units.truncate(options.limit.unwrap_or(usize::MAX));
    if units.is_empty() {
        say(&format!("{}\n", text.get("arbitrate-none")))?;
        return Ok(());
    }
    let states: Vec<(String, String)> = units
        .iter()
        .map(|s| (s.path.clone(), s.for_privacy(options.privacy).to_text()))
        .collect();

    if options.preview {
        for (_, state) in &states {
            say(&format!("{state}\n"))?;
        }
        return Ok(());
    }

    // Exactly what goes out, before anything does.
    let first = chat.body(&states[0].1, &questions.questions);
    say(&format!(
        "{}\n{}\n\n{} ",
        text.get("arbitrate-preview"),
        serde_json::to_string_pretty(&first)?,
        text.with(
            "arbitrate-confirm",
            &[("count", states.len().into()), ("url", chat.url().into()),],
        )
    ))?;
    if !crate::ask::confirmed(&mut std::io::stdin().lock())? {
        say(&format!("{}\n", text.get("arbitrate-nothing-sent")))?;
        return Ok(());
    }

    let mut failures = 0;
    for (path, state) in &states {
        let answers = match chat.ask(state, &questions.questions) {
            Ok(answers) => {
                failures = 0;
                answers
            }
            Err(error) => {
                failures += 1;
                say(&format!(
                    "{}\n",
                    text.with(
                        "arbitrate-failed",
                        &[
                            ("path", shown(path).into()),
                            ("error", error.to_string().into())
                        ]
                    )
                ))?;
                if failures >= MAX_FAILURES_IN_A_ROW {
                    return Err(anyhow!(
                        text.with("arbitrate-stopped", &[("count", failures.into())])
                    ));
                }
                continue;
            }
        };
        let verdict = combine(&answers, chat.calibrated());
        scan.plan.apply_verdict(path, &verdict, chat.name());
        decision_log::append(
            &options.log,
            &Entry::Answered {
                time: decision_log::now(),
                path: path.clone(),
                state: state.clone(),
                privacy: options.privacy.into(),
                questions_version: questions.version,
                backend: chat.name().to_owned(),
                model: options.model.clone(),
                answers,
                verdict: verdict.clone(),
            },
        )?;
        let bytes = scan
            .plan
            .nodes()
            .into_iter()
            .find(|n| &n.path == path)
            .map_or(0, |n| n.stats.bytes);
        let mut line = format!(
            "  {:>10}  {}  {}",
            human_bytes(bytes),
            shown(path),
            text.get(&format!("decision-{}", verdict.decision.as_str()))
        );
        if verdict.decision == Decision::Review && verdict.opinion == Some(Decision::Leave) {
            line.push_str(&format!("  ({})", text.get("arbitrate-leave-to-confirm")));
        }
        say(&format!("{line}\n"))?;
    }

    if let Some(out) = &options.out {
        std::fs::write(out, serde_json::to_string_pretty(&scan.plan)?)
            .with_context(|| format!("cannot write {}", out.display()))?;
    }
    say(&format!(
        "\n{}",
        crate::render::plan(&scan.plan, &library.recipes, text)
    ))?;
    Ok(())
}

fn shown(path: &str) -> &str {
    if path.is_empty() { "." } else { path }
}

fn say(text: &str) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

/// Where the log goes when no `--log` is given.
pub fn default_log() -> PathBuf {
    Path::new("bellcart-decisions.jsonl").to_path_buf()
}
