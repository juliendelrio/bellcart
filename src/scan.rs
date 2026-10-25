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
//!
//! Sub-folders are listed in parallel with `rayon`, on whatever thread
//! pool the caller runs the scan in. Results are put back in listing order,
//! so the plan is the same whatever the number of threads.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use rayon::prelude::*;

use crate::pack::{ProgramAt, Rule, RuleSet};
use crate::plan::{Decision, Plan, PlanNode, Program, Source, SplitBudget, Stats, UnitReason};
use crate::recipe::{Planned, Recipes, keeps_more};
use crate::signature::{Named, Sample};
use crate::tree::{Entry, Kind, Tree};
use crate::{Error, Result};

/// What a scan produces.
#[derive(Debug, Clone)]
pub struct Scan {
    pub plan: Plan,
    /// For each review unit, keyed by its path, the largest entries lying
    /// directly in it. They feed [`crate::signature`]. They are kept out of
    /// the plan, which goes to copy tools and has no use for file names.
    pub samples: BTreeMap<String, Sample>,
}

/// Scans `root` and builds its plan, cutting the review mass with `budget`.
///
/// Fails only if `root` itself cannot be read. Unreadable folders below it
/// are counted in [`Stats::unreadable`] and the scan goes on.
pub fn scan(tree: &dyn Tree, root: &Path, rules: &RuleSet, budget: &SplitBudget) -> Result<Scan> {
    scan_with(
        tree,
        root,
        rules,
        &Recipes::default(),
        budget,
        &Progress::default(),
    )
}

/// [`scan`], applying the steps of the `recipes` found on the profile, and
/// counting what it has read in `progress` as it goes, so that another
/// thread can show how far it is.
///
/// A recipe step settles exactly its path, wherever it lies: in the review
/// mass, or inside a folder a rule settled. When a rule settles the same
/// path, the decision that keeps more wins, and the rule on a tie: a recipe
/// can rescue a folder from `leave`, never send one there against a rule.
pub fn scan_with(
    tree: &dyn Tree,
    root: &Path,
    rules: &RuleSet,
    recipes: &Recipes,
    budget: &SplitBudget,
    progress: &Progress,
) -> Result<Scan> {
    scan_drive(tree, root, "", rules, recipes, budget, progress)
}

/// [`scan_with`] on a root that holds the user profile at `profile`
/// (`Users/ana`, relative to `root`), such as a whole drive. Profile rules
/// apply inside the profile, relative to it; drive rules apply outside it.
/// `recipes` must be detected with [`crate::recipe::Variables::profile_at`]
/// the same profile. An empty `profile` means the root is the profile.
pub fn scan_drive(
    tree: &dyn Tree,
    root: &Path,
    profile: &str,
    rules: &RuleSet,
    recipes: &Recipes,
    budget: &SplitBudget,
    progress: &Progress,
) -> Result<Scan> {
    let profile = profile.trim_matches('/');
    let entries = tree.read_dir(root).map_err(|source| Error::Io {
        path: root.to_path_buf(),
        source,
    })?;
    progress.folders.fetch_add(1, Ordering::Relaxed);
    let walker = Walker {
        tree,
        rules,
        recipes,
        budget,
        progress,
        profile,
        programs: Mutex::default(),
    };
    let walked = walker.walk(root, String::new(), String::new(), 0, entries, Mode::Cut);
    // Folding a folder into a larger unit leaves its samples behind: keep
    // only those of the units the plan has.
    let mut units = BTreeSet::new();
    units_in(&walked.node, &mut units);
    let samples = walked
        .samples
        .into_iter()
        .filter(|(path, _)| units.contains(path))
        .collect();
    let mut plan = Plan::new(
        root.to_string_lossy().into_owned(),
        walked.node,
        Some(budget.clone()),
    );
    plan.profile = (!profile.is_empty()).then(|| profile.to_owned());
    plan.applications = recipes.applications().to_vec();
    // Sub-folders are walked in parallel: sort for a plan that does not
    // depend on the number of threads.
    let mut programs = walker
        .programs
        .into_inner()
        .unwrap_or_else(|e| e.into_inner());
    programs.sort_by(|a, b| a.path.cmp(&b.path));
    // The same program in `Program Files` and `Program Files (x86)` is
    // listed once, at its first folder.
    let mut seen = BTreeSet::new();
    programs.retain(|p| seen.insert((p.rule.clone(), p.name.to_lowercase())));
    plan.programs = programs;
    Ok(Scan { plan, samples })
}

