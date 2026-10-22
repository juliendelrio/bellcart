//! What the signature of a review unit says. A signature is what an
//! arbiter reads about a folder, and what a person reads on a review card.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::collections::BTreeSet;
use std::path::Path;

use bellcart::plan::{Decision, SplitBudget, Stats, UnitReason};
use bellcart::scan::{Scan, scan};
use bellcart::signature::{MAX_CHARS, Named, Privacy, Sample, Signature, signatures};
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

#[test]
fn files_of_a_folded_rest_are_named_with_their_folder() {
    let scan = scan_with(
        &MemTree::new()
            .file("Downloads/desktop.ini", 5)
            .file("Downloads/old/a.zip", 700)
            .file("Downloads/new/b.mp4", 700)
            .file("notes.txt", 20),
        &small_budget(),
    );
    let text = signature_of(&scan, "");
    assert_eq!(
        line(&text, "files"),
        "files (2): notes.txt 20 B, Downloads/desktop.ini 5 B"
    );
}

// Privacy: what a signature says once redacted for a cloud endpoint.

fn redacted_of(scan: &Scan, path: &str) -> String {
    signatures(scan)
        .into_iter()
        .find(|s| s.path == path)
        .map(|s| s.for_privacy(Privacy::Redacted).to_text())
        .unwrap_or_else(|| panic!("no signature for `{path}`"))
}

/// A profile with personal names in several places.
fn personal_profile() -> MemTree {
    MemTree::new()
        .file_at(
            "Documents/Wedding Smith/guests.xlsx",
            300,
            Some(1_000_000_000),
        )
        .file("Documents/Wedding Smith/vows.docx", 200)
        .file("Documents/Wedding Smith/site/node_modules/x.js", 50_000)
        .file("Documents/Wedding Smith/photos/beach.jpg", 900)
        .file("Johnson Family Tree/tree.ged", 400)
        .file("AppData/Roaming/Slack/storage/root-state.json", 300)
}

#[test]
fn full_privacy_sends_the_signature_as_built() {
    let scan = scan_with(&personal_profile(), &small_budget());
    for signature in signatures(&scan) {
        assert_eq!(signature.for_privacy(Privacy::Full), signature);
    }
}

#[test]
fn redaction_masks_names_under_personal_folders() {
    let scan = scan_with(&personal_profile(), &small_budget());
    let text = redacted_of(&scan, "Documents/Wedding Smith");
    assert_eq!(line(&text, "path:"), "path: Documents/<1>");
    assert_eq!(
        line(&text, "files"),
        "files (2): <3>.xlsx 300 B, <4>.docx 200 B"
    );
    // `photos` is a unit of its own; what is left is `site`, empty once
    // its `node_modules` is set aside, and masked the same way below.
    assert_eq!(line(&text, "folders"), "folders (1): <2> 0 B");
    // The folder a rule settled keeps the name the rule matched.
    assert_eq!(
        line(&text, "settled inside:"),
        "settled inside: <2>/node_modules (leave)"
    );
}

#[test]
fn redaction_keeps_sizes_dates_and_extensions() {
    let scan = scan_with(&personal_profile(), &small_budget());
    let full = signature_of(&scan, "Documents/Wedding Smith");
    let redacted = redacted_of(&scan, "Documents/Wedding Smith");
    for key in ["size:", "modified:", "extensions:", "unit:"] {
        assert_eq!(line(&redacted, key), line(&full, key), "{key}");
    }
}

#[test]
fn a_folder_the_user_made_at_the_profile_root_is_masked_too() {
    let scan = scan_with(&personal_profile(), &small_budget());
    let text = redacted_of(&scan, "Johnson Family Tree");
    assert_eq!(line(&text, "path:"), "path: <1>");
    assert_eq!(line(&text, "files"), "files (1): <2>.ged 400 B");
}

#[test]
fn application_paths_stay_in_clear() {
    let scan = scan_with(&personal_profile(), &small_budget());
    let text = redacted_of(&scan, "AppData");
    assert_eq!(text, signature_of(&scan, "AppData"));
    assert!(text.contains("Roaming"), "{text}");
}

#[test]
fn the_profile_root_keeps_the_names_of_its_standard_folders() {
    let scan = scan_with(
        &MemTree::new()
            .file("Documents/a.pdf", 10)
            .file("Pictures/x.jpg", 10)
            .file("Secret Project/plan.txt", 10)
            .file("notes about Bob.txt", 10),
        &SplitBudget::default(),
    );
    let text = redacted_of(&scan, "");
    let folders = line(&text, "folders");
    assert!(folders.contains("Documents 10 B"), "{folders}");
    assert!(folders.contains("Pictures 10 B"), "{folders}");
    assert!(!folders.contains("Secret"), "{folders}");
    assert!(!line(&text, "files").contains("Bob"), "{text}");
}

#[test]
fn the_same_name_gets_the_same_mask_within_a_signature() {
    let signature = Signature {
        path: "Documents/Smith".to_owned(),
        unit: UnitReason::Small,
        stats: Stats::default(),
        sample: Sample::new(
            vec![
                Named {
                    name: "Smith".to_owned(),
                    bytes: 10,
                },
                Named {
                    name: "Jones".to_owned(),
                    bytes: 5,
                },
            ],
            vec![Named {
                name: "Smith".to_owned(),
                bytes: 3,
            }],
        ),
        settled: vec![("Jones/node_modules".to_owned(), Decision::Leave)],
    };
    let text = signature.redacted().to_text();
    assert_eq!(line(&text, "path:"), "path: Documents/<1>");
    assert_eq!(line(&text, "folders"), "folders (2): <1> 10 B, <2> 5 B");
    assert_eq!(line(&text, "files"), "files (1): <1> 3 B");
    assert_eq!(
        line(&text, "settled inside:"),
        "settled inside: <2>/node_modules (leave)"
    );
}

#[test]
fn no_personal_name_survives_redaction() {
    let scan = scan_with(&personal_profile(), &small_budget());
    let all: String = signatures(&scan)
        .into_iter()
        .map(|s| s.for_privacy(Privacy::Redacted).to_text())
        .collect();
    for name in [
        "Wedding", "Smith", "guests", "vows", "beach", "photos", "Johnson", "tree.ged", "site",
    ] {
        assert!(!all.contains(name), "`{name}` leaked:\n{all}");
    }
}

#[test]
fn a_redacted_signature_is_the_same_from_one_scan_to_the_next() {
    let once = || -> Vec<String> {
        signatures(&scan_with(&personal_profile(), &small_budget()))
            .iter()
            .map(|s| s.for_privacy(Privacy::Redacted).to_text())
            .collect()
    };
    assert_eq!(once(), once());
}
