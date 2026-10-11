//! Recipes on a scanned profile: which applications are there, and which
//! paths their `copy` and `skip` steps settle.
//!
//! Recipe paths start with a `%VARIABLE%`. Variables are resolved against
//! the scanned profile, never against the running process: an elevated
//! Bellcart may run as another account than the one whose profile it
//! reads. Detection only lists folders, like the scan: no file is opened.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::pack::{Recipe, Step};
use crate::plan::{Application, Decision};
use crate::tree::{Kind, Tree};

/// Where each variable points, relative to the scan root, which is taken
/// to be the user profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variables {
    /// Uppercase name to a path with `/` separators.
    map: BTreeMap<String, String>,
}

impl Variables {
    /// The standard layout of a Windows profile. Redirected known folders
    /// are resolved by identifier in the Windows crate (milestone M5).
    pub fn profile() -> Self {
        let map = [
            ("USERPROFILE", ""),
            ("APPDATA", "AppData/Roaming"),
            ("LOCALAPPDATA", "AppData/Local"),
        ]
        .into_iter()
        .map(|(name, path)| (name.to_owned(), path.to_owned()))
        .collect();
        Self { map }
    }

    /// Turns `%APPDATA%\Code\User` into `AppData/Roaming/Code/User`.
    ///
    /// `None` when the path does not start with a known variable, or climbs
    /// out of it with `..`: a recipe must not reach outside the profile.
    pub fn expand(&self, path: &str) -> Option<String> {
        let (name, tail) = path.strip_prefix('%')?.split_once('%')?;
        let base = self.map.get(&name.to_ascii_uppercase())?;
        let mut parts: Vec<&str> = base.split('/').filter(|p| !p.is_empty()).collect();
        for part in tail.split(['\\', '/']) {
            match part {
                "" | "." => {}
                ".." => return None,
                part => parts.push(part),
            }
        }
        Some(parts.join("/"))
    }
}

/// Checks a recipe path when a pack is loaded.
pub fn check_path(path: &str) -> Result<String, String> {
    Variables::profile().expand(path).ok_or_else(|| {
        format!(
            "`{path}` must start with %USERPROFILE%, %APPDATA% or %LOCALAPPDATA% \
             and must not contain `..`"
        )
    })
}

/// Checks the path of a `skip` step. A skip leaves what it names behind,
/// so it may only point inside an application's own data, never at the
/// user's folders nor at a whole `AppData` folder.
pub fn check_skip(path: &str) -> Result<(), String> {
    let expanded = check_path(path)?;
    let inside = |base: &str| {
        expanded
            .strip_prefix(base)
            .is_some_and(|rest| rest.starts_with('/') && rest.len() > 1)
    };
    if inside("AppData/Roaming") || inside("AppData/Local") {
        Ok(())
    } else {
        Err(format!(
            "a skip must point inside %APPDATA% or %LOCALAPPDATA%, not `{path}`"
        ))
    }
}

/// A path settled by a recipe step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    pub decision: Decision,
    pub recipe: String,
}

/// The recipes of the applications found on a profile.
#[derive(Debug, Clone, Default)]
pub struct Recipes {
    applications: Vec<Application>,
    /// Keyed by lowercase path, since the file system ignores case.
    steps: HashMap<String, Planned>,
}

impl Recipes {
    /// Runs every recipe's probes against the tree, and keeps the steps of
    /// the applications found.
    ///
    /// When two recipes settle the same path, `copy` wins over `skip`: when
    /// in doubt, keep. Recipes are taken in id order, so the result does not
    /// depend on the order of the files in the pack.
    pub fn detect(tree: &dyn Tree, root: &Path, recipes: &[Recipe], vars: &Variables) -> Self {
        let mut sorted: Vec<&Recipe> = recipes.iter().collect();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));

        let mut found = Self::default();
        for recipe in sorted {
            let present = recipe.detect.any.iter().any(|probe| {
                vars.expand(&probe.path)
                    .is_some_and(|rel| exists(tree, root, &rel))
            });
            if !present {
                continue;
            }
            found.applications.push(Application {
                recipe: recipe.id.clone(),
                name: recipe.name.clone(),
            });
            for step in recipe.before.iter().chain(&recipe.after) {
                let (path, decision) = match step {
                    Step::Copy { path, .. } => (path, Decision::Migrate),
                    Step::Skip { path } => (path, Decision::Leave),
                    _ => continue,
                };
                let Some(rel) = vars.expand(path) else {
                    continue;
                };
                let planned = Planned {
                    decision,
                    recipe: recipe.id.clone(),
                };
                found
                    .steps
                    .entry(rel.to_lowercase())
                    .and_modify(|current| {
                        if keeps_more(planned.decision, current.decision) {
                            *current = planned.clone();
                        }
                    })
                    .or_insert(planned);
            }
        }
        found
    }

    /// The applications found, by recipe id.
    pub fn applications(&self) -> &[Application] {
        &self.applications
    }

    /// The recipe step settling exactly this path, if any.
    pub fn at(&self, rel: &str) -> Option<&Planned> {
        if self.steps.is_empty() {
            return None;
        }
        self.steps.get(&rel.to_lowercase())
    }
}

