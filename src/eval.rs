//! Replays a labelled set against an arbiter, to compare backends on data
//! rather than impressions.
//!
//! The figure that matters most is not agreement: it is how often a folder
//! labelled `migrate` or `archive` would end up left behind. That must be
//! zero. A suggested `leave` on such a folder is counted apart: it is held
//! for the user, but shows how far the backend can be trusted.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::arbiter::{Arbiter, Question};
use crate::plan::Decision;
use crate::signature::Privacy;
use crate::verdict::combine;
use crate::{Error, Result};

/// One labelled folder: its signature in both forms, and what a person
/// decided for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Labelled {
    pub path: String,
    pub full: String,
    pub redacted: String,
    pub label: Decision,
}

impl Labelled {
    pub fn state(&self, privacy: Privacy) -> &str {
        match privacy {
            Privacy::Full => &self.full,
            Privacy::Redacted => &self.redacted,
        }
    }
}

/// Appends one label to a JSON Lines file.
pub fn append_label(file: &Path, labelled: &Labelled) -> Result<()> {
    let io = |source| Error::Io {
        path: file.to_path_buf(),
        source,
    };
    let mut line = serde_json::to_string(labelled)
        .map_err(|e| Error::Arbiter(format!("cannot write a label: {e}")))?;
    line.push('\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .map_err(io)?
        .write_all(line.as_bytes())
        .map_err(io)
}

/// Reads a labelled set. Unlike the decision log, a bad line is an error:
/// a silently smaller set would give a misleading report.
pub fn read_labels(file: &Path) -> Result<Vec<Labelled>> {
    let io = |source| Error::Io {
        path: file.to_path_buf(),
        source,
    };
    let reader = std::io::BufReader::new(std::fs::File::open(file).map_err(io)?);
    let mut out = Vec::new();
    for (number, line) in reader.lines().enumerate() {
        let line = line.map_err(io)?;
        if line.trim().is_empty() {
            continue;
        }
        let labelled = serde_json::from_str(&line).map_err(|e| Error::PackFile {
            file: file.display().to_string(),
            message: format!("line {}: {e}", number + 1),
        })?;
        out.push(labelled);
    }
    Ok(out)
}

/// Agreement for one label.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Tally {
    pub total: usize,
    /// The backend's suggestion matched the label.
    pub agreed: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Report {
    pub backend: String,
    pub per_label: BTreeMap<Decision, Tally>,
    /// Folders labelled `migrate` or `archive` whose verdict applied
    /// `leave`. Must be empty.
    pub lost: Vec<String>,
    /// Folders labelled `migrate` for which `leave` was suggested and
    /// held for the user.
    pub leave_suggested_on_migrate: Vec<String>,
    /// Folders the backend gave no usable answer for.
    pub failed: Vec<String>,
}

impl Report {
    pub fn total(&self) -> usize {
        self.per_label.values().map(|t| t.total).sum()
    }

    pub fn agreed(&self) -> usize {
        self.per_label.values().map(|t| t.agreed).sum()
    }
}

/// Puts every labelled folder to `arbiter` and compares.
///
/// The suggestion is the verdict's opinion, or `review` when the backend
/// gave none: a held `leave` counts as `leave`, since that is what the
/// backend thought.
pub fn evaluate(
    arbiter: &dyn Arbiter,
    questions: &BTreeMap<String, Question>,
    labels: &[Labelled],
    privacy: Privacy,
) -> Report {
    let mut report = Report {
        backend: arbiter.name().to_owned(),
        ..Report::default()
    };
    for labelled in labels {
        let tally = report.per_label.entry(labelled.label).or_default();
        tally.total += 1;
        let Ok(answers) = arbiter.ask(labelled.state(privacy), questions) else {
            report.failed.push(labelled.path.clone());
            continue;
        };
        let verdict = combine(&answers, arbiter.calibrated());
        let suggested = verdict.opinion.unwrap_or(Decision::Review);
        if suggested == labelled.label {
            tally.agreed += 1;
        }
        let kept = matches!(labelled.label, Decision::Migrate | Decision::Archive);
        if kept && verdict.decision == Decision::Leave {
            report.lost.push(labelled.path.clone());
        }
        if labelled.label == Decision::Migrate && suggested == Decision::Leave {
            report
                .leave_suggested_on_migrate
                .push(labelled.path.clone());
        }
    }
    report
}
