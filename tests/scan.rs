//! What a scan must decide. These cases are the specification: when
//! behaviour changes, a case here changes first.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::path::Path;

use bellcart::plan::{Decision, Plan, PlanNode, Source};
use bellcart::scan::scan;
use bellcart::tree::MemTree;

fn plan(tree: &MemTree) -> Plan {
    scan(tree, Path::new(""), &common::core_rules()).expect("the scan succeeds")
}

fn node<'a>(plan: &'a Plan, path: &str) -> &'a PlanNode {
    plan.nodes()
        .into_iter()
        .find(|n| n.path == path)
        .unwrap_or_else(|| panic!("no plan node for `{path}`"))
}

fn rule_of(node: &PlanNode) -> &str {
    match &node.source {
        Source::Rule { rule, .. } => rule,
        Source::Default => "",
    }
}

#[test]
fn a_tree_no_rule_knows_is_one_review_node() {
    let plan = plan(
        &MemTree::new()
            .file("Documents/taxes/2025.pdf", 100)
            .file("Pictures/holiday.jpg", 300),
    );
    assert_eq!(plan.tree.decision, Decision::Review);
    assert!(plan.tree.overrides.is_empty());
    assert_eq!(plan.tree.stats.bytes, 400);
    assert_eq!(plan.tree.stats.files, 2);
}

#[test]
fn dependencies_are_left_and_the_project_stays_to_review() {
    let plan = plan(
        &MemTree::new()
            .file("dev/app/src/main.js", 10)
            .file("dev/app/node_modules/left-pad/index.js", 1_000)
            .file("dev/app/node_modules/left-pad/package.json", 500),
    );

    let deps = node(&plan, "dev/app/node_modules");
    assert_eq!(deps.decision, Decision::Leave);
    assert_eq!(rule_of(deps), "node-modules");
    assert_eq!(deps.stats.bytes, 1_500);

    assert_eq!(plan.tree.decision, Decision::Review);
    assert_eq!(plan.tree.stats.bytes, 10);
    assert_eq!(plan.totals[&Decision::Leave].bytes, 1_500);
    assert_eq!(plan.totals[&Decision::Review].bytes, 10);
}

#[test]
fn a_folder_named_venv_is_only_left_when_it_is_a_virtual_environment() {
    let plan = plan(
        &MemTree::new()
            .file("work/api/venv/pyvenv.cfg", 80)
            .file("work/api/venv/Lib/site-packages/x.py", 900)
            .file("Documents/venv/wedding-venues.docx", 5_000),
    );

    assert_eq!(node(&plan, "work/api/venv").decision, Decision::Leave);
    // The other one is somebody's documents: it must not get its own node,
    // and must stay in the review mass.
    assert!(plan.nodes().iter().all(|n| n.path != "Documents/venv"));
    assert_eq!(plan.totals[&Decision::Review].bytes, 5_000);
}

#[test]
fn ssh_keys_are_migrated() {
    let plan = plan(&MemTree::new().file(".ssh/id_ed25519", 400));
    let keys = node(&plan, ".ssh");
    assert_eq!(keys.decision, Decision::Migrate);
    assert_eq!(rule_of(keys), "ssh-keys");
}

#[test]
fn profile_caches_are_left_wherever_the_scan_starts() {
    let from_profile = plan(&MemTree::new().file("AppData/Local/Temp/a.tmp", 7));
    assert_eq!(
        node(&from_profile, "AppData/Local/Temp").decision,
        Decision::Leave
    );

    let from_users = plan(&MemTree::new().file("julien/AppData/Local/Temp/a.tmp", 7));
    assert_eq!(
        node(&from_users, "julien/AppData/Local/Temp").decision,
        Decision::Leave
    );
}

#[test]
fn browser_caches_are_left_but_not_the_profile() {
    let plan = plan(
        &MemTree::new()
            .file(
                "AppData/Local/Google/Chrome/User Data/Default/Cache/f_0001",
                9_000,
            )
            .file(
                "AppData/Local/Google/Chrome/User Data/Default/Bookmarks",
                40,
            ),
    );
    let cache = node(&plan, "AppData/Local/Google/Chrome/User Data/Default/Cache");
    assert_eq!(cache.decision, Decision::Leave);
    assert_eq!(plan.totals[&Decision::Review].bytes, 40);
}

#[test]
fn rules_are_not_applied_inside_a_settled_folder() {
    // A `.ssh` folder inside node_modules is part of the dependency tree,
    // not a key store.
    let plan = plan(&MemTree::new().file("app/node_modules/pkg/.ssh/config", 5));
    assert!(plan.nodes().iter().all(|n| n.decision != Decision::Migrate));
    assert_eq!(plan.totals[&Decision::Leave].bytes, 5);
}

#[test]
fn links_are_never_followed() {
    let plan = plan(
        &MemTree::new()
            .file("Documents/a.txt", 1)
            .link("Application Data"),
    );
    assert_eq!(plan.tree.stats.files, 1);
}

#[test]
fn stats_track_extensions_and_dates() {
    let plan = plan(
        &MemTree::new()
            .file_at("Pictures/a.JPG", 10, Some(1_000))
            .file_at("Pictures/b.jpg", 20, Some(3_000))
            .file_at("Pictures/notes", 1, Some(2_000)),
    );
    let stats = &plan.tree.stats;
    assert_eq!(stats.extensions["jpg"], 30);
    assert_eq!(stats.extensions[""], 1);
    assert_eq!(stats.oldest, Some(1_000));
    assert_eq!(stats.newest, Some(3_000));
}

#[test]
fn the_plan_round_trips_through_json() {
    let plan = plan(
        &MemTree::new()
            .file("a/node_modules/x.js", 1)
            .file("b.txt", 2),
    );
    let json = serde_json::to_string(&plan).expect("serializes");
    let back: Plan = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(plan, back);
}
