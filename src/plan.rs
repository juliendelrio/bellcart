//! The migration plan: the only thing Bellcart produces in V1.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Where a folder goes.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    /// Needed on the new PC.
    Migrate,
    /// Worth keeping, not needed day to day: goes to cold storage.
    Archive,
    /// Regenerable or obsolete: stays behind.
    Leave,
    /// Nothing settled it yet: an arbiter or the user must decide.
    Review,
}

impl Decision {
    pub const ALL: [Decision; 4] = [
        Decision::Migrate,
        Decision::Archive,
        Decision::Leave,
        Decision::Review,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Decision::Migrate => "migrate",
            Decision::Archive => "archive",
            Decision::Leave => "leave",
            Decision::Review => "review",
        }
    }
}

/// Who took a decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "by", rename_all = "lowercase")]
pub enum Source {
    /// A rule from a pack. `reason` is a translation key (`reason-<key>`).
    Rule { rule: String, reason: String },
    /// Inherited default: no rule matched.
    Default,
}

/// Size figures for the files a node covers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Stats {
    pub bytes: u64,
    pub files: u64,
    /// Oldest and newest modification, seconds since the Unix epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newest: Option<u64>,
    /// Bytes per lowercase extension (`""` for files without one).
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub extensions: BTreeMap<String, u64>,
    /// Entries that could not be read (access denied, for instance).
    #[serde(skip_serializing_if = "is_zero", default)]
    pub unreadable: u64,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

impl Stats {
    pub fn add_file(&mut self, name: &str, size: u64, modified: Option<u64>) {
        self.bytes += size;
        self.files += 1;
        if let Some(m) = modified {
            self.oldest = Some(self.oldest.map_or(m, |o| o.min(m)));
            self.newest = Some(self.newest.map_or(m, |n| n.max(m)));
        }
        *self.extensions.entry(extension_of(name)).or_default() += size;
    }

    pub fn merge(&mut self, other: &Stats) {
        self.bytes += other.bytes;
        self.files += other.files;
        self.unreadable += other.unreadable;
        self.oldest = min_opt(self.oldest, other.oldest);
        self.newest = max_opt(self.newest, other.newest);
        for (ext, bytes) in &other.extensions {
            *self.extensions.entry(ext.clone()).or_default() += bytes;
        }
    }
}

fn extension_of(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ext.to_lowercase(),
        _ => String::new(),
    }
}

fn min_opt(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

fn max_opt(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}

/// A folder and the decision that applies to everything under it, except
/// the sub-folders listed in `overrides`.
///
/// This mirrors how copy tools work: "take this folder, but not those".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PlanNode {
    /// Path relative to the scan root, with `/` separators. Empty for the root.
    pub path: String,
    pub decision: Decision,
    pub source: Source,
    /// Files covered by this node, excluding `overrides`.
    pub stats: Stats,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub overrides: Vec<PlanNode>,
    /// Set on `review` nodes that are one unit of decision: why the cut of
    /// the review mass stopped here. `stats` are then what there is to judge.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub unit: Option<UnitReason>,
}

/// Why a review folder is judged as one unit instead of being cut further.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "reason", rename_all = "kebab-case")]
pub enum UnitReason {
    /// Small enough to judge as a whole.
    Small,
    /// Mostly files of one extension, and the rest is small.
    Homogeneous {
        extension: String,
        /// Share of the bytes, rounded down.
        percent: u8,
    },
    /// Reached the depth limit of the cut.
    DepthLimit,
    /// No sub-folder big enough to be judged on its own.
    Leaf,
    /// What is left directly in a folder that was cut: its own files and
    /// its sub-folders too small to stand alone.
    Rest,
}

/// How far the review mass is cut. Sizes are in bytes, depth counts
/// folders below the scan root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SplitBudget {
    /// A folder at most this big is one unit.
    pub split_above: u64,
    /// A sub-folder smaller than this stays in its parent's rest.
    pub min_unit: u64,
    /// A folder where one extension holds at least this share of the bytes
    /// is one unit, provided the other bytes are at most `split_above`.
    pub homogeneous_percent: u8,
    /// Folders at this depth are not cut further.
    pub max_depth: u32,
}

impl Default for SplitBudget {
    /// Provisional values, to tune on real profiles.
    fn default() -> Self {
        Self {
            split_above: 1 << 30,
            min_unit: 100 << 20,
            homogeneous_percent: 90,
            max_depth: 8,
        }
    }
}

/// The plan for one scan root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Plan {
    /// Format version of this document.
    pub version: u32,
    /// The scanned root, as given by the user.
    pub root: String,
    pub totals: BTreeMap<Decision, Totals>,
    pub tree: PlanNode,
    /// The budget the review mass was cut with, so a cut can be reproduced.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub split: Option<SplitBudget>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Totals {
    pub bytes: u64,
    pub files: u64,
}

/// 2: review nodes may nest inside review nodes, one per unit of decision
/// (`PlanNode::unit`), and the plan records its `split` budget.
pub const PLAN_VERSION: u32 = 2;

impl Plan {
    pub fn new(root: String, tree: PlanNode, split: Option<SplitBudget>) -> Self {
        let mut totals: BTreeMap<Decision, Totals> = Decision::ALL
            .into_iter()
            .map(|d| (d, Totals::default()))
            .collect();
        accumulate(&tree, &mut totals);
        Self {
            version: PLAN_VERSION,
            root,
            totals,
            tree,
            split,
        }
    }

    /// Every node of the plan, depth first.
    pub fn nodes(&self) -> Vec<&PlanNode> {
        let mut out = Vec::new();
        collect(&self.tree, &mut out);
        out
    }
}

fn accumulate(node: &PlanNode, totals: &mut BTreeMap<Decision, Totals>) {
    let entry = totals.entry(node.decision).or_default();
    entry.bytes += node.stats.bytes;
    entry.files += node.stats.files;
    for child in &node.overrides {
        accumulate(child, totals);
    }
}

fn collect<'a>(node: &'a PlanNode, out: &mut Vec<&'a PlanNode>) {
    out.push(node);
    for child in &node.overrides {
        collect(child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_is_lowercased_and_dotfiles_have_none() {
        assert_eq!(extension_of("Photo.JPG"), "jpg");
        assert_eq!(extension_of("archive.tar.gz"), "gz");
        assert_eq!(extension_of(".gitconfig"), "");
        assert_eq!(extension_of("README"), "");
    }
}