/// The paths of the review units at or under `node`.
fn units_in(node: &PlanNode, out: &mut BTreeSet<String>) {
    if node.unit.is_some() {
        out.insert(node.path.clone());
    }
    for child in &node.overrides {
        units_in(child, out);
    }
}

/// How much a running scan has read so far: every folder listed, and
/// every file seen, settled or not.
///
/// The counters are atomics: the scan threads add to them without a lock,
/// and any thread may read them at any time. Each count is exact once the
/// scan is over; while it runs, the three are read one after the other and
/// may be a few entries apart.
#[derive(Debug, Default)]
pub struct Progress {
    folders: AtomicU64,
    files: AtomicU64,
    bytes: AtomicU64,
}

/// A reading of [`Progress`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub folders: u64,
    pub files: u64,
    pub bytes: u64,
}

impl Progress {
    pub fn counts(&self) -> Counts {
        Counts {
            folders: self.folders.load(Ordering::Relaxed),
            files: self.files.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
        }
    }

    /// Adds the files of one folder at once, to touch the shared counters
    /// once per folder rather than once per file.
    fn add_files(&self, stats: &Stats) {
        self.files.fetch_add(stats.files, Ordering::Relaxed);
        self.bytes.fetch_add(stats.bytes, Ordering::Relaxed);
    }
}

struct Walker<'a> {
    tree: &'a dyn Tree,
    rules: &'a RuleSet,
    recipes: &'a Recipes,
    budget: &'a SplitBudget,
    progress: &'a Progress,
    /// The user profile, relative to the root: `""` when it is the root.
    profile: &'a str,
    /// Programs the rules found installed, in no order.
    programs: Mutex<Vec<Program>>,
}

/// A review folder once walked: its node, already cut, the figures of
/// everything still to review under it, and the samples of the units its
/// node holds.
struct Walked {
    name: String,
    node: PlanNode,
    review: Stats,
    samples: Vec<(String, Sample)>,
}

/// An entry of a review folder still to look at: a sub-folder, or a file
/// already settled by a recipe.
enum Pending {
    Dir(String),
    Settled(Box<PlanNode>),
}

/// A sub-folder of a review folder, in listing order.
enum Item {
    Settled(PlanNode),
    Review(Walked),
    /// A folder a `review` rule recognised: one unit of its own, never cut
    /// and never folded into another.
    Recognised(Walked),
}

/// How a review folder is walked.
#[derive(Clone)]
enum Mode {
    /// Cut by the budget.
    Cut,
    /// Judged whole, for this reason.
    Unit(UnitReason),
    /// Cut one level: each sub-folder is judged whole for `part`, and the
    /// small ones share the folder's rest, which carries `rest`.
    Parts { part: UnitReason, rest: UnitReason },
}