/// `true` if `a` keeps more of the data than `b`. The order follows the
/// cost of a mistake: leaving what was needed loses it, migrating what was
/// not only costs space.
pub fn keeps_more(a: Decision, b: Decision) -> bool {
    rank(a) > rank(b)
}

fn rank(decision: Decision) -> u8 {
    match decision {
        Decision::Leave => 0,
        Decision::Review => 1,
        Decision::Archive => 2,
        Decision::Migrate => 3,
    }
}

/// Whether `rel` exists under `root`, comparing names without case.
fn exists(tree: &dyn Tree, root: &Path, rel: &str) -> bool {
    let parts: Vec<&str> = rel.split('/').filter(|p| !p.is_empty()).collect();
    let Some((last, parents)) = parts.split_last() else {
        return true;
    };
    let mut dir = root.to_path_buf();
    for part in parents {
        let Some(name) = find(tree, &dir, part, true) else {
            return false;
        };
        dir.push(name);
    }
    find(tree, &dir, last, false).is_some()
}

/// The on-disk name of `name` in `dir`, if it is there.
fn find(tree: &dyn Tree, dir: &Path, name: &str, dir_only: bool) -> Option<String> {
    let wanted = name.to_lowercase();
    tree.read_dir(dir)
        .ok()?
        .into_iter()
        .find(|e| e.name.to_lowercase() == wanted && (!dir_only || e.kind == Kind::Dir))
        .map(|e| e.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variables_expand_relative_to_the_profile() {
        let vars = Variables::profile();
        assert_eq!(
            vars.expand(r"%APPDATA%\Code\User").as_deref(),
            Some("AppData/Roaming/Code/User")
        );
        assert_eq!(
            vars.expand(r"%localappdata%\Temp").as_deref(),
            Some("AppData/Local/Temp")
        );
        assert_eq!(
            vars.expand(r"%USERPROFILE%\.gitconfig").as_deref(),
            Some(".gitconfig")
        );
        assert_eq!(vars.expand("%USERPROFILE%").as_deref(), Some(""));
    }

    #[test]
    fn paths_outside_the_profile_do_not_expand() {
        let vars = Variables::profile();
        assert_eq!(vars.expand(r"C:\Windows"), None);
        assert_eq!(vars.expand(r"%WINDIR%\System32"), None);
        assert_eq!(vars.expand(r"%APPDATA%\..\..\Documents"), None);
        assert_eq!(vars.expand("%APPDATA"), None);
        assert_eq!(vars.expand(r"Documents\x"), None);
    }

    #[test]
    fn skips_must_stay_inside_application_data() {
        assert!(check_skip(r"%APPDATA%\Code\Cache").is_ok());
        assert!(check_skip(r"%LOCALAPPDATA%\App\Cache").is_ok());
        assert!(check_skip(r"%USERPROFILE%\Documents").is_err());
        assert!(check_skip(r"%USERPROFILE%\AppData\Roaming\X").is_ok());
        assert!(check_skip("%APPDATA%").is_err());
        assert!(check_skip(r"%APPDATA%\").is_err());
        assert!(check_skip(r"%USERPROFILE%\AppData\RoamingX\y").is_err());
    }

    #[test]
    fn keeping_ranks_follow_the_cost_of_a_mistake() {
        assert!(keeps_more(Decision::Migrate, Decision::Leave));
        assert!(keeps_more(Decision::Archive, Decision::Leave));
        assert!(keeps_more(Decision::Migrate, Decision::Archive));
        assert!(!keeps_more(Decision::Leave, Decision::Leave));
    }
}
