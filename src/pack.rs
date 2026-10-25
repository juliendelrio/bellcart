//! Packs: the declarative, community-maintained knowledge of Bellcart.
//!
//! A pack is a folder of YAML files. It holds no code: rules say where
//! folders go, recipes say what to do about an application, and the
//! question set is what an arbiter is asked about undecided folders.
//!
//! ```text
//! packs/core/
//!   pack.yaml          metadata
//!   questions.yaml     arbiter question set (optional)
//!   rules/*.yaml       folder rules
//!   recipes/*.yaml     one application per file
//! ```
//!
//! The types below are the single source of truth for the format: the JSON
//! Schemas in `schemas/` are generated from them (see [`crate::schema`]).

use std::collections::BTreeMap;
use std::path::Path;

use globset::{GlobBuilder, GlobMatcher};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::arbiter::Question;
use crate::plan::Decision;
use crate::{Error, Result};

/// `pack.yaml`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackMeta {
    /// Unique, lowercase, dash-separated.
    pub id: String,
    pub name: String,
    /// Version of the pack content, not of the format.
    pub version: String,
    /// Version of the pack format this pack targets.
    pub format: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// SPDX identifier of the licence the pack content is published under.
    pub license: String,
}

pub const PACK_FORMAT: u32 = 1;

/// `rules/*.yaml`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleFile {
    pub rules: Vec<Rule>,
}

/// Settles a folder and everything under it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// Unique within the pack, lowercase, dash-separated.
    pub id: String,
    /// `review` settles nothing: it makes the folder one unit to review,
    /// whatever its size, with this reason. Rules still apply inside it.
    /// Use it for folders Bellcart recognises but must not decide, such as
    /// a synced folder.
    pub decision: Decision,
    /// Translation key. `locales/*/bellcart.ftl` must define `reason-<key>`.
    pub reason: String,
    /// What the folder belongs to, shown on its card: an application's
    /// name. Only with `decision: review`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
    /// With `decision: review`: the decision proposed on the card, which
    /// the user takes with one key. Never applied without the user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propose: Option<Decision>,
    /// With `decision: review`: cut the folder one level instead of making
    /// it one unit. Each sub-folder no other rule knows is one unit, of the
    /// kind this translation key names (`reason-<key>`); the small ones
    /// share the folder's rest, whose card shows `reason`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parts: Option<String>,
    /// With `decision: review`: the folder is judged whole like a part,
    /// but one too small for a card of its own joins the card of the
    /// folder above. Without it, a recognised folder always has its own
    /// card. Not with `name`, `propose` or `parts`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fold_small: bool,
    /// The folder shows an installed program, which goes on the plan's
    /// list of programs to install again (`Plan::programs`). Any decision.
    /// Not with `fold_small`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program: Option<ProgramAt>,
    #[serde(rename = "match")]
    pub matcher: Matcher,
}

/// Where a rule finds installed programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProgramAt {
    /// The folder itself is one program, named by the rule's `name` or by
    /// the folder name.
    Folder,
    /// Each sub-folder is one program, named by its folder name, such as
    /// in `AppData/Local/Programs`.
    Each,
}

/// Conditions on a folder. All the conditions given must hold.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Matcher {
    /// The folder name is one of these (case-insensitive).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub name_in: Vec<String>,
    /// The path relative to the scan root matches this glob. `/` separates
    /// components, `*` stays within one component, `**` crosses them.
    /// Case-insensitive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_glob: Option<String>,
    /// The folder directly contains an entry with one of these names
    /// (case-insensitive). Use it to tell a real `venv` from a folder that
    /// merely shares the name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contains: Vec<String>,
}

/// `recipes/*.yaml`: what to do about one application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    /// Unique within the pack, lowercase, dash-separated.
    pub id: String,
    pub name: Name,
    pub detect: Detect,
    /// Steps on the old PC, before leaving it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<Step>,
    /// Steps on the new PC.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<Step>,
}

/// The application is considered present if any probe succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Detect {
    pub any: Vec<Probe>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Probe {
    /// A path that exists. May start with a `%VARIABLE%`.
    pub path: String,
}

