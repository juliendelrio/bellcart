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
    /// A `copy` or `skip` step in the recipe of an application found on the
    /// profile. `recipe` is its id, listed in [`Plan::applications`].
    Recipe { recipe: String },
    /// An arbiter's answers about a review unit, combined under the
    /// prudence rules. `opinion` is what it suggested: on a `review` node,
    /// a `leave` waiting for the user to confirm.
    Arbiter {
        backend: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        opinion: Option<Decision>,
    },
    /// The user decided, in the review interface. It overrides whatever
    /// decided before.
    User,
    /// Inherited default: no rule matched.
    Default,
}

/// An application whose recipe detected it on the profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Application {
    /// The recipe id.
    pub recipe: String,
    pub name: String,
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

/// A size for people and for signatures: `1.5 KB`, `42.0 GB`.
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// The lowercase extension of a file name, `""` when it has none.
///
/// A suffix of more than three digits is not an extension: it is a counter,
/// a process number or a year (`app.exe.old.33452`, `report.2024`), and
/// each value would otherwise count as an extension of its own. Three
/// digits stay, for the parts of split archives (`backup.7z.001`).
pub(crate) fn extension_of(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => {
            if ext.len() > 3 && ext.bytes().all(|b| b.is_ascii_digit()) {
                String::new()
            } else {
                ext.to_lowercase()
            }
        }
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
/// the sub-folders listed in `overrides`. A node settled by a recipe may
/// also be a single file (`.gitconfig`).
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
    /// A `review` rule recognised the folder (a synced folder, say): it is
    /// judged whole, whatever its size, and never folded into another unit.
    Rule {
        rule: String,
        /// Translation key of the reason, as in `Source::Rule`.
        key: String,
    },
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
    /// Applications found on the profile, by recipe id.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub applications: Vec<Application>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Totals {
    pub bytes: u64,
    pub files: u64,
}

/// 2: review nodes may nest inside review nodes, one per unit of decision
/// (`PlanNode::unit`), and the plan records its `split` budget.
/// 3: a node may be settled by a recipe (`Source::Recipe`) and may be a
/// single file; the plan lists the `applications` found.
/// 4: a review unit may be decided by an arbiter (`Source::Arbiter`).
/// 5: any node may be decided by the user (`Source::User`).
/// 6: a unit may be one because a rule recognised it (`UnitReason::Rule`).
pub const PLAN_VERSION: u32 = 6;

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
            applications: Vec::new(),
        }
    }

    /// Applies an arbiter's verdict to the review unit at `path`. Returns
    /// `false`, changing nothing, when there is no such unit or when it was
    /// already decided: a verdict never overrides a rule, a recipe or the
    /// user.
    pub fn apply_verdict(
        &mut self,
        path: &str,
        verdict: &crate::verdict::Verdict,
        backend: &str,
    ) -> bool {
        let Some(node) = find_mut(&mut self.tree, path) else {
            return false;
        };
        if node.unit.is_none() || node.source != Source::Default {
            return false;
        }
        node.decision = verdict.decision;
        node.source = Source::Arbiter {
            backend: backend.to_owned(),
            opinion: verdict.opinion,
        };
        self.retotal();
        true
    }

    /// Records the user's decision for the node at `path`: a review unit,
    /// or a folder a rule, a recipe or an arbiter settled. Returns `false`
    /// when the plan has no node there. The user may decide anything,
    /// `leave` included: it is their data.
    pub fn apply_user(&mut self, path: &str, decision: Decision) -> bool {
        let Some(node) = find_mut(&mut self.tree, path) else {
            return false;
        };
        node.decision = decision;
        node.source = Source::User;
        self.retotal();
        true
    }

    /// Records the user's decision for the folder at `path` and everything
    /// under it, whether or not the plan has a node there: every node at
    /// or below `path` takes `decision`, except those a rule or a recipe
    /// settled, which keep theirs ("take this folder, but not those").
    /// Returns the paths changed, empty when there was nothing to change.
    pub fn apply_user_below(&mut self, path: &str, decision: Decision) -> Vec<String> {
        let mut changed = Vec::new();
        below_mut(&mut self.tree, path, &mut |node| {
            if !matches!(node.source, Source::Rule { .. } | Source::Recipe { .. }) {
                node.decision = decision;
                node.source = Source::User;
                changed.push(node.path.clone());
            }
        });
        if !changed.is_empty() {
            self.retotal();
        }
        changed
    }

    /// Recomputes `totals` from the tree.
    pub fn retotal(&mut self) {
        let mut totals: BTreeMap<Decision, Totals> = Decision::ALL
            .into_iter()
            .map(|d| (d, Totals::default()))
            .collect();
        accumulate(&self.tree, &mut totals);
        self.totals = totals;
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

fn find_mut<'a>(node: &'a mut PlanNode, path: &str) -> Option<&'a mut PlanNode> {
    if node.path == path {
        return Some(node);
    }
    node.overrides
        .iter_mut()
        .find_map(|child| find_mut(child, path))
}

/// Whether `path` is `folder` or lies under it.
pub fn is_within(path: &str, folder: &str) -> bool {
    folder.is_empty()
        || path == folder
        || path
            .strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Calls `f` on every node at or under `folder`. Nodes are not nested by
/// path alone (a cut moves units up), so the whole tree is visited.
fn below_mut(node: &mut PlanNode, folder: &str, f: &mut impl FnMut(&mut PlanNode)) {
    if is_within(&node.path, folder) {
        f(node);
    }
    for child in &mut node.overrides {
        below_mut(child, folder, f);
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

    #[test]
    fn bytes_are_readable() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1023), "1023 B");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes(42 * 1024 * 1024 * 1024), "42.0 GB");
    }
}
