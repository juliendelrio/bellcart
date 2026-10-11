//! The migration notebook: the human half of the output. What to do by
//! hand before leaving the old PC and on arrival on the new one, taken from
//! the recipes of the applications found.
//!
//! The engine only gathers the tasks. Their wording, the language and the
//! format (Markdown, HTML) belong to the front ends.

use crate::pack::{Recipe, Step, Text};
use crate::plan::Application;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Notebook {
    /// Before leaving the old PC.
    pub before: Vec<Chapter>,
    /// On arrival on the new PC.
    pub after: Vec<Chapter>,
}

/// The tasks of one application, in the order of its recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    pub recipe: String,
    pub name: String,
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
    },
}

impl Notebook {
    pub fn is_empty(&self) -> bool {
        self.before.is_empty() && self.after.is_empty()
    }
}

/// The notebook for the `applications` found, in their order, from the
/// pack's `recipes`.
pub fn notebook(applications: &[Application], recipes: &[Recipe]) -> Notebook {
    let mut out = Notebook::default();
    for application in applications {
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
        Step::Install { winget } => Task::Install {
            winget: winget.clone(),
        },
    })
}
