//! What the signature of a review unit says. A signature is what an
//! arbiter reads about a folder, and what a person reads on a review card.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::collections::BTreeSet;
use std::path::Path;

use bellcart::plan::{Decision, SplitBudget};
use bellcart::scan::{Scan, scan};
use bellcart::signature::{MAX_CHARS, Signature, signatures};
use bellcart::tree::MemTree;
use common::small_budget;

fn scan_with(tree: &MemTree, budget: &SplitBudget) -> Scan {
    scan(tree, Path::new(""), &common::core_rules(), budget).expect("the scan succeeds")
}

fn signature_of(scan: &Scan, path: &str) -> String {
    signatures(scan)
        .into_iter()
        .find(|s| s.path == path)
        .map(|s| s.to_text())
        .unwrap_or_else(|| panic!("no signature for `{path}`"))
}

fn line<'a>(text: &'a str, key: &str) -> &'a str {
    text.lines()
        .find(|l| l.starts_with(key))
        .unwrap_or_else(|| panic!("no `{key}` line in:\n{text}"))
}

#[test]
fn a_signature_describes_the_unit() {
    let scan = scan_with(
        &MemTree::new()
            .file("Documents/report.pdf", 600)
            .file("Documents/cv.docx", 200),
        &SplitBudget::default(),
    );
    let text = signature_of(&scan, "");
    assert_eq!(line(&text, "path:"), "path: .");
    assert_eq!(
        line(&text, "unit:"),
        "unit: small enough to judge as a whole"
    );
    assert_eq!(line(&text, "size:"), "size: 800 B in 2 files");
    assert_eq!(line(&text, "extensions:"), "extensions: pdf 75%, docx 25%");
}

#[test]
fn a_signature_lists_the_largest_sub_folders_first() {
    let scan = scan_with(
        &MemTree::new()
            .file("a/x.txt", 300)
            .file("b/x.txt", 100)
            .file("c/x.txt", 500),
        &SplitBudget::default(),
    );
    let text = signature_of(&scan, "");
    assert_eq!(
        line(&text, "folders"),
        "folders (3): c 500 B, a 300 B, b 100 B"
    );
}

#[test]
fn a_signature_lists_the_largest_files_first() {
    let scan = scan_with(
        &MemTree::new()
            .file("small.txt", 10)
            .file("big.iso", 900)
            .file("mid.zip", 90),
        &SplitBudget::default(),
    );
    let text = signature_of(&scan, "");
    assert_eq!(
        line(&text, "files"),
        "files (3): big.iso 900 B, mid.zip 90 B, small.txt 10 B"
    );
}

#[test]
fn a_signature_stays_within_its_budget_however_many_children() {
    let mut tree = MemTree::new();
    for i in 0..3_000 {
        let long = "a-rather-long-folder-name-that-goes-on-and-on-and-on-and-on";
        tree = tree
            .file(&format!("{long}-{i}/data.bin"), 1_000 + i)
            .file(&format!("{long}-{i}.ext{i}"), 10 + i);
    }
    let scan = scan_with(&tree, &SplitBudget::default());
    let text = signature_of(&scan, "");
    assert!(text.chars().count() <= MAX_CHARS, "{} chars", text.len());
    assert!(line(&text, "folders").starts_with("folders (3000): "));
    assert!(line(&text, "folders").ends_with(" more"));
}

#[test]
fn long_names_are_shortened() {
    let name = "x".repeat(200);
    let scan = scan_with(
        &MemTree::new().file(&format!("{name}/a.txt"), 5),
        &SplitBudget::default(),
    );
    let text = signature_of(&scan, "");
    let folders = line(&text, "folders");
    assert!(folders.contains("..."));
    assert!(folders.len() < 100, "{folders}");
}

