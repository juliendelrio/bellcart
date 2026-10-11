//! The local decision log (SPEC-V1 section 8): every arbiter answer, and
//! every correction the user makes, one JSON object per line.
//!
//! It stays on the machine unless the user exports it. It is the material
//! for comparing backends (`bellcart eval`) and, later, for training a
//! small model. Lines are only ever appended, so a log can be read while
//! it grows and survives an interrupted run.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::arbiter::Answer;
use crate::plan::Decision;
use crate::signature::Privacy;
use crate::verdict::Verdict;
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "kebab-case")]
pub enum Entry {
    /// An arbiter answered about a review unit.
    Answered {
        /// Seconds since the Unix epoch.
        time: u64,
        path: String,
        /// The signature exactly as sent.
        state: String,
        privacy: LoggedPrivacy,
        questions_version: u32,
        backend: String,
        model: String,
        answers: BTreeMap<String, Answer>,
        verdict: Verdict,
    },
    /// The user decided a unit, confirming or correcting the arbiter.
    Corrected {
        time: u64,
        path: String,
        decision: Decision,
    },
}

/// [`Privacy`] as written in the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoggedPrivacy {
    Full,
    Redacted,
}

impl From<Privacy> for LoggedPrivacy {
    fn from(privacy: Privacy) -> Self {
        match privacy {
            Privacy::Full => LoggedPrivacy::Full,
            Privacy::Redacted => LoggedPrivacy::Redacted,
        }
    }
}

/// Seconds since the Unix epoch, for entries.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Adds one entry at the end of the log, creating it if needed.
pub fn append(file: &Path, entry: &Entry) -> Result<()> {
    let io = |source| Error::Io {
        path: file.to_path_buf(),
        source,
    };
    let mut line = serde_json::to_string(entry)
        .map_err(|e| Error::Arbiter(format!("cannot write a log entry: {e}")))?;
    line.push('\n');
    let mut out = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .map_err(io)?;
    out.write_all(line.as_bytes()).map_err(io)
}

/// Reads a log. A line that does not parse, such as one cut short by a
/// crash, is skipped rather than making the whole log unreadable.
pub fn read(file: &Path) -> Result<Vec<Entry>> {
    let io = |source| Error::Io {
        path: file.to_path_buf(),
        source,
    };
    let reader = std::io::BufReader::new(std::fs::File::open(file).map_err(io)?);
    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(io)?;
        if let Ok(entry) = serde_json::from_str(&line) {
            entries.push(entry);
        }
    }
    Ok(entries)
}
