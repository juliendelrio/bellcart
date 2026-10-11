//! End-to-end checks on the real binary.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::process::Command;

use bellcart::plan::{Decision, Plan};

fn bellcart(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_bellcart"))
        .args(args)
        .output()
        .expect("bellcart runs")
}

#[test]
fn scan_prints_a_plan_other_tools_can_read() {
    // The repository's own `packs` folder is a convenient real directory.
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/packs");
    let output = bellcart(&["scan", dir, "--json"]);
    assert!(output.status.success());

    let plan: Plan = serde_json::from_slice(&output.stdout).expect("valid plan JSON");
    assert_eq!(plan.tree.decision, Decision::Review);
    assert!(plan.totals[&Decision::Review].files > 0);
}

#[test]
fn scan_prints_signatures_when_asked() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/packs");
    let output = bellcart(&["scan", dir, "--signatures"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("path: "), "{stdout}");
    assert!(stdout.contains("\nextensions: yaml "), "{stdout}");
}

#[test]
fn the_interface_speaks_french_when_asked() {
    let output = bellcart(&["--lang", "fr", "packs"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("recettes"));
}

#[test]
fn a_missing_folder_is_a_clean_error() {
    let output = bellcart(&["scan", "this-folder-does-not-exist"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("bellcart: cannot read"));
    assert_eq!(
        stderr.lines().count(),
        1,
        "no backtrace, one line: {stderr}"
    );
}

#[test]
fn one_job_gives_the_same_plan_as_the_default() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/packs");
    let default = bellcart(&["scan", dir, "--json"]);
    let sequential = bellcart(&["scan", dir, "--json", "--jobs", "1"]);
    assert!(sequential.status.success());
    assert_eq!(sequential.stdout, default.stdout);
}

#[test]
fn zero_jobs_is_refused() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/packs");
    let output = bellcart(&["scan", dir, "--jobs", "0"]);
    assert!(!output.status.success());
}