#[test]
fn the_same_unit_always_gives_the_same_signature() {
    let tree = || {
        MemTree::new()
            .file_at("Documents/a.pdf", 300, Some(1_000_000_000))
            .file("Documents/b/c.docx", 200)
            .file("Documents/d/e.txt", 200)
    };
    let first: Vec<String> = signatures(&scan_with(&tree(), &small_budget()))
        .iter()
        .map(Signature::to_text)
        .collect();
    let second: Vec<String> = signatures(&scan_with(&tree(), &small_budget()))
        .iter()
        .map(Signature::to_text)
        .collect();
    assert_eq!(first, second);
}

#[test]
fn a_signature_does_not_count_settled_folders_in_its_size_or_mix() {
    let scan = scan_with(
        &MemTree::new()
            .file("app/src/main.rs", 150)
            .file("app/node_modules/big.js", 50_000),
        &SplitBudget::default(),
    );
    let text = signature_of(&scan, "");
    assert_eq!(line(&text, "size:"), "size: 150 B in 1 file");
    assert_eq!(line(&text, "extensions:"), "extensions: rs 100%");
    assert_eq!(
        line(&text, "settled inside:"),
        "settled inside: app/node_modules (leave)"
    );
}

#[test]
fn a_rest_signature_does_not_list_sub_folders_that_are_their_own_units() {
    let scan = scan_with(
        &MemTree::new()
            .file("Downloads/setup.exe", 400)
            .file("Downloads/old/a.zip", 700)
            .file("Downloads/tiny/note.txt", 50),
        &small_budget(),
    );
    let text = signature_of(&scan, "Downloads");
    assert_eq!(
        line(&text, "unit:"),
        "unit: files directly in a folder that was cut, and its small sub-folders"
    );
    assert_eq!(line(&text, "folders"), "folders (1): tiny 50 B");
    assert_eq!(line(&text, "files"), "files (1): setup.exe 400 B");
}

#[test]
fn dates_are_shown_as_year_and_month() {
    // 2000-02-29 and 2001-09-09.
    let scan = scan_with(
        &MemTree::new()
            .file_at("a.txt", 1, Some(951_782_400))
            .file_at("b.txt", 1, Some(1_000_000_000)),
        &SplitBudget::default(),
    );
    let text = signature_of(&scan, "");
    assert_eq!(line(&text, "modified:"), "modified: 2000-02 to 2001-09");
}

#[test]
fn files_without_an_extension_and_small_shares_are_readable() {
    let scan = scan_with(
        &MemTree::new().file("README", 995).file("a.txt", 5),
        &SplitBudget::default(),
    );
    let text = signature_of(&scan, "");
    assert_eq!(
        line(&text, "extensions:"),
        "extensions: (none) 99%, txt <1%"
    );
}

#[test]
fn every_review_unit_has_a_signature() {
    let tree = MemTree::new()
        .file("Documents/taxes/2025.pdf", 300)
        .file("Pictures/2023/a.jpg", 2_000)
        .file("Downloads/setup.exe", 700)
        .file("Downloads/archive/old.zip", 900)
        .file("dev/app/node_modules/x.js", 5_000)
        .file(".ssh/id_ed25519", 400);
    let scan = scan_with(&tree, &small_budget());

    let units: BTreeSet<String> = scan
        .plan
        .nodes()
        .into_iter()
        .filter(|n| n.unit.is_some())
        .map(|n| n.path.clone())
        .collect();
    let signed: BTreeSet<String> = signatures(&scan).into_iter().map(|s| s.path).collect();
    assert_eq!(units, signed);
    assert!(
        scan.plan
            .nodes()
            .iter()
            .filter(|n| n.decision != Decision::Review)
            .all(|n| !signed.contains(&n.path))
    );
}

#[test]
fn signatures_come_largest_first() {
    let scan = scan_with(
        &MemTree::new()
            .file("small/a.pdf", 200)
            .file("large/b.zip", 800)
            .file("medium/c.mp3", 400),
        &small_budget(),
    );
    let order: Vec<String> = signatures(&scan).into_iter().map(|s| s.path).collect();
    assert_eq!(order, ["large", "medium", "small"]);
}
