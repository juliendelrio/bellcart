//! Turns a tree and a rule set into a plan.
//!
//! The walk is top-down. A folder matched by a rule is settled together
//! with everything under it: we only measure it, we do not look for more
//! rules inside. A folder no rule matches inherits its parent's decision
//! (`review` at the root). Sub-folders that end up with the same decision
//! as their parent are folded into it, so the plan only lists the places
//! where the decision changes.

use std::path::Path;

use crate::pack::RuleSet;
use crate::plan::{Decision, Plan, PlanNode, Source, Stats};
use crate::tree::{Kind, Tree};
use crate::{Error, Result};

/// Scans `root` and builds its plan.
///
/// Fails only if `root` itself cannot be read. Unreadable folders below it
/// are counted in [`Stats::unreadable`] and the scan goes on.
pub fn scan(tree: &dyn Tree, root: &Path, rules: &RuleSet) -> Result<Plan> {
    let entries = tree.read_dir(root).map_err(|source| Error::Io {
        path: root.to_path_buf(),
        source,
    })?;
    let walker = Walker { tree, rules };
    let node = walker.walk(
        root,
        String::new(),
        Decision::Review,
        Source::Default,
        entries,
    );
    Ok(Plan::new(root.to_string_lossy().into_owned(), node))
}

struct Walker<'a> {
    tree: &'a dyn Tree,
    rules: &'a RuleSet,
}

impl Walker<'_> {
    fn walk(
        &self,
        abs: &Path,
        rel: String,
        decision: Decision,
        source: Source,
        entries: Vec<crate::tree::Entry>,
    ) -> PlanNode {
        let mut stats = Stats::default();
        let mut overrides = Vec::new();

        for entry in entries {
            match entry.kind {
                Kind::File { size, modified } => stats.add_file(&entry.name, size, modified),
                Kind::Other => {}
                Kind::Dir => {
                    let child_abs = abs.join(&entry.name);
                    let child_rel = join(&rel, &entry.name);
                    let Ok(child_entries) = self.tree.read_dir(&child_abs) else {
                        stats.unreadable += 1;
                        continue;
                    };
                    let names: Vec<&str> = child_entries.iter().map(|e| e.name.as_str()).collect();

                    let child = match self.rules.find(&child_rel, &entry.name, &names) {
                        Some(rule) => PlanNode {
                            path: child_rel,
                            decision: rule.decision,
                            source: Source::Rule {
                                rule: rule.id.clone(),
                                reason: rule.reason.clone(),
                            },
                            stats: self.measure(&child_abs, child_entries),
                            overrides: Vec::new(),
                        },
                        None => self.walk(
                            &child_abs,
                            child_rel,
                            decision,
                            Source::Default,
                            child_entries,
                        ),
                    };

                    if child.decision == decision && child.overrides.is_empty() {
                        stats.merge(&child.stats);
                    } else if child.decision == decision && child.source == Source::Default {
                        // Same decision by inheritance: keep its exceptions,
                        // fold the rest.
                        stats.merge(&child.stats);
                        overrides.extend(child.overrides);
                    } else {
                        overrides.push(child);
                    }
                }
            }
        }

        PlanNode {
            path: rel,
            decision,
            source,
            stats,
            overrides,
        }
    }

    /// Sizes a settled folder without applying rules inside it.
    fn measure(&self, abs: &Path, entries: Vec<crate::tree::Entry>) -> Stats {
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

fn join(rel: &str, name: &str) -> String {
    if rel.is_empty() {
        name.to_owned()
    } else {
        format!("{rel}/{name}")
    }
}