impl Walker<'_> {
    fn walk(
        &self,
        abs: &Path,
        name: String,
        rel: String,
        depth: u32,
        entries: Vec<Entry>,
        mode: Mode,
    ) -> Walked {
        let mut own = Stats::default();
        let mut files = Vec::new();
        let mut pending = Vec::new();

        for entry in entries {
            match entry.kind {
                Kind::File { size, modified } => {
                    let file_rel = join(&rel, &entry.name);
                    if let Some(planned) = self.recipes.at(&file_rel) {
                        let node = file_node(file_rel, &entry.name, size, modified, planned);
                        self.progress.add_files(&node.stats);
                        pending.push(Pending::Settled(Box::new(node)));
                        continue;
                    }
                    let in_profile = within(&file_rel, self.profile);
                    if let Some(rule) = self.rules.find_file(in_profile, &file_rel, &entry.name) {
                        let mut stats = Stats::default();
                        stats.add_file(&entry.name, size, modified);
                        self.progress.add_files(&stats);
                        let node = PlanNode {
                            path: file_rel,
                            decision: rule.decision,
                            source: Source::Rule {
                                rule: rule.id.clone(),
                                reason: rule.reason.clone(),
                            },
                            stats,
                            overrides: Vec::new(),
                            unit: None,
                        };
                        pending.push(Pending::Settled(Box::new(node)));
                        continue;
                    }
                    own.add_file(&entry.name, size, modified);
                    files.push(Named {
                        name: entry.name,
                        bytes: size,
                    });
                }
                Kind::Other => {}
                Kind::Dir => pending.push(Pending::Dir(entry.name)),
            }
        }
        self.progress.add_files(&own);

        // `into_par_iter` hands the sub-folders to the thread pool, and
        // `collect` puts the results back in the original order.
        let children: Vec<Option<Item>> = pending
            .into_par_iter()
            .map(|entry| match entry {
                Pending::Dir(child) => self.child(abs, &rel, depth, child, &mode),
                Pending::Settled(node) => Some(Item::Settled(*node)),
            })
            .collect();
        let mut items = Vec::with_capacity(children.len());
        for child in children {
            match child {
                Some(item) => items.push(item),
                None => own.unreadable += 1,
            }
        }

        let mut review = own.clone();
        for item in &items {
            if let Item::Review(child) = item {
                review.merge(&child.review);
            }
        }

        let stop = match &mode {
            Mode::Cut => self.stop(&review, depth, &items),
            Mode::Unit(reason) => Some(reason.clone()),
            Mode::Parts { .. } => None,
        };
        let (node, samples) = match stop {
            Some(reason) => {
                let folders = items
                    .iter()
                    .filter_map(|item| match item {
                        // A sub-folder whose content all has its own node
                        // adds nothing to this unit.
                        Item::Review(child) if child.review.files > 0 => Some(named(child)),
                        Item::Review(_) | Item::Settled(_) | Item::Recognised(_) => None,
                    })
                    .collect();
                let sample = (rel.clone(), Sample::new(folders, files));
                let (mut node, mut samples) = as_unit(rel, review.clone(), reason, items);
                if review.files == 0 && review.unreadable == 0 {
                    // Nothing left to judge here: no card, only what it holds.
                    node.unit = None;
                } else {
                    samples.push(sample);
                }
                (node, samples)
            }
            None => {
                let (mut node, samples) = self.cut(rel, own, files, items);
                if let Mode::Parts { rest, .. } = mode
                    && node.unit == Some(UnitReason::Rest)
                {
                    node.unit = Some(rest);
                }
                (node, samples)
            }
        };
        Walked {
            name,
            node,
            review,
            samples,
        }
    }

    /// Lists a sub-folder and either settles it, by a rule or a recipe, or
    /// walks it. `None` when it cannot be listed.
    /// `mode` is the parent's: in a folder cut into parts, a sub-folder no
    /// rule knows is one part.
    fn child(&self, abs: &Path, rel: &str, depth: u32, name: String, mode: &Mode) -> Option<Item> {
        let child_abs = abs.join(&name);
        let child_rel = join(rel, &name);
        let Some(child_entries) = self.list(&child_abs) else {
            return self.unlisted(child_rel, &name);
        };
        let names: Vec<&str> = child_entries.iter().map(|e| e.name.as_str()).collect();

        let in_profile = within(&child_rel, self.profile);
        let found = self.rules.find_at(in_profile, &child_rel, &name, &names);
        if let Some(rule) = found {
            self.note_programs(rule, &child_rel, &name, &child_entries);
        }
        // A `review` rule settles nothing: it makes the folder one unit.
        let (rule, recognised) = match found {
            Some(rule) if rule.decision == Decision::Review => (None, Some(rule)),
            other => (other, None),
        };
        let settled = match (rule, self.recipes.at(&child_rel)) {
            (Some(rule), Some(planned)) if keeps_more(planned.decision, rule.decision) => {
                Some((planned.decision, recipe_source(planned)))
            }
            (Some(rule), _) => Some((
                rule.decision,
                Source::Rule {
                    rule: rule.id.clone(),
                    reason: rule.reason.clone(),
                },
            )),
            (None, Some(planned)) => Some((planned.decision, recipe_source(planned))),
            (None, None) => None,
        };
        Some(match settled {
            Some((decision, source)) => {
                let (stats, overrides) =
                    self.measure(&child_abs, &child_rel, child_entries, decision);
                Item::Settled(PlanNode {
                    path: child_rel,
                    decision,
                    source,
                    stats,
                    overrides,
                    unit: None,
                })
            }
            None => {
                let depth = depth + 1;
                let walk =
                    |mode| self.walk(&child_abs, name, child_rel, depth, child_entries, mode);
                match recognised {
                    // A folder whose sub-folders are the units, such as the
                    // applications in AppData/Local. Never folded into a
                    // larger unit, unless nothing in it is left to review.
                    Some(rule) if rule.parts.is_some() => {
                        let walked = walk(Mode::Parts {
                            part: UnitReason::Part {
                                rule: rule.id.clone(),
                                key: rule.parts.clone().unwrap_or_default(),
                            },
                            rest: UnitReason::Rule {
                                rule: rule.id.clone(),
                                key: rule.reason.clone(),
                                propose: None,
                            },
                        });
                        if walked.review.files == 0 && walked.review.unreadable == 0 {
                            Item::Review(walked)
                        } else {
                            Item::Recognised(walked)
                        }
                    }
                    // Judged whole like a part, and folded above when small,
                    // such as a tool's dot folder at the profile root.
                    Some(rule) if rule.fold_small => {
                        Item::Review(walk(Mode::Unit(UnitReason::Part {
                            rule: rule.id.clone(),
                            key: rule.reason.clone(),
                        })))
                    }
                    Some(rule) => {
                        let mut walked = walk(Mode::Unit(UnitReason::Rule {
                            rule: rule.id.clone(),
                            key: rule.reason.clone(),
                            propose: rule.propose,
                        }));
                        // Recognised, but everything in it is settled: no card.
                        if walked.review.files == 0 && walked.review.unreadable == 0 {
                            walked.node.unit = None;
                            Item::Review(walked)
                        } else {
                            Item::Recognised(walked)
                        }
                    }
                    None => match mode {
                        Mode::Parts { part, .. } => Item::Review(walk(Mode::Unit(part.clone()))),
                        Mode::Cut | Mode::Unit(_) => Item::Review(walk(Mode::Cut)),
                    },
                }
            }
        })
    }

    /// Lists the programs `rule` finds in the folder at `rel`.
    fn note_programs(&self, rule: &Rule, rel: &str, name: &str, entries: &[Entry]) {
        let program = |path: String, name: String| Program {
            path,
            name,
            rule: rule.id.clone(),
            reinstall: Default::default(),
        };
        let found: Vec<Program> = match rule.program {
            None => return,
            Some(ProgramAt::Folder) => {
                let named = rule.name.as_ref().map(|n| n.en().to_owned());
                vec![program(
                    rel.to_owned(),
                    named.unwrap_or_else(|| name.to_owned()),
                )]
            }
            Some(ProgramAt::Each) => entries
                .iter()
                .filter(|e| matches!(e.kind, Kind::Dir) && self.rules.is_program(rule, &e.name))
                .map(|e| program(join(rel, &e.name), e.name.clone()))
                .collect(),
        };
        // A poisoned lock only means another thread panicked: the list it
        // holds is still whole.
        let mut programs = self.programs.lock().unwrap_or_else(|e| e.into_inner());
        programs.extend(found);
    }

    /// The recipe step on `rel` inside a folder settled as `decision`, if
    /// it decides otherwise.
    fn recipe_inside(&self, rel: &str, decision: Decision) -> Option<&Planned> {
        self.recipes.at(rel).filter(|p| p.decision != decision)
    }

    /// Lists a folder, counting it in the progress when it can be read.
    /// A sub-folder that cannot be listed. A `leave` rule that knows it by
    /// path or name alone still settles it, such as `Recovery` at a drive
    /// root without administrator rights: its node shows the unreadable
    /// entry. `None` otherwise, and the parent counts it as unreadable.
    fn unlisted(&self, rel: String, name: &str) -> Option<Item> {
        let in_profile = within(&rel, self.profile);
        let rule = self.rules.find_unlisted(in_profile, &rel, name)?;
        if rule.decision != Decision::Leave || self.recipes.at(&rel).is_some() {
            return None;
        }
        Some(Item::Settled(PlanNode {
            path: rel,
            decision: Decision::Leave,
            source: Source::Rule {
                rule: rule.id.clone(),
                reason: rule.reason.clone(),
            },
            stats: Stats {
                unreadable: 1,
                ..Stats::default()
            },
            overrides: Vec::new(),
            unit: None,
        }))
    }

    fn list(&self, dir: &Path) -> Option<Vec<Entry>> {
        let entries = self.tree.read_dir(dir).ok()?;
        self.progress.folders.fetch_add(1, Ordering::Relaxed);
        Some(entries)
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
    fn cut(
        &self,
        rel: String,
        mut rest: Stats,
        files: Vec<Named>,
        items: Vec<Item>,
    ) -> (PlanNode, Vec<(String, Sample)>) {
        let mut overrides = Vec::new();
        let mut samples = Vec::new();
        let mut folded = Vec::new();
        let mut tiny_rests = Vec::new();
        for item in items {
            match item {
                Item::Settled(node) => overrides.push(node),
                Item::Recognised(child) => {
                    keep(child.node, &mut overrides);
                    samples.extend(child.samples);
                }
                Item::Review(child) if child.review.bytes < self.budget.min_unit => {
                    rest.merge(&child.review);
                    folded.push(named(&child));
                    settled_within(child.node, &mut overrides);
                    samples.extend(child.samples);
                }
                // Cut, with a rest too small to judge on its own: that rest
                // joins ours, and its units move up. A rest holding an
                // unreadable folder stays, so it shows where access failed.
                Item::Review(child) if self.is_tiny_rest(&child.node) => {
                    let Walked {
                        name,
                        node,
                        samples: mut child_samples,
                        ..
                    } = child;
                    rest.merge(&node.stats);
                    overrides.extend(node.overrides);
                    if let Some(at) = child_samples
                        .iter()
                        .position(|(path, _)| *path == node.path)
                    {
                        tiny_rests.push((name, child_samples.remove(at).1));
                    }
                    samples.extend(child_samples);
                }
                // Cut, with nothing of its own to judge: its units move up.
                Item::Review(child) if child.node.unit.is_none() => {
                    overrides.extend(child.node.overrides);
                    samples.extend(child.samples);
                }
                Item::Review(child) => {
                    overrides.push(child.node);
                    samples.extend(child.samples);
                }
            }
        }
        let has_rest = rest.files > 0 || rest.unreadable > 0;
        if has_rest {
            let mut sample = Sample::new(folded, files);
            for (name, tiny) in tiny_rests {
                sample.absorb(tiny, &name);
            }
            samples.push((rel.clone(), sample));
        }
        let node = PlanNode {
            path: rel,
            decision: Decision::Review,
            source: Source::Default,
            stats: rest,
            overrides,
            unit: has_rest.then_some(UnitReason::Rest),
        };
        (node, samples)
    }

    fn is_tiny_rest(&self, node: &PlanNode) -> bool {
        node.unit == Some(UnitReason::Rest)
            && node.stats.bytes < self.budget.min_unit
            && node.stats.unreadable == 0
    }

    /// Sizes a folder settled as `decision`, without applying rules inside
    /// it. Recipe steps still apply, and come back as its overrides.
    fn measure(
        &self,
        abs: &Path,
        rel: &str,
        entries: Vec<Entry>,
        decision: Decision,
    ) -> (Stats, Vec<PlanNode>) {
        let mut stats = Stats::default();
        let mut overrides = Vec::new();
        let mut dirs: Vec<String> = Vec::new();
        for entry in entries {
            match entry.kind {
                Kind::File { size, modified } => {
                    let file_rel = join(rel, &entry.name);
                    match self.recipe_inside(&file_rel, decision) {
                        Some(planned) => {
                            let node = file_node(file_rel, &entry.name, size, modified, planned);
                            self.progress.add_files(&node.stats);
                            overrides.push(node);
                        }
                        None => stats.add_file(&entry.name, size, modified),
                    }
                }
                Kind::Other => {}
                Kind::Dir => dirs.push(entry.name),
            }
        }
        self.progress.add_files(&stats);

        let children: Vec<(Stats, Vec<PlanNode>)> = dirs
            .into_par_iter()
            .map(|name| {
                let dir_abs: PathBuf = abs.join(&name);
                let dir_rel = join(rel, &name);
                let Some(entries) = self.list(&dir_abs) else {
                    let unreadable = Stats {
                        unreadable: 1,
                        ..Stats::default()
                    };
                    return (unreadable, Vec::new());
                };
                match self.recipe_inside(&dir_rel, decision) {
                    Some(planned) => {
                        let (stats, overrides) =
                            self.measure(&dir_abs, &dir_rel, entries, planned.decision);
                        let node = PlanNode {
                            path: dir_rel,
                            decision: planned.decision,
                            source: recipe_source(planned),
                            stats,
                            overrides,
                            unit: None,
                        };
                        (Stats::default(), vec![node])
                    }
                    None => self.measure(&dir_abs, &dir_rel, entries, decision),
                }
            })
            .collect();
        for (child, nodes) in children {
            stats.merge(&child);
            overrides.extend(nodes);
        }
        (stats, overrides)
    }
}

