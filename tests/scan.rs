//! What a scan must decide. These cases are the specification: when
//! behaviour changes, a case here changes first.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::path::Path;

use bellcart::plan::{Decision, Plan, PlanNode, Source, SplitBudget, UnitReason};
use bellcart::scan::scan;
use bellcart::tree::MemTree;

fn plan(tree: &MemTree) -> Plan {
    plan_with(tree, &SplitBudget::default())
}

fn plan_with(tree: &MemTree, budget: &SplitBudget) -> Plan {
    scan(tree, Path::new(""), &common::core_rules(), budget).expect("the scan succeeds")
}

/// A budget scaled to the tiny trees below: folders above 1000 bytes are
/// cut, sub-folders under 100 bytes stay in their parent's rest.
fn small_budget() -> SplitBudget {
    SplitBudget {
        split_above: 1_000,
        min_unit: 100,
        homogeneous_percent: 90,
        max_depth: 4,
    }
}

fn cut(tree: &MemTree) -> Plan {
    plan_with(tree, &small_budget())
}

fn has_node(plan: &Plan, path: &str) -> bool {
    plan.nodes().iter().any(|n| n.path == path)
}

/// A profile with a bit of everything, large enough for `small_budget`.
fn profile() -> MemTree {
    MemTree::new()
        .file("Documents/taxes/2025.pdf", 300)
        .file("Documents/cv.docx", 200)
        .file("Documents/notes/a.txt", 50)
        .file("Pictures/2023/a.jpg", 2_000)
        .file("Pictures/2024/b.jpg", 2_000)
        .file("Downloads/setup.exe", 700)
        .file("Downloads/archive/old.zip", 900)
        .file("Music/a.mp3", 600)
        .file("Music/b.flac", 600)
        .file("Music/live/c.ogg", 50)
        .file("dev/app/src/main.rs", 150)
        .file("dev/app/node_modules/x.js", 5_000)
        .file(".ssh/id_ed25519", 400)
        .file("AppData/Local/Temp/a.tmp", 300)
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
fn a_small_tree_no_rule_knows_is_still_one_review_node() {
    let plan = plan(
        &MemTree::new()
            .file("Documents/taxes/2025.pdf", 100)
            .file("Pictures/holiday.jpg", 300),
    );
    assert_eq!(plan.tree.decision, Decision::Review);
    assert!(plan.tree.overrides.is_empty());
    assert_eq!(plan.tree.unit, Some(UnitReason::Small));
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
fn the_google_drive_cache_is_left_but_not_a_folder_with_the_same_name() {
    let plan = plan(
        &MemTree::new()
            .file(
                "AppData/Local/Google/DriveFS/1234/content_cache/d1/f_01",
                9_000,
            )
            .file("Documents/Google/DriveFS/notes.txt", 40),
    );
    let cache = node(&plan, "AppData/Local/Google/DriveFS");
    assert_eq!(cache.decision, Decision::Leave);
    assert_eq!(rule_of(cache), "google-drive-cache");
    // Somebody's own folder that happens to share the name stays to review.
    assert!(!has_node(&plan, "Documents/Google/DriveFS"));
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

// Cutting the review mass into folders that can be judged one at a time.

#[test]
fn a_large_review_folder_is_cut_into_its_sub_folders() {
    let plan = cut(&MemTree::new()
        .file("Documents/report.pdf", 600)
        .file("Pictures/a.jpg", 300)
        .file("Pictures/b.png", 300)
        .file("Music/c.mp3", 400));

    for (path, bytes) in [("Documents", 600), ("Pictures", 600), ("Music", 400)] {
        let unit = node(&plan, path);
        assert_eq!(unit.decision, Decision::Review);
        assert_eq!(unit.unit, Some(UnitReason::Small), "{path}");
        assert_eq!(unit.stats.bytes, bytes, "{path}");
    }
    // Nothing lies directly at the root: it is a container, not a unit.
    assert_eq!(plan.tree.unit, None);
    assert_eq!(plan.tree.stats.files, 0);
}

#[test]
fn sub_folders_below_the_unit_size_stay_in_their_parents_rest() {
    let plan = cut(&MemTree::new()
        .file("Downloads/setup.exe", 400)
        .file("Downloads/old/a.zip", 700)
        .file("Downloads/tiny/note.txt", 50));

    let downloads = node(&plan, "Downloads");
    assert_eq!(downloads.unit, Some(UnitReason::Rest));
    assert_eq!(downloads.stats.bytes, 450);
    assert_eq!(node(&plan, "Downloads/old").stats.bytes, 700);
    assert!(!has_node(&plan, "Downloads/tiny"));
}

#[test]
fn a_folder_dominated_by_one_extension_is_one_unit_however_deep() {
    let plan = cut(&MemTree::new()
        .file("Pictures/2023/01/a.jpg", 400)
        .file("Pictures/2023/02/b.jpg", 400)
        .file("Pictures/2024/03/c.jpg", 400)
        .file("Pictures/2024/03/d.png", 50)
        .file("Documents/report.pdf", 900));

    let pictures = node(&plan, "Pictures");
    assert_eq!(
        pictures.unit,
        Some(UnitReason::Homogeneous {
            extension: "jpg".to_owned(),
            percent: 96
        })
    );
    assert_eq!(pictures.stats.bytes, 1_250);
    assert!(!has_node(&plan, "Pictures/2023"));
    assert!(!has_node(&plan, "Pictures/2024/03"));
}

#[test]
fn a_dominant_extension_does_not_hide_a_large_minority() {
    // 93 % video, but the other 7 % are more than a unit's worth of
    // documents: they must be judged on their own.
    let plan = cut(&MemTree::new()
        .file("Videos/movies/a.mkv", 20_000)
        .file("Videos/work/b.docx", 1_500));

    assert!(has_node(&plan, "Videos/work"));
    assert_eq!(node(&plan, "Videos/work").stats.bytes, 1_500);
    assert_eq!(node(&plan, "Videos/movies").stats.bytes, 20_000);
}

#[test]
fn a_mixed_folder_below_the_homogeneity_threshold_is_cut() {
    let plan = cut(&MemTree::new()
        .file("Projects/a/x.psd", 850)
        .file("Projects/b/y.jpg", 160));

    assert_eq!(node(&plan, "Projects/a").unit, Some(UnitReason::Small));
    assert_eq!(node(&plan, "Projects/b").unit, Some(UnitReason::Small));
}

#[test]
fn the_cut_stops_at_the_depth_limit() {
    let plan = cut(&MemTree::new()
        .file("a/b/c/d/e/x.pdf", 600)
        .file("a/b/c/d/f/y.zip", 600));

    let deep = node(&plan, "a/b/c/d");
    assert_eq!(deep.unit, Some(UnitReason::DepthLimit));
    assert_eq!(deep.stats.bytes, 1_200);
    assert!(!has_node(&plan, "a/b/c/d/e"));
}

#[test]
fn rules_still_apply_below_the_depth_limit() {
    let plan = cut(&MemTree::new()
        .file("a/b/c/d/e/f/main.js", 10)
        .file("a/b/c/d/e/f/node_modules/x.js", 5_000));

    let deps = node(&plan, "a/b/c/d/e/f/node_modules");
    assert_eq!(deps.decision, Decision::Leave);
    assert_eq!(deps.stats.bytes, 5_000);
}

#[test]
fn a_rule_settled_folder_inside_a_unit_keeps_its_own_node() {
    let plan = cut(&MemTree::new()
        .file("Backup/old.zip", 5_000)
        .file("Backup/home/.ssh/id_ed25519", 400)
        .file("Documents/report.pdf", 3_000));

    let backup = node(&plan, "Backup");
    assert!(matches!(backup.unit, Some(UnitReason::Homogeneous { .. })));
    let keys = node(&plan, "Backup/home/.ssh");
    assert_eq!(keys.decision, Decision::Migrate);
    assert_eq!(keys.stats.bytes, 400);
}

#[test]
fn settled_folders_do_not_count_toward_a_units_size_or_mix() {
    // 50 kB of dependencies must not make a 150-byte project look big.
    let plan = cut(&MemTree::new()
        .file("dev/app/src/main.rs", 150)
        .file("dev/app/node_modules/big.js", 50_000)
        .file("Documents/report.pdf", 600)
        .file("Music/song.mp3", 600));

    let dev = node(&plan, "dev");
    assert_eq!(dev.unit, Some(UnitReason::Small));
    assert_eq!(dev.stats.bytes, 150);
    assert!(!has_node(&plan, "dev/app"));
    assert_eq!(
        node(&plan, "dev/app/node_modules").decision,
        Decision::Leave
    );
}

#[test]
fn cutting_only_ever_produces_review_nodes() {
    let settled = |plan: &Plan| -> Vec<(String, Decision)> {
        plan.nodes()
            .into_iter()
            .filter(|n| n.decision != Decision::Review)
            .map(|n| (n.path.clone(), n.decision))
            .collect()
    };
    assert_eq!(settled(&plan(&profile())), settled(&cut(&profile())));
}

#[test]
fn cutting_never_changes_the_totals() {
    assert_eq!(plan(&profile()).totals, cut(&profile()).totals);
}

#[test]
fn a_cut_folder_without_files_of_its_own_has_no_node() {
    let plan = cut(&MemTree::new()
        .file("Work/client-a/a.pdf", 800)
        .file("Work/client-b/b.zip", 800)
        .file("Documents/c.docx", 600));

    assert!(!has_node(&plan, "Work"));
    assert!(has_node(&plan, "Work/client-a"));
    assert!(has_node(&plan, "Work/client-b"));
}

#[test]
fn each_unit_says_why_the_cut_stopped_there() {
    let plan = cut(&profile());
    assert_eq!(node(&plan, "Documents").unit, Some(UnitReason::Small));
    assert_eq!(
        node(&plan, "Pictures").unit,
        Some(UnitReason::Homogeneous {
            extension: "jpg".to_owned(),
            percent: 100
        })
    );
    assert_eq!(node(&plan, "Downloads").unit, Some(UnitReason::Rest));
    assert_eq!(
        node(&plan, "Downloads/archive").unit,
        Some(UnitReason::Small)
    );
    // Mixed, and no sub-folder big enough to stand alone.
    assert_eq!(node(&plan, "Music").unit, Some(UnitReason::Leaf));
    // Settled folders are not review units.
    assert_eq!(node(&plan, ".ssh").unit, None);
}

#[test]
fn how_a_folder_is_cut_does_not_depend_on_its_siblings() {
    let downloads = || {
        MemTree::new()
            .file("Downloads/setup.exe", 700)
            .file("Downloads/archive/old.zip", 900)
            .file("Downloads/tiny/a.txt", 20)
    };
    let under_downloads = |plan: &Plan| -> Vec<PlanNode> {
        plan.nodes()
            .into_iter()
            .filter(|n| n.path.starts_with("Downloads"))
            .cloned()
            .collect()
    };
    let alone = cut(&downloads());
    let with_sibling = cut(&downloads().file("Videos/huge.mkv", 1_000_000));
    assert_eq!(under_downloads(&alone), under_downloads(&with_sibling));
}

#[test]
fn unreadable_folders_are_counted_in_the_unit_that_holds_them() {
    let plan = cut(&MemTree::new()
        .file("Downloads/setup.exe", 700)
        .file("Downloads/archive/old.zip", 900)
        .unreadable("Downloads/locked"));

    assert_eq!(node(&plan, "Downloads").stats.unreadable, 1);
    assert_eq!(node(&plan, "Downloads/archive").stats.unreadable, 0);
}

#[test]
fn the_plan_records_the_budget_it_was_cut_with() {
    assert_eq!(cut(&profile()).split, Some(small_budget()));
}

#[test]
fn a_version_1_plan_still_parses() {
    let v1 = r#"{
        "version": 1,
        "root": "C:\\Users\\me",
        "totals": {"review": {"bytes": 2, "files": 1}},
        "tree": {
            "path": "",
            "decision": "review",
            "source": {"by": "default"},
            "stats": {"bytes": 2, "files": 1}
        }
    }"#;
    let plan: Plan = serde_json::from_str(v1).expect("a v1 plan parses");
    assert_eq!(plan.version, 1);
    assert_eq!(plan.split, None);
    assert_eq!(plan.tree.unit, None);
}