/// One step of a recipe.
///
/// This list is closed on purpose. A recipe can never carry a shell command:
/// anything that runs is implemented and reviewed in the engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Step {
    /// Take this file or folder. May start with a `%VARIABLE%`.
    Copy {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<Text>,
    },
    /// Do not take this file or folder, even if a parent is migrated.
    Skip { path: String },
    /// Something only the user can do.
    Manual { text: Text },
    /// Save the list of applications installed through winget.
    ExportWingetPackages,
    /// Save the list of installed Visual Studio Code extensions.
    ExportVscodeExtensions,
    /// Save one registry key to a `.reg` file.
    ExportRegistryKey { key: String },
    /// Install the application with winget.
    Install { winget: String },
}

/// A text shown to the user. English is mandatory, other languages are
/// optional and fall back to English.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Text {
    pub en: String,
    /// Other languages, keyed by language code (`fr`, `de`...).
    #[serde(flatten)]
    pub translations: BTreeMap<String, String>,
}

impl Text {
    pub fn get(&self, lang: &str) -> &str {
        self.translations
            .get(lang)
            .map_or(self.en.as_str(), String::as_str)
    }
}

/// A name shown to the user: one string for every language, or a [`Text`]
/// when it reads differently in some.
///
/// `untagged` lets serde try each form in turn, so both `name: Git` and
/// `name: {en: Installed applications, fr: Applications installées}` load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Name {
    Same(String),
    Translated(Text),
}

impl Name {
    pub fn get(&self, lang: &str) -> &str {
        match self {
            Name::Same(name) => name,
            Name::Translated(text) => text.get(lang),
        }
    }

    /// The English name, which the plan records.
    pub fn en(&self) -> &str {
        self.get("en")
    }
}

/// `questions.yaml`: what an arbiter is asked about an undecided folder.
///
/// Question ids are stable: decision logs refer to them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuestionSet {
    /// Bumped whenever a question changes meaning.
    pub version: u32,
    pub questions: BTreeMap<String, Question>,
}

/// A loaded pack.
#[derive(Debug, Clone)]
pub struct Pack {
    pub meta: PackMeta,
    pub rules: Vec<Rule>,
    pub recipes: Vec<Recipe>,
    pub questions: Option<QuestionSet>,
}

impl Pack {
    /// Builds a pack from `(relative path, content)` pairs. Paths use `/`.
    ///
    /// Taking plain pairs keeps this crate independent of where packs come
    /// from: embedded in the binary, or a folder on disk.
    pub fn from_files<I, P, C>(files: I) -> Result<Self>
    where
        I: IntoIterator<Item = (P, C)>,
        P: AsRef<str>,
        C: AsRef<str>,
    {
        let mut sorted: Vec<(String, String)> = files
            .into_iter()
            .map(|(p, c)| (p.as_ref().replace('\\', "/"), c.as_ref().to_owned()))
            .collect();
        sorted.sort();

        let mut meta = None;
        let mut rules = Vec::new();
        let mut recipes = Vec::new();
        let mut questions = None;

        for (path, content) in &sorted {
            if !path.ends_with(".yaml") {
                continue;
            }
            if path == "pack.yaml" {
                meta = Some(parse::<PackMeta>(path, content)?);
            } else if path == "questions.yaml" {
                questions = Some(parse::<QuestionSet>(path, content)?);
            } else if path.starts_with("rules/") {
                rules.extend(parse::<RuleFile>(path, content)?.rules);
            } else if path.starts_with("recipes/") {
                let recipe = parse::<Recipe>(path, content)?;
                if *path != format!("recipes/{}.yaml", recipe.id) {
                    return Err(Error::PackFile {
                        file: path.clone(),
                        message: format!(
                            "a recipe file is named after its id: expected recipes/{}.yaml",
                            recipe.id
                        ),
                    });
                }
                recipes.push(recipe);
            } else {
                return Err(Error::PackFile {
                    file: path.clone(),
                    message: "unexpected file: packs hold pack.yaml, questions.yaml, \
                              rules/*.yaml and recipes/*.yaml"
                        .to_owned(),
                });
            }
        }

        let meta = meta.ok_or_else(|| Error::PackFile {
            file: "pack.yaml".to_owned(),
            message: "missing".to_owned(),
        })?;
        if meta.format != PACK_FORMAT {
            return Err(Error::PackFile {
                file: "pack.yaml".to_owned(),
                message: format!(
                    "format {} is not supported (expected {PACK_FORMAT})",
                    meta.format
                ),
            });
        }

        let pack = Self {
            meta,
            rules,
            recipes,
            questions,
        };
        pack.check_ids()?;
        // Compiling validates every matcher.
        RuleSet::compile(&pack.rules)?;
        Ok(pack)
    }