fn recipe_source(planned: &Planned) -> Source {
    Source::Recipe {
        recipe: planned.recipe.clone(),
    }
}

/// A single file settled by a recipe step.
fn file_node(
    path: String,
    name: &str,
    size: u64,
    modified: Option<u64>,
    planned: &Planned,
) -> PlanNode {
    let mut stats = Stats::default();
    stats.add_file(name, size, modified);
    PlanNode {
        path,
        decision: planned.decision,
        source: recipe_source(planned),
        stats,
        overrides: Vec::new(),
        unit: None,
    }
}

/// Makes a review folder one unit: everything to review under it joins its
/// stats, and only the settled folders keep their own nodes.
/// Makes `rel` one unit. The samples returned are those of the recognised
/// units inside it, which keep their own node; the scan drops the others.
fn as_unit(
    rel: String,
    stats: Stats,
    reason: UnitReason,
    items: Vec<Item>,
) -> (PlanNode, Vec<(String, Sample)>) {
    let mut overrides = Vec::new();
    let mut samples = Vec::new();
    for item in items {
        match item {
            Item::Settled(node) => overrides.push(node),
            Item::Recognised(child) => {
                keep(child.node, &mut overrides);
                samples.extend(child.samples);
            }
            Item::Review(child) => {
                settled_within(child.node, &mut overrides);
                samples.extend(child.samples);
            }
        }
    }
    let node = PlanNode {
        path: rel,
        decision: Decision::Review,
        source: Source::Default,
        stats,
        overrides,
        unit: Some(reason),
    };
    (node, samples)
}