#[test]
fn no_progress_is_printed_when_the_output_is_not_a_terminal() {
    // Tests capture stderr, so it is not a terminal here.
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/packs");
    let output = bellcart(&["scan", dir, "--json"]);
    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_reader_that_stops_early_is_not_an_error() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/packs");
    let mut child = Command::new(env!("CARGO_BIN_EXE_bellcart"))
        .args(["scan", dir, "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("bellcart starts");
    // Close the reading end before bellcart writes anything.
    drop(child.stdout.take());
    let output = child.wait_with_output().expect("bellcart ends");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
}

/// A fresh folder under Cargo's temporary directory for integration tests.
fn fresh_dir(name: &str) -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temporary folder");
    dir
}

/// A small profile with Git and SSH keys, and a folder to write outputs to.
fn profile_and_outputs(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let base = fresh_dir(name);
    let profile = base.join("profile");
    std::fs::create_dir_all(profile.join(".ssh")).unwrap();
    std::fs::write(profile.join(".gitconfig"), "[user]\n").unwrap();
    std::fs::write(profile.join(".ssh/id_ed25519"), "key").unwrap();
    let outputs = base.join("outputs");
    std::fs::create_dir_all(&outputs).unwrap();
    (profile, outputs)
}

#[test]
fn scan_writes_a_markdown_notebook_in_the_chosen_language() {
    let (profile, outputs) = profile_and_outputs("notebook-md");
    let file = outputs.join("carnet.md");
    let output = bellcart(&[
        "--lang",
        "fr",
        "scan",
        profile.to_str().unwrap(),
        "--notebook",
        file.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let book = std::fs::read_to_string(&file).expect("the notebook is written");
    assert!(book.starts_with("# Carnet de migration pour "), "{book}");
    assert!(
        book.contains("\n## Avant de quitter l'ancien PC\n"),
        "{book}"
    );
    assert!(book.contains("\n### Git\n"), "{book}");
    assert!(
        book.contains("- [ ] Emportez `%USERPROFILE%\\.gitconfig`\n"),
        "{book}"
    );
    assert!(
        book.contains("- [ ] Installez-la avec winget : `winget install --id Git.Git -e`"),
        "{book}"
    );
}

#[test]
fn scan_writes_an_html_notebook_with_checkboxes() {
    let (profile, outputs) = profile_and_outputs("notebook-html");
    let file = outputs.join("notebook.html");
    let output = bellcart(&[
        "--lang",
        "en",
        "scan",
        profile.to_str().unwrap(),
        "--notebook",
        file.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let book = std::fs::read_to_string(&file).expect("the notebook is written");
    assert!(
        book.starts_with("<!doctype html>\n<html lang=\"en\">"),
        "{book}"
    );
    assert!(
        book.contains("<h2>Before you leave the old PC</h2>"),
        "{book}"
    );
    assert!(
        book.contains("<input type=\"checkbox\"> Take <code>%USERPROFILE%\\.gitconfig</code>"),
        "{book}"
    );
}

#[test]
fn a_notebook_with_no_application_says_so() {
    let base = fresh_dir("notebook-empty");
    let profile = base.join("profile");
    std::fs::create_dir_all(profile.join("Documents")).unwrap();
    std::fs::write(profile.join("Documents/a.txt"), "a").unwrap();
    let file = base.join("notebook.md");
    let output = bellcart(&[
        "--lang",
        "en",
        "scan",
        profile.to_str().unwrap(),
        "--notebook",
        file.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let book = std::fs::read_to_string(&file).unwrap();
    assert!(
        book.contains("No application with a recipe was found"),
        "{book}"
    );
}

#[test]
fn no_output_is_written_inside_the_scanned_folder() {
    let (profile, _) = profile_and_outputs("inside");
    for option in ["--out", "--notebook"] {
        let file = profile.join("plan.json");
        let output = bellcart(&[
            "scan",
            profile.to_str().unwrap(),
            option,
            file.to_str().unwrap(),
        ]);
        assert!(
            !output.status.success(),
            "{option} inside the scan must fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("inside the scanned folder"), "{stderr}");
        assert!(!profile.join("plan.json").exists(), "{option} wrote a file");
    }
}

#[test]
fn an_output_next_to_the_scanned_folder_is_fine() {
    let (profile, outputs) = profile_and_outputs("outside");
    let file = outputs.join("plan.json");
    let output = bellcart(&[
        "scan",
        profile.to_str().unwrap(),
        "--out",
        file.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plan: Plan = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    let found: Vec<&str> = plan
        .applications
        .iter()
        .map(|a| a.recipe.as_str())
        .collect();
    assert_eq!(found, ["git", "openssh"]);
}

/// Writes a pack folder from `(relative path, content)` pairs.
fn pack_folder(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = fresh_dir(name);
    let meta = "id: extra\nname: Extra pack\nversion: 0.1.0\nformat: 1\nlicense: Apache-2.0\n";
    std::fs::write(dir.join("pack.yaml"), meta).unwrap();
    for (path, content) in files {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, content).unwrap();
    }
    dir
}

const BUILD_CACHE_RULE: &str = "rules:\n  - id: tool-build-cache\n    decision: leave\n    reason: build-output\n    match:\n      name_in: [.toolcache]\n      contains: [TOOLCACHE.TAG]\n";

#[test]
fn a_pack_from_disk_is_listed_after_the_built_in_one() {
    let pack = pack_folder("pack-listed", &[("rules/cache.yaml", BUILD_CACHE_RULE)]);
    let output = bellcart(&["--lang", "en", "--pack", pack.to_str().unwrap(), "packs"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let core = stdout
        .find("Bellcart core pack")
        .expect("the core pack is listed");
    let extra = stdout.find("Extra pack").expect("the extra pack is listed");
    assert!(core < extra, "{stdout}");
}

#[test]
fn a_pack_from_disk_adds_its_rules_to_the_scan() {
    let pack = pack_folder("pack-rules", &[("rules/cache.yaml", BUILD_CACHE_RULE)]);
    let base = fresh_dir("pack-rules-profile");
    std::fs::create_dir_all(base.join("dev/.toolcache")).unwrap();
    std::fs::write(base.join("dev/.toolcache/TOOLCACHE.TAG"), "x").unwrap();
    let output = bellcart(&[
        "--pack",
        pack.to_str().unwrap(),
        "scan",
        base.to_str().unwrap(),
        "--json",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plan: Plan = serde_json::from_slice(&output.stdout).unwrap();
    let cache = plan
        .nodes()
        .into_iter()
        .find(|n| n.path == "dev/.toolcache")
        .expect("the extra rule settles the folder");
    assert_eq!(cache.decision, Decision::Leave);
}

#[test]
fn a_pack_from_disk_cannot_change_what_a_core_rule_settles() {
    let rule = "rules:\n  - id: careless-keys\n    decision: leave\n    reason: temporary-files\n    match:\n      name_in: [.ssh]\n";
    let pack = pack_folder("pack-careless", &[("rules/keys.yaml", rule)]);
    let base = fresh_dir("pack-careless-profile");
    std::fs::create_dir_all(base.join(".ssh")).unwrap();
    std::fs::write(base.join(".ssh/id_ed25519"), "key").unwrap();
    let output = bellcart(&[
        "--pack",
        pack.to_str().unwrap(),
        "scan",
        base.to_str().unwrap(),
        "--json",
    ]);
    assert!(output.status.success());
    let plan: Plan = serde_json::from_slice(&output.stdout).unwrap();
    let keys = plan.nodes().into_iter().find(|n| n.path == ".ssh").unwrap();
    assert_eq!(keys.decision, Decision::Migrate);
}

#[test]
fn an_invalid_pack_names_its_folder_file_and_field() {
    let rule = "rules:\n  - id: typo\n    decision: leave\n    reason: build-output\n    mach:\n      name_in: [x]\n";
    let pack = pack_folder("pack-invalid", &[("rules/typo.yaml", rule)]);
    let output = bellcart(&["--pack", pack.to_str().unwrap(), "packs"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot load the pack in"), "{stderr}");
    assert!(stderr.contains("rules/typo.yaml"), "{stderr}");
    assert!(stderr.contains("mach"), "{stderr}");
}

#[test]
fn a_pack_cannot_reuse_a_rule_id_of_the_core_pack() {
    let rule = "rules:\n  - id: ssh-keys\n    decision: migrate\n    reason: keys-and-credentials\n    match:\n      name_in: [.keys]\n";
    let pack = pack_folder("pack-duplicate", &[("rules/keys.yaml", rule)]);
    let output = bellcart(&["--pack", pack.to_str().unwrap(), "packs"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ssh-keys") && stderr.contains("core"),
        "{stderr}"
    );
}
