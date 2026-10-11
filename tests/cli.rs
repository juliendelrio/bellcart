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
