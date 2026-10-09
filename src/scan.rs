//! Turns a tree and a rule set into a plan.
//!
//! The walk is top-down. A folder matched by a rule is settled together
//! with everything under it: we only measure it, we do not look for more
//! rules inside. A folder no rule matches stays `review`.
//!
//! The review mass is then cut into units of decision, bottom-up: once a
//! review folder has been walked, we know its size and extension mix
//! (settled sub-folders excluded) and decide whether it is judged as one
//! unit or cut into its sub-folders. The criteria are in [`SplitBudget`].
//! A folder's cut depends only on what is under it, so it does not change
//! when unrelated folders grow.
//!
//! The plan lists the places where the decision changes, plus one `review`
//! node per unit. Cutting never decides anything: it only creates `review`
//! nodes, and leaves the totals unchanged.

use std::path::Path;

use crate::pack::RuleSet;
use crate::plan::{Decision, Plan, PlanNode, Source, SplitBudget, Stats, UnitReason};
use crate::tree::{Entry, Kind, Tree};
use crate::{Error, Result};

/// Scans `root` and builds its plan, cutting the review mass with `budget`.
///
/// Fails only if `root` itself cannot be read. Unreadable folders below it
/// are counted in [`Stats::unreadable`] and the scan goes on.
pub fn scan(tree: &dyn Tree, root: &Path, rules: &RuleSet, budget: &SplitBudget) -> Result<Plan> {
    let entries = tree.read_dir(root).map_err(|source| Error::Io {
        path: root.to_path_buf(),
        source,
    })?;
    let walker = Walker {
        tree,
        rules,
        budget,
    };
    let walked = walker.walk(root, String::new(), 0, entries);
    Ok(Plan::new(
        root.to_string_lossy().into_owned(),
        walked.node,
        Some(budget.clone()),
    ))
}

struct Walker<'a> {
    tree: &'a dyn Tree,
    rules: &'a RuleSet,
    budget: &'a SplitBudget,
}

/// A review folder once walked: its node, already cut, and the figures of
/// everything still to review under it.
struct Walked {
    node: PlanNode,
    review: Stats,
}

/// A sub-folder of a review folder, in listing order.
enum Item {
    Settled(PlanNode),
    Review(Walked),
}