    /// Loads a pack from a folder on disk.
    pub fn from_dir(dir: &Path) -> Result<Self> {
        let mut files = Vec::new();
        read_yaml_files(dir, dir, &mut files)?;
        Self::from_files(files)
    }

    fn check_ids(&self) -> Result<()> {
        let mut seen = std::collections::BTreeSet::new();
        for rule in &self.rules {
            check_id(&rule.id).map_err(|message| Error::Rule {
                rule: rule.id.clone(),
                message,
            })?;
            if !seen.insert(&rule.id) {
                return Err(Error::Rule {
                    rule: rule.id.clone(),
                    message: "duplicate id".to_owned(),
                });
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for recipe in &self.recipes {
            let file = format!("recipes/{}.yaml", recipe.id);
            check_id(&recipe.id).map_err(|message| Error::PackFile {
                file: file.clone(),
                message,
            })?;
            if !seen.insert(&recipe.id) {
                return Err(Error::PackFile {
                    file,
                    message: "duplicate recipe id".to_owned(),
                });
            }
            check_recipe_paths(recipe).map_err(|message| Error::PackFile { file, message })?;
        }
        Ok(())
    }
}

/// Every path of a recipe stays inside the profile, and a `skip` inside an
/// application's data: see [`crate::recipe::check_skip`].
fn check_recipe_paths(recipe: &Recipe) -> std::result::Result<(), String> {
    for probe in &recipe.detect.any {
        crate::recipe::check_path(&probe.path)?;
    }
    for step in recipe.before.iter().chain(&recipe.after) {
        match step {
            Step::Copy { path, .. } => {
                crate::recipe::check_path(path)?;
            }
            Step::Skip { path } => crate::recipe::check_skip(path)?,
            Step::ExportRegistryKey { key } => crate::export::check_registry_key(key)?,
            _ => {}
        }
    }
    Ok(())
}

/// The rules and recipes of several packs, in the order the packs come.
#[derive(Debug, Clone, Default)]
pub struct Library {
    pub rules: Vec<Rule>,
    pub recipes: Vec<Recipe>,
}

/// Puts packs together, the core pack first.
///
/// Rules keep the order of their packs, and the first match wins: a pack
/// loaded after the core pack can add rules but never change what a core
/// rule settles. Pack, rule and recipe ids must be unique across packs.
pub fn combine(packs: &[Pack]) -> Result<Library> {
    let mut library = Library::default();
    let mut pack_ids = std::collections::BTreeSet::new();
    let mut rule_ids: BTreeMap<&str, &str> = BTreeMap::new();
    let mut recipe_ids: BTreeMap<&str, &str> = BTreeMap::new();
    for pack in packs {
        let id = pack.meta.id.as_str();
        if !pack_ids.insert(id) {
            return Err(Error::PackFile {
                file: "pack.yaml".to_owned(),
                message: format!("pack `{id}` is loaded twice"),
            });
        }
        for rule in &pack.rules {
            if let Some(owner) = rule_ids.insert(&rule.id, id) {
                return Err(Error::Rule {
                    rule: rule.id.clone(),
                    message: format!("id already used by pack `{owner}`"),
                });
            }
        }
        for recipe in &pack.recipes {
            if let Some(owner) = recipe_ids.insert(&recipe.id, id) {
                return Err(Error::PackFile {
                    file: format!("recipes/{}.yaml", recipe.id),
                    message: format!("recipe id already used by pack `{owner}`"),
                });
            }
        }
        library.rules.extend(pack.rules.iter().cloned());
        library.recipes.extend(pack.recipes.iter().cloned());
    }
    Ok(library)
}

fn check_id(id: &str) -> std::result::Result<(), String> {
    let ok = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err("ids are lowercase letters, digits and dashes".to_owned())
    }
}

fn parse<T: for<'de> Deserialize<'de>>(file: &str, content: &str) -> Result<T> {
    serde_saphyr::from_str(content).map_err(|e| Error::PackFile {
        file: file.to_owned(),
        message: e.to_string(),
    })
}

fn read_yaml_files(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) -> Result<()> {
    let io = |source| Error::Io {
        path: dir.to_path_buf(),
        source,
    };
    for item in std::fs::read_dir(dir).map_err(io)? {
        let item = item.map_err(io)?;
        let path = item.path();
        if path.is_dir() {
            read_yaml_files(root, &path, out)?;
        } else if path.extension().is_some_and(|e| e == "yaml") {
            let content = std::fs::read_to_string(&path).map_err(|source| Error::Io {
                path: path.clone(),
                source,
            })?;
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, content));
        }
    }
    Ok(())
}