/// Keeps a recognised node. One cut into parts with no rest of its own has
/// nothing to judge: its parts move up.
fn keep(node: PlanNode, overrides: &mut Vec<PlanNode>) {
    if node.unit.is_some() {
        overrides.push(node);
    } else {
        overrides.extend(node.overrides);
    }
}

/// A unit a `review` rule made, or one of its parts: never folded into a
/// unit above. (A small part is folded into its own folder's rest, by the
/// cut of that folder.)
fn is_recognised(node: &PlanNode) -> bool {
    matches!(
        node.unit,
        Some(UnitReason::Rule { .. } | UnitReason::Part { .. })
    )
}

/// Collects the settled nodes of a review node that is being folded into a
/// larger one, and the recognised units, which stay. Its review figures are
/// already counted by the caller.
fn settled_within(node: PlanNode, out: &mut Vec<PlanNode>) {
    for child in node.overrides {
        if child.decision == Decision::Review && !is_recognised(&child) {
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

/// A walked sub-folder as an entry of its parent's sample.
fn named(child: &Walked) -> Named {
    Named {
        name: child.name.clone(),
        bytes: child.review.bytes,
    }
}

fn join(rel: &str, name: &str) -> String {
    if rel.is_empty() {
        name.to_owned()
    } else {
        format!("{rel}/{name}")
    }
}

/// `path` relative to `folder` when it lies inside it, `None` otherwise.
/// Every path lies inside the empty folder, the root.
fn within<'a>(path: &'a str, folder: &str) -> Option<&'a str> {
    if folder.is_empty() {
        return Some(path);
    }
    let rest = path.get(folder.len()..)?;
    let head = path.get(..folder.len())?;
    if !head.eq_ignore_ascii_case(folder) {
        return None;
    }
    match rest.strip_prefix('/') {
        Some(rest) => Some(rest),
        None if rest.is_empty() => Some(""),
        None => None,
    }
}
