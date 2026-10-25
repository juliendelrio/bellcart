//! What the migration notebook asks the user to do.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::path::Path;

use bellcart::notebook::{Chapter, Notebook, Task, notebook};
use bellcart::plan::SplitBudget;
use bellcart::recipe::{Recipes, Variables};
use bellcart::scan::{Progress, scan_with};
use bellcart::tree::MemTree;

fn notebook_for(tree: &MemTree) -> Notebook {
    let pack = common::core_pack();
    let found = Recipes::detect(tree, Path::new(""), &pack.recipes, &Variables::profile());
    let scan = scan_with(
        tree,
        Path::new(""),
        &common::core_rules(),
        &found,
        &SplitBudget::default(),
        &Progress::default(),
    )
    .expect("the scan succeeds");
    notebook(&scan.plan.applications, &pack.recipes)
}

fn names(chapters: &[Chapter]) -> Vec<&str> {
    chapters.iter().map(|c| c.recipe.as_str()).collect()
}

#[test]
fn the_notebook_has_a_chapter_per_application_found() {
    let book = notebook_for(
        &MemTree::new()
            .file(".gitconfig", 1)
            .file(".ssh/id_ed25519", 1),
    );
    assert_eq!(names(&book.before), ["git", "openssh"]);
    assert_eq!(names(&book.after), ["git", "openssh"]);
}

#[test]
fn applications_not_found_are_not_in_the_notebook() {
    let book = notebook_for(&MemTree::new().file("Documents/a.txt", 1));
    assert!(book.is_empty());
}

#[test]
fn tasks_follow_the_order_of_the_recipe() {
    let book = notebook_for(&MemTree::new().file(".gitconfig", 1));
    let git = &book.before[0];
    assert!(
        matches!(&git.tasks[0], Task::Copy { path, .. } if path == r"%USERPROFILE%\.gitconfig")
    );
    assert!(matches!(&git.tasks[1], Task::Manual(text) if text.en.starts_with("Push")));
    assert!(matches!(&book.after[0].tasks[0], Task::Install { winget } if winget == "Git.Git"));
}

#[test]
fn skip_steps_are_not_tasks() {
    let book = notebook_for(&MemTree::new().file("AppData/Roaming/Code/User/settings.json", 1));
    let vscode = &book.before[0];
    assert_eq!(vscode.recipe, "vscode");
    assert!(
        vscode
            .tasks
            .iter()
            .all(|t| !matches!(t, Task::Copy { path, .. } if path.contains("Cache")))
    );
    assert!(vscode.tasks.contains(&Task::ExportVscodeExtensions));
}

#[test]
fn extensions_saved_before_leaving_are_installed_again_on_arrival() {
    let book = notebook_for(&MemTree::new().file("AppData/Roaming/Code/User/settings.json", 1));
    let arrival = &book.after[0];
    assert_eq!(arrival.recipe, "vscode");
    assert!(arrival.tasks.iter().any(|t| matches!(
        t,
        Task::Manual(text) if text.en.contains("code --install-extension")
            && text.en.contains("vscode-extensions.txt")
    )));
}

#[test]
fn an_application_without_steps_on_one_side_has_no_chapter_there() {
    // The winget recipe only exports before, and only explains after.
    let book = notebook_for(&MemTree::new().link("AppData/Local/Microsoft/WindowsApps/winget.exe"));
    assert_eq!(names(&book.before), ["winget-apps"]);
    assert_eq!(book.before[0].tasks, [Task::ExportWingetPackages]);
    assert_eq!(names(&book.after), ["winget-apps"]);
}

#[test]
fn leaving_the_google_drive_cache_comes_with_a_check_before_leaving() {
    // The cache is left by a rule; the notebook must ask to check that
    // nothing is still waiting to upload.
    let book = notebook_for(&MemTree::new().file("AppData/Local/Google/DriveFS/1234/metadata", 1));
    assert_eq!(names(&book.before), ["google-drive"]);
    assert!(
        matches!(&book.before[0].tasks[0], Task::Manual(text) if text.en.contains("up to date"))
    );
    assert!(book.after[0].tasks.contains(&Task::Install {
        winget: "Google.GoogleDrive".to_owned()
    }));
}
