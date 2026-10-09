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
