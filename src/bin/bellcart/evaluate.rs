//! `bellcart label` builds a labelled set by hand; `bellcart eval` replays
//! it against an arbiter.

use std::collections::BTreeSet;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Result, bail};
use bellcart::eval::{Labelled, append_label, evaluate, read_labels};
use bellcart::openai_chat::{ChatConfig, OpenAiChat};
use bellcart::pack::{Library, QuestionSet, RuleSet};
use bellcart::plan::{Decision, SplitBudget};
use bellcart::signature::{Privacy, signatures};

use crate::i18n::Localizer;

/// Shows each folder to review, largest first, and records what the user
/// says it is. Folders already in `out` are skipped, so labelling can stop
/// and resume.
pub fn label(profile: &Path, out: &Path, library: &Library, text: &Localizer) -> Result<()> {
    crate::outside(profile, out)?;
    let done: BTreeSet<String> = if out.exists() {
        read_labels(out)?.into_iter().map(|l| l.path).collect()
    } else {
        BTreeSet::new()
    };
    let rules = RuleSet::compile(&library.rules)?;
    let scan = crate::progress::scan(
        profile,
        &rules,
        &library.recipes,
        &SplitBudget::default(),
        None,
        text,
    )?;
    let todo: Vec<_> = signatures(&scan)
        .into_iter()
        .filter(|s| !done.contains(&s.path))
        .collect();
    if !done.is_empty() {
        say(&format!(
            "{}\n",
            text.with("label-already", &[("count", done.len().into())])
        ))?;
    }
    let mut answers = std::io::stdin().lock();
    let mut written = 0;
    for signature in todo {
        let full = signature.to_text();
        say(&format!("\n{full}{} ", text.get("label-prompt")))?;
        let mut line = String::new();
        // Nothing read at all: the input is over.
        if answers.read_line(&mut line)? == 0 {
            break;
        }
        let label = match line.trim().to_lowercase().as_str() {
            "m" => Decision::Migrate,
            "a" => Decision::Archive,
            "l" => Decision::Leave,
            "q" => break,
            // `s`, an empty line or anything else: skip this folder.
            _ => continue,
        };
        append_label(
            out,
            &Labelled {
                path: signature.path.clone(),
                redacted: signature.for_privacy(Privacy::Redacted).to_text(),
                full,
                label,
            },
        )?;
        written += 1;
    }
    say(&format!(
        "\n{}\n",
        text.with(
            "label-done",
            &[
                ("count", written.into()),
                ("file", out.display().to_string().into())
            ]
        )
    ))?;
    Ok(())
}

pub struct EvalOptions {
    pub labels: PathBuf,
    pub endpoint: String,
    pub model: String,
    pub privacy: Privacy,
    pub timeout: Duration,
}

/// Replays the labelled set. Fails when a folder to keep would be left
/// behind, so it can guard a change of backend in a script.
pub fn eval(options: &EvalOptions, questions: &QuestionSet, text: &Localizer) -> Result<()> {
    let labels = read_labels(&options.labels)?;
    let chat = OpenAiChat::new(ChatConfig {
        endpoint: options.endpoint.clone(),
        model: options.model.clone(),
        key: std::env::var(crate::arbitrate::KEY_VARIABLE)
            .ok()
            .filter(|k| !k.is_empty()),
        timeout: options.timeout,
    })?;
    say(&format!(
        "{} ",
        text.with(
            "eval-confirm",
            &[("count", labels.len().into()), ("url", chat.url().into())]
        )
    ))?;
    if !crate::ask::confirmed(&mut std::io::stdin().lock())? {
        say(&format!("{}\n", text.get("arbitrate-nothing-sent")))?;
        return Ok(());
    }

    let report = evaluate(&chat, &questions.questions, &labels, options.privacy);
    let mut out = format!(
        "\n{}\n",
        text.with(
            "eval-title",
            &[
                (
                    "backend",
                    format!("{} ({})", report.backend, options.model).into()
                ),
                ("count", report.total().into()),
                ("agreed", report.agreed().into()),
            ]
        )
    );
    for (label, tally) in &report.per_label {
        out.push_str(&format!(
            "  {}\n",
            text.with(
                "eval-label",
                &[
                    (
                        "label",
                        text.get(&format!("decision-{}", label.as_str())).into()
                    ),
                    ("agreed", tally.agreed.into()),
                    ("total", tally.total.into()),
                ]
            )
        ));
    }
    let list = |paths: &[String]| paths.join(", ");
    if report.lost.is_empty() {
        out.push_str(&format!("{}\n", text.get("eval-lost-none")));
    } else {
        out.push_str(&format!(
            "{}\n",
            text.with("eval-lost", &[("paths", list(&report.lost).into())])
        ));
    }
    if !report.leave_suggested_on_migrate.is_empty() {
        out.push_str(&format!(
            "{}\n",
            text.with(
                "eval-leave-suggested",
                &[("paths", list(&report.leave_suggested_on_migrate).into())]
            )
        ));
    }
    if !report.failed.is_empty() {
        out.push_str(&format!(
            "{}\n",
            text.with("eval-failed", &[("paths", list(&report.failed).into())])
        ));
    }
    say(&out)?;
    if !report.lost.is_empty() {
        bail!(
            "{} folder(s) to keep would be left behind",
            report.lost.len()
        );
    }
    Ok(())
}

fn say(text: &str) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()?;
    Ok(())
}