/// Rules compiled for matching. The first matching rule wins.
#[derive(Debug, Clone)]
pub struct RuleSet {
    rules: Vec<Compiled>,
}

#[derive(Debug, Clone)]
struct Compiled {
    rule: Rule,
    names: Vec<String>,
    glob: Option<GlobMatcher>,
    contains: Vec<String>,
}

impl RuleSet {
    pub fn compile(rules: &[Rule]) -> Result<Self> {
        let mut compiled = Vec::with_capacity(rules.len());
        for rule in rules {
            let fail = |message: String| Error::Rule {
                rule: rule.id.clone(),
                message,
            };
            let review = rule.decision == Decision::Review;
            if !review
                && (rule.name.is_some()
                    || rule.propose.is_some()
                    || rule.parts.is_some()
                    || rule.fold_small)
            {
                return Err(fail(
                    "`name`, `propose`, `parts` and `fold_small` go with `decision: review` only"
                        .to_owned(),
                ));
            }
            if rule.fold_small
                && (rule.name.is_some() || rule.propose.is_some() || rule.parts.is_some())
            {
                return Err(fail(
                    "a rule with `fold_small` is a kind of folder, not a known one: \
                     no `name`, `propose` or `parts`"
                        .to_owned(),
                ));
            }
            if rule.fold_small && rule.program.is_some() {
                return Err(fail(
                    "a rule with `fold_small` is a kind of folder: it names no `program`"
                        .to_owned(),
                ));
            }
            if rule.propose == Some(Decision::Review) {
                return Err(fail(
                    "`propose` must be a decision, not `review`".to_owned(),
                ));
            }
            if rule.parts.is_some() && (rule.propose.is_some() || rule.name.is_some()) {
                return Err(fail(
                    "a rule with `parts` proposes nothing: its parts are unknown".to_owned(),
                ));
            }
            let m = &rule.matcher;
            if m.name_in.is_empty() && m.path_glob.is_none() {
                return Err(fail(
                    "a rule needs `name_in` or `path_glob`; `contains` alone is too broad"
                        .to_owned(),
                ));
            }
            let glob = match &m.path_glob {
                Some(pattern) => Some(
                    GlobBuilder::new(pattern)
                        .case_insensitive(true)
                        .literal_separator(true)
                        .build()
                        .map_err(|e| fail(e.to_string()))?
                        .compile_matcher(),
                ),
                None => None,
            };
            compiled.push(Compiled {
                rule: rule.clone(),
                names: m.name_in.iter().map(|n| n.to_lowercase()).collect(),
                glob,
                contains: m.contains.iter().map(|n| n.to_lowercase()).collect(),
            });
        }
        Ok(Self { rules: compiled })
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Finds the rule that settles a folder, if any.
    ///
    /// `rel_path` is relative to the scan root with `/` separators,
    /// `children` are the names of the folder's direct entries.
    pub fn find(&self, rel_path: &str, name: &str, children: &[&str]) -> Option<&Rule> {
        let name = name.to_lowercase();
        self.rules
            .iter()
            .find(|c| {
                (c.names.is_empty() || c.names.contains(&name))
                    && c.glob.as_ref().is_none_or(|g| g.is_match(rel_path))
                    && (c.contains.is_empty()
                        || children
                            .iter()
                            .any(|child| c.contains.contains(&child.to_lowercase())))
            })
            .map(|c| &c.rule)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(yaml: &str) -> Rule {
        serde_saphyr::from_str(yaml).unwrap()
    }

    #[test]
    fn name_match_is_case_insensitive() {
        let set = RuleSet::compile(&[rule(
            "{id: deps, decision: leave, reason: r, match: {name_in: [node_modules]}}",
        )])
        .unwrap();
        assert!(set.find("app/Node_Modules", "Node_Modules", &[]).is_some());
        assert!(set.find("app/src", "src", &[]).is_none());
    }

    #[test]
    fn star_does_not_cross_folders() {
        let set = RuleSet::compile(&[rule(
            "{id: cache, decision: leave, reason: r, match: {path_glob: 'AppData/Local/*/Cache'}}",
        )])
        .unwrap();
        assert!(set.find("AppData/Local/Foo/Cache", "Cache", &[]).is_some());
        assert!(
            set.find("AppData/Local/Foo/Bar/Cache", "Cache", &[])
                .is_none()
        );
    }

    #[test]
    fn contains_narrows_a_name_match() {
        let set = RuleSet::compile(&[rule(
            "{id: venv, decision: leave, reason: r, match: {name_in: [venv], contains: [pyvenv.cfg]}}",
        )])
        .unwrap();
        assert!(set.find("p/venv", "venv", &["Lib", "pyvenv.cfg"]).is_some());
        assert!(set.find("p/venv", "venv", &["notes.txt"]).is_none());
    }

    #[test]
    fn a_review_rule_recognises_a_folder_by_what_it_contains() {
        let set = RuleSet::compile(&[rule(
            "{id: x, decision: review, reason: r, match: {path_glob: '**', contains: [.marker]}}",
        )])
        .unwrap();
        assert!(set.find("a/b", "b", &[".marker"]).is_some());
        assert!(set.find("a/b", "b", &["marker"]).is_none());
    }

    #[test]
    fn contains_alone_is_rejected() {
        let err = RuleSet::compile(&[rule(
            "{id: x, decision: leave, reason: r, match: {contains: [a]}}",
        )]);
        assert!(err.is_err());
    }

    #[test]
    fn only_a_review_rule_may_propose_or_cut_into_parts() {
        for yaml in [
            "{id: x, decision: leave, propose: leave, reason: r, match: {name_in: [a]}}",
            "{id: x, decision: leave, name: A, reason: r, match: {name_in: [a]}}",
            "{id: x, decision: migrate, parts: p, reason: r, match: {name_in: [a]}}",
            "{id: x, decision: review, propose: review, reason: r, match: {name_in: [a]}}",
            "{id: x, decision: review, parts: p, propose: leave, reason: r, match: {name_in: [a]}}",
            "{id: x, decision: leave, fold_small: true, reason: r, match: {name_in: [a]}}",
            "{id: x, decision: review, fold_small: true, name: A, reason: r, match: {name_in: [a]}}",
            "{id: x, decision: review, fold_small: true, parts: p, reason: r, match: {name_in: [a]}}",
            "{id: x, decision: review, fold_small: true, program: folder, reason: r, match: {name_in: [a]}}",
        ] {
            assert!(RuleSet::compile(&[rule(yaml)]).is_err(), "{yaml}");
        }
        for yaml in [
            "{id: x, decision: leave, program: each, reason: r, match: {name_in: [a]}}",
            "{id: x, decision: review, name: A, program: folder, reason: r, match: {name_in: [a]}}",
            "{id: x, decision: review, name: A, propose: leave, reason: r, match: {name_in: [a]}}",
            "{id: x, decision: review, parts: p, reason: r, match: {name_in: [a]}}",
            "{id: x, decision: review, fold_small: true, reason: r, match: {name_in: [a]}}",
        ] {
            assert!(RuleSet::compile(&[rule(yaml)]).is_ok(), "{yaml}");
        }
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let parsed: std::result::Result<Rule, _> = serde_saphyr::from_str(
            "{id: x, decision: leave, reason: r, match: {name_in: [a]}, run: 'del /s'}",
        );
        assert!(parsed.is_err());
    }

    #[test]
    fn text_falls_back_to_english() {
        let text: Text = serde_saphyr::from_str("{en: Hello, fr: Bonjour}").unwrap();
        assert_eq!(text.get("fr"), "Bonjour");
        assert_eq!(text.get("de"), "Hello");
    }
}
