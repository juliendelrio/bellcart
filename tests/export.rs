//! Which exports Bellcart offers to run, and what a pack may ask for.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::path::Path;

use bellcart::export::{Export, exports};
use bellcart::pack::Pack;
use bellcart::recipe::{Recipes, Variables};
use bellcart::tree::MemTree;

fn offered(tree: &MemTree) -> Vec<Export> {
    let pack = common::core_pack();
    let found = Recipes::detect(tree, Path::new(""), &pack.recipes, &Variables::profile());
    exports(found.applications(), &pack.recipes)
}

fn pack_with_recipe(recipe: &str) -> bellcart::Result<Pack> {
    let meta = std::fs::read_to_string(common::repo_root().join("packs/core/pack.yaml")).unwrap();
    Pack::from_files([("pack.yaml", meta.as_str()), ("recipes/tool.yaml", recipe)])
}

fn registry_recipe(key: &str) -> String {
    format!(
        "id: tool\nname: Tool\ndetect:\n  any:\n    - path: '%APPDATA%\\Tool'\nbefore:\n  - type: export-registry-key\n    key: '{key}'\n"
    )
}

#[test]
fn exports_are_offered_for_the_applications_found() {
    let tree = MemTree::new()
        .file("AppData/Roaming/Code/User/settings.json", 1)
        .link("AppData/Local/Microsoft/WindowsApps/winget.exe");
    assert_eq!(
        offered(&tree),
        [Export::VscodeExtensions, Export::WingetPackages]
    );
}

#[test]
fn plex_saves_its_settings_from_the_registry() {
    let tree = MemTree::new().dir("AppData/Local/Plex Media Server/Plug-in Support/Databases");
    assert_eq!(
        offered(&tree),
        [Export::RegistryKey(
            r"HKCU\Software\Plex, Inc.\Plex Media Server".to_owned()
        )]
    );
}

#[test]
fn no_export_is_offered_when_no_application_asks_for_one() {
    assert!(offered(&MemTree::new().file(".gitconfig", 1)).is_empty());
}

#[test]
fn each_export_is_offered_once() {
    let twice = "id: tool\nname: Tool\ndetect:\n  any:\n    - path: '%APPDATA%\\Tool'\nbefore:\n  - type: export-winget-packages\nafter:\n  - type: export-winget-packages\n";
    let pack = pack_with_recipe(twice).expect("the recipe loads");
    let tree = MemTree::new().file("AppData/Roaming/Tool/a", 1);
    let found = Recipes::detect(&tree, Path::new(""), &pack.recipes, &Variables::profile());
    assert_eq!(
        exports(found.applications(), &pack.recipes),
        [Export::WingetPackages]
    );
}

fn install_recipe(step: &str) -> String {
    format!(
        "id: tool\nname: Tool\ndetect:\n  any:\n    - path: '%APPDATA%\\Tool'\nafter:\n  - type: install\n{step}"
    )
}

#[test]
fn a_recipe_may_install_a_package_per_language() {
    let step = "    winget: Mozilla.Firefox\n    winget_lang:\n      fr: Mozilla.Firefox.fr\n      pt-br: Mozilla.Firefox.pt-BR\n";
    assert!(pack_with_recipe(&install_recipe(step)).is_ok());
}

#[test]
fn a_winget_identifier_is_never_more_than_an_identifier() {
    // The notebook shows it inside a command to type.
    for id in [
        "Git.Git & del x",
        "\"Git.Git\"",
        "Git.Git;calc",
        "--source",
        "Git",
    ] {
        let step = format!("    winget: '{id}'\n");
        let error = pack_with_recipe(&install_recipe(&step)).expect_err(id);
        assert!(
            error.to_string().contains("winget identifier"),
            "{id}: {error}"
        );
        let step = format!("    winget: Git.Git\n    winget_lang:\n      fr: '{id}'\n");
        let error = pack_with_recipe(&install_recipe(&step)).expect_err(id);
        assert!(
            error.to_string().contains("winget identifier"),
            "{id}: {error}"
        );
    }
}

#[test]
fn a_language_of_a_winget_package_is_a_language_code() {
    let step = "    winget: Git.Git\n    winget_lang:\n      'fr fr': Git.Git\n";
    let error = pack_with_recipe(&install_recipe(step)).expect_err("a space");
    assert!(error.to_string().contains("language code"), "{error}");
}

#[test]
fn a_recipe_may_export_a_key_of_the_current_user() {
    assert!(pack_with_recipe(&registry_recipe(r"HKCU\Software\SimonTatham\PuTTY")).is_ok());
    assert!(
        pack_with_recipe(&registry_recipe(
            r"HKCU\Software\Plex, Inc.\Plex Media Server"
        ))
        .is_ok()
    );
}

#[test]
fn a_recipe_may_not_export_other_registry_keys() {
    for key in [
        r"HKLM\SOFTWARE\Microsoft",
        r"HKCU\Software\a&calc",
        r"HKCU\Software\..\x",
    ] {
        let error = pack_with_recipe(&registry_recipe(key)).expect_err(key);
        assert!(error.to_string().contains("registry key"), "{key}: {error}");
    }
}