impl Walker<'_> {
    fn walk(&self, abs: &Path, rel: String, depth: u32, entries: Vec<Entry>) -> Walked {
        let mut own = Stats::default();
        let mut items = Vec::new();

        for entry in entries {
            match entry.kind {
                Kind::File { size, modified } => own.add_file(&entry.name, size, modified),
                Kind::Other => {}
                Kind::Dir => {
                    let child_abs = abs.join(&entry.name);
                    let child_rel = join(&rel, &entry.name);
                    let Ok(child_entries) = self.tree.read_dir(&child_abs) else {
                        own.unreadable += 1;
                        continue;
                    };
                    let names: Vec<&str> = child_entries.iter().map(|e| e.name.as_str()).collect();

                    items.push(match self.rules.find(&child_rel, &entry.name, &names) {
                        Some(rule) => Item::Settled(PlanNode {
                            path: child_rel,
                            decision: rule.decision,
                            source: Source::Rule {
                                rule: rule.id.clone(),
                                reason: rule.reason.clone(),
                            },
                            stats: self.measure(&child_abs, child_entries),
                            overrides: Vec::new(),
                            unit: None,
                        }),
                        None => {
                            Item::Review(self.walk(&child_abs, child_rel, depth + 1, child_entries))
                        }
                    });
                }
            }
        }

        let mut review = own.clone();
        for item in &items {
            if let Item::Review(child) = item {
                review.merge(&child.review);
            }
        }

        let node = match self.stop(&review, depth, &items) {
            Some(reason) => as_unit(rel, review.clone(), reason, items),
            None => self.cut(rel, own, items),
        };
        Walked { node, review }
    }

    /// Why this folder is one unit, or `None` if it must be cut.
    fn stop(&self, review: &Stats, depth: u32, items: &[Item]) -> Option<UnitReason> {
        let budget = self.budget;
        if review.bytes <= budget.split_above {
            return Some(UnitReason::Small);
        }
        if let Some((extension, percent)) = dominant_extension(review, budget) {
            return Some(UnitReason::Homogeneous { extension, percent });
        }
        if depth >= budget.max_depth {
            return Some(UnitReason::DepthLimit);
        }
        let standalone = items.iter().any(
            |item| matches!(item, Item::Review(child) if child.review.bytes >= budget.min_unit),
        );
        if !standalone {
            return Some(UnitReason::Leaf);
        }
        None
    }

    /// Keeps the sub-folders big enough to stand alone as their own nodes,
    /// and folds the others into this folder's rest.
    fn cut(&self, rel: String, mut rest: Stats, items: Vec<Item>) -> PlanNode {
        let mut overrides = Vec::new();
        for item in items {
            match item {
                Item::Settled(node) => overrides.push(node),
                Item::Review(child) if child.review.bytes < self.budget.min_unit => {
                    rest.merge(&child.review);
                    settled_within(child.node, &mut overrides);
                }
                // Cut, with nothing of its own to judge: its units move up.
                Item::Review(child) if child.node.unit.is_none() => {
                    overrides.extend(child.node.overrides);
                }
                Item::Review(child) => overrides.push(child.node),
            }
        }
        let has_rest = rest.files > 0 || rest.unreadable > 0;
        PlanNode {
            path: rel,
            decision: Decision::Review,
            source: Source::Default,
            stats: rest,
            overrides,
            unit: has_rest.then_some(UnitReason::Rest),
        }
    }

    /// Sizes a settled folder without applying rules inside it.
    fn measure(&self, abs: &Path, entries: Vec<Entry>) -> Stats {
        let mut stats = Stats::default();
        let mut pending = vec![(abs.to_path_buf(), entries)];
        while let Some((dir, entries)) = pending.pop() {
            for entry in entries {
                match entry.kind {
                    Kind::File { size, modified } => stats.add_file(&entry.name, size, modified),
                    Kind::Other => {}
                    Kind::Dir => {
                        let child = dir.join(&entry.name);
                        match self.tree.read_dir(&child) {
                            Ok(children) => pending.push((child, children)),
                            Err(_) => stats.unreadable += 1,
                        }
                    }
                }
            }
        }
        stats
    }
}

/// Makes a review folder one unit: everything to review under it joins its
/// stats, and only the settled folders keep their own nodes.
fn as_unit(rel: String, stats: Stats, reason: UnitReason, items: Vec<Item>) -> PlanNode {
    let mut overrides = Vec::new();
    for item in items {
        match item {
            Item::Settled(node) => overrides.push(node),
            Item::Review(child) => settled_within(child.node, &mut overrides),
        }
    }
    PlanNode {
        path: rel,
        decision: Decision::Review,
        source: Source::Default,
        stats,
        overrides,
        unit: Some(reason),
    }
}

/// Collects the settled nodes of a review node that is being folded into a
/// larger one. Its review figures are already counted by the caller.
fn settled_within(node: PlanNode, out: &mut Vec<PlanNode>) {
    for child in node.overrides {
        if child.decision == Decision::Review {
            settled_within(child, out);
        } else {
            out.push(child);
        }
    }
}

/// The extension that makes `stats` homogeneous, with its share in percent.
///
/// The bytes of the other extensions must also be small enough to be one
/// unit: a 93 % video folder can still hide gigabytes of documents.
fn dominant_extension(stats: &Stats, budget: &SplitBudget) -> Option<(String, u8)> {
    let (extension, &bytes) = stats.extensions.iter().max_by_key(|(_, bytes)| **bytes)?;
    // u128: `bytes * 100` cannot overflow.
    let percent = (u128::from(bytes) * 100).checked_div(u128::from(stats.bytes))?;
    let others = stats.bytes.saturating_sub(bytes);
    let homogeneous =
        percent >= u128::from(budget.homogeneous_percent) && others <= budget.split_above;
    homogeneous.then(|| (extension.clone(), u8::try_from(percent).unwrap_or(100)))
}

fn join(rel: &str, name: &str) -> String {
    if rel.is_empty() {
        name.to_owned()
    } else {
        format!("{rel}/{name}")
    }
}
