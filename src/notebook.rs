//! The migration notebook: the human half of the output. What to do by
//! hand before leaving the old PC and on arrival on the new one, taken from
//! the recipes of the applications found, and the programs to install
//! again.
//!
//! The engine only gathers the tasks. Their wording, the language and the
//! format (Markdown, HTML) belong to the front ends.

use std::collections::BTreeMap;

use crate::pack::{Name, Recipe, Step, Text};
use crate::plan::{Plan, Reinstall, UnitReason};

/// The reason of the rule that recognises git repositories. The notebook
/// asks to check each one for work that exists only on this PC.
pub const GIT_REPOSITORY: &str = "git-repository";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Notebook {
    /// Before leaving the old PC.
    pub before: Vec<Chapter>,
    /// On arrival on the new PC.
    pub after: Vec<Chapter>,
    /// Programs found installed that the user has not decided about yet:
    /// to settle before leaving the old PC. By name, in plan order.
    pub undecided: Vec<String>,
    /// Programs the user wants on the new PC, to install again there.
    pub reinstall: Vec<String>,
    /// Git repositories found, by plan path: to check before leaving for
    /// changes not committed or pushed, which no server has.
    pub repositories: Vec<String>,
}

/// The tasks of one application, in the order of its recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    pub recipe: String,
    pub name: Name,
    pub tasks: Vec<Task>,
}

/// Something the user does, or checks. `skip` steps are not tasks: the plan
/// already leaves what they name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Task {
    /// Take this file or folder, as the recipe wrote it (`%APPDATA%\...`).
    Copy {
        path: String,
        note: Option<Text>,
    },
    Manual(Text),
    ExportWingetPackages,
    ExportVscodeExtensions,
    ExportRegistryKey {
        key: String,
    },
    Install {
        winget: String,
        winget_lang: BTreeMap<String, String>,
    },
}

impl Task {
    /// The winget package to install for a notebook in `lang`: the one
    /// published in that language when the recipe names it.
    pub fn winget<'a>(
        winget: &'a str,
        winget_lang: &'a BTreeMap<String, String>,
        lang: &str,
    ) -> &'a str {
        winget_lang.get(lang).map_or(winget, String::as_str)
    }
}

impl Notebook {
    pub fn is_empty(&self) -> bool {
        self.before.is_empty()
            && self.after.is_empty()
            && self.undecided.is_empty()
            && self.reinstall.is_empty()
            && self.repositories.is_empty()
    }
}

/// The notebook for a plan: the tasks of the applications found, in their
/// order, from the pack's `recipes`, and the programs to install again.
pub fn notebook(plan: &Plan, recipes: &[Recipe]) -> Notebook {
    let mut out = Notebook::default();
    for program in &plan.programs {
        match program.reinstall {
            Reinstall::Undecided => out.undecided.push(program.name.clone()),
            Reinstall::Yes => out.reinstall.push(program.name.clone()),
            Reinstall::No => {}
        }
    }
    out.repositories = plan
        .nodes()
        .into_iter()
        .filter(|node| {
            matches!(&node.unit, Some(UnitReason::Rule { key, .. }) if key == GIT_REPOSITORY)
        })
        .map(|node| node.path.clone())
        .collect();
    for application in &plan.applications {
        let Some(recipe) = recipes.iter().find(|r| r.id == application.recipe) else {
            continue;
        };
        for (steps, chapters) in [
            (&recipe.before, &mut out.before),
            (&recipe.after, &mut out.after),
        ] {
            let tasks: Vec<Task> = steps.iter().filter_map(task).collect();
            if !tasks.is_empty() {
                chapters.push(Chapter {
                    recipe: recipe.id.clone(),
                    name: recipe.name.clone(),
                    tasks,
                });
            }
        }
    }
    out
}

fn task(step: &Step) -> Option<Task> {
    Some(match step {
        Step::Copy { path, note } => Task::Copy {
            path: path.clone(),
            note: note.clone(),
        },
        Step::Skip { .. } => return None,
        Step::Manual { text } => Task::Manual(text.clone()),
        Step::ExportWingetPackages => Task::ExportWingetPackages,
        Step::ExportVscodeExtensions => Task::ExportVscodeExtensions,
        Step::ExportRegistryKey { key } => Task::ExportRegistryKey { key: key.clone() },
        Step::Install {
            winget,
            winget_lang,
        } => Task::Install {
            winget: winget.clone(),
            winget_lang: winget_lang.clone(),
        },
    })
}
