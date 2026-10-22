//! The decisions of a review session, saved as they are made so that the
//! next `bellcart ui` can resume (D33).
//!
//! The file lives in the proposed export folder, outside the scanned
//! profile: Bellcart never writes there. It holds the decisions only, not
//! the scan: scanning again takes about a second, and replaying the
//! decisions on a fresh scan keeps the plan true to the disk.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use bellcart::plan::{Decision, Plan};
use serde::{Deserialize, Serialize};

pub const FILE: &str = "bellcart-session.json";
const FORMAT: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    pub format: u32,
    /// The profile the decisions were made on, as the user gave it.
    pub profile: String,
    /// In the order they were made: a later one may change an earlier one.
    pub decisions: Vec<Choice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    pub path: String,
    pub decision: Decision,
    /// Taken from a parent in the card's path: holds for all it contains.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub below: bool,
}

impl Saved {
    pub fn new(profile: &Path) -> Self {
        Self {
            format: FORMAT,
            profile: profile.display().to_string(),
            decisions: Vec::new(),
        }
    }

    /// The decisions saved for `profile` in `file`, if any. A file that
    /// cannot be read, or that is about another profile, gives none.
    pub fn load(file: &Path, profile: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(file).ok()?;
        let saved: Saved = serde_json::from_str(&text).ok()?;
        (saved.format == FORMAT && same_folder(Path::new(&saved.profile), profile)).then_some(saved)
    }

    pub fn write(&self, file: &Path) -> Result<()> {
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)
                .with_context(|| format!("cannot create {}", folder.display()))?;
        }
        std::fs::write(file, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("cannot write {}", file.display()))
    }
}

/// Applies one choice to the plan. Returns the paths changed: none when
/// the folder is no longer there.
pub fn apply(plan: &mut Plan, choice: &Choice) -> Vec<String> {
    if choice.below {
        plan.apply_user_below(&choice.path, choice.decision)
    } else if plan.apply_user(&choice.path, choice.decision) {
        vec![choice.path.clone()]
    } else {
        Vec::new()
    }
}

/// Where the session of `profile` is saved: in `folder`, provided that is
/// outside the profile. `None` when it is not, and nothing is saved.
pub fn location(profile: &Path, folder: &Path) -> Option<PathBuf> {
    let file = folder.join(FILE);
    crate::outside(profile, &file).is_ok().then_some(file)
}

fn same_folder(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}
