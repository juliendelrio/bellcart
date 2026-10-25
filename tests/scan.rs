//! What a scan must decide. These cases are the specification: when
//! behaviour changes, a case here changes first.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::path::Path;

use bellcart::pack::{Pack, Recipe, RuleSet};
use bellcart::plan::{Decision, Plan, PlanNode, Source, SplitBudget, UnitReason};
use bellcart::recipe::{Recipes, Variables};
use bellcart::scan::{Counts, Progress, scan, scan_with};
use bellcart::tree::MemTree;
use common::small_budget;

fn plan(tree: &MemTree) -> Plan {
    plan_with(tree, &SplitBudget::default())
}

fn plan_with(tree: &MemTree, budget: &SplitBudget) -> Plan {
    scan(tree, Path::new(""), &common::core_rules(), budget)
        .expect("the scan succeeds")
        .plan
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
        _ => "",
    }
}

fn recipe_of(node: &PlanNode) -> &str {
    match &node.source {
        Source::Recipe { recipe } => recipe,
        _ => "",
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
fn a_rest_too_small_to_judge_joins_the_rest_of_the_folder_above() {
    let plan = cut(&MemTree::new()
        .file("Downloads/desktop.ini", 5)
        .file("Downloads/old/a.zip", 700)
        .file("Downloads/new/b.mp4", 700)
        .file("notes.txt", 20));

    assert!(!has_node(&plan, "Downloads"));
    assert_eq!(node(&plan, "Downloads/old").unit, Some(UnitReason::Small));
    assert_eq!(node(&plan, "Downloads/new").unit, Some(UnitReason::Small));
    assert_eq!(plan.tree.unit, Some(UnitReason::Rest));
    assert_eq!(plan.tree.stats.bytes, 25);
    assert_eq!(plan.tree.stats.files, 2);
}

#[test]
fn a_rest_of_at_least_the_unit_size_keeps_its_own_node() {
    let plan = cut(&MemTree::new()
        .file("Downloads/setup.exe", 100)
        .file("Downloads/old/a.zip", 700)
        .file("Downloads/new/b.mp4", 700));

    assert_eq!(node(&plan, "Downloads").unit, Some(UnitReason::Rest));
    assert_eq!(node(&plan, "Downloads").stats.bytes, 100);
}

#[test]
fn a_small_rest_with_an_unreadable_folder_keeps_its_own_node() {
    // Where access was denied must stay visible where it happened.
    let plan = cut(&MemTree::new()
        .file("Downloads/old/a.zip", 700)
        .file("Downloads/new/b.mp4", 700)
        .unreadable("Downloads/locked"));

    assert_eq!(node(&plan, "Downloads").unit, Some(UnitReason::Rest));
    assert_eq!(node(&plan, "Downloads").stats.unreadable, 1);
}

#[test]
fn a_tiny_rest_at_the_root_stays_where_it_is() {
    let plan = cut(&MemTree::new()
        .file("desktop.ini", 5)
        .file("old/a.zip", 700)
        .file("new/b.mp4", 700));

    assert_eq!(plan.tree.unit, Some(UnitReason::Rest));
    assert_eq!(plan.tree.stats.bytes, 5);
}

#[test]
fn long_numeric_suffixes_are_not_extensions() {
    let plan = plan(
        &MemTree::new()
            .file("claude.exe.old.12.33452", 10)
            .file("backup.7z.001", 20)
            .file("report.2024", 40)
            .file("docker.log.657", 5),
    );

    let extensions = &plan.tree.stats.extensions;
    // Split archives number their parts with three digits.
    assert_eq!(extensions["001"], 20);
    // Other runs of digits are counters, process numbers or years.
    assert_eq!(extensions[""], 55);
    assert_eq!(extensions.len(), 2);
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

/// A tree wide and deep enough for several threads to share the work.
fn large_tree() -> MemTree {
    let mut tree = profile();
    for a in 0..12 {
        for b in 0..12 {
            for c in 0..6 {
                let ext = ["pdf", "jpg", "txt", "zip"][(a + b + c) % 4];
                tree = tree.file(
                    &format!("Work/p{a}/s{b}/f{c}.{ext}"),
                    (a * 1_000 + b * 37 + c * 11) as u64,
                );
            }
            tree = tree.file(&format!("Work/p{a}/s{b}/node_modules/m/i.js"), 70);
        }
    }
    tree.unreadable("Work/p3/locked")
}

fn scan_on_threads(tree: &MemTree, threads: usize) -> bellcart::scan::Scan {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("a thread pool");
    pool.install(|| {
        scan(tree, Path::new(""), &common::core_rules(), &small_budget())
            .expect("the scan succeeds")
    })
}

#[test]
fn a_parallel_scan_gives_the_same_plan_as_a_sequential_one() {
    let tree = large_tree();
    let sequential = scan_on_threads(&tree, 1);
    for threads in [2, 4, 8] {
        let parallel = scan_on_threads(&tree, threads);
        assert_eq!(
            serde_json::to_value(&parallel.plan).unwrap(),
            serde_json::to_value(&sequential.plan).unwrap(),
            "{threads} threads"
        );
        assert_eq!(parallel.samples, sequential.samples, "{threads} threads");
    }
}

#[test]
fn progress_counts_every_folder_file_and_byte_read() {
    let tree = MemTree::new()
        .file("Documents/a.pdf", 100)
        .file("Documents/b/c.txt", 20)
        .file("dev/app/node_modules/x/y.js", 5_000)
        .file("top.txt", 3)
        .link("Shortcut")
        .unreadable("Locked");
    let progress = Progress::default();
    scan_with(
        &tree,
        Path::new(""),
        &common::core_rules(),
        &Recipes::default(),
        &SplitBudget::default(),
        &progress,
    )
    .expect("the scan succeeds");

    // The root, Documents, Documents/b, dev, dev/app, and the settled
    // node_modules with its folder x. Not the unreadable one, nor the link.
    assert_eq!(
        progress.counts(),
        Counts {
            folders: 7,
            files: 4,
            bytes: 5_123,
        }
    );
}

#[test]
fn progress_starts_at_zero() {
    assert_eq!(Progress::default().counts(), Counts::default());
}

// Rules added after a first run on a real Windows 11 profile. Each `leave`
// shows a folder it settles and a look-alike it must leave to review.

/// Scans `file` under `folder` next to `look_alike`, then checks that
/// `rule` leaves `folder` and that the look-alike stays to review.
fn left_by_rule_but_not_look_alike(folder: &str, file: &str, rule: &str, look_alike: &str) {
    let plan = plan(
        &MemTree::new()
            .file(&format!("{folder}/{file}"), 9_000)
            .file(look_alike, 40),
    );
    let settled = node(&plan, folder);
    assert_eq!(settled.decision, Decision::Leave, "{folder}");
    assert_eq!(rule_of(settled), rule, "{folder}");
    assert_eq!(
        plan.totals[&Decision::Review].bytes,
        40,
        "`{look_alike}` must stay to review"
    );
}

#[test]
fn electron_application_caches_are_left() {
    left_by_rule_but_not_look_alike(
        "AppData/Roaming/Slack/Cache",
        "Cache_Data/f_000001",
        "electron-cache",
        "AppData/Roaming/SomeApp/Cache/my-notes.txt",
    );
}

#[test]
fn electron_code_and_gpu_caches_are_left() {
    for cache in [
        "Code Cache",
        "GPUCache",
        "DawnGraphiteCache",
        "DawnWebGPUCache",
    ] {
        left_by_rule_but_not_look_alike(
            &format!("AppData/Roaming/Slack/{cache}"),
            "index",
            "electron-gpu-cache",
            &format!("Documents/{cache}/index"),
        );
    }
}

#[test]
fn service_worker_caches_are_left_but_not_other_site_data() {
    left_by_rule_but_not_look_alike(
        "AppData/Local/Google/Chrome/User Data/Default/Service Worker/CacheStorage",
        "abc/index",
        "service-worker-cache",
        "AppData/Local/Google/Chrome/User Data/Default/File System/000/p/00/0001",
    );
    left_by_rule_but_not_look_alike(
        "AppData/Roaming/Slack/Service Worker/ScriptCache",
        "index",
        "service-worker-cache",
        "AppData/Roaming/Slack/IndexedDB/a.leveldb",
    );
}

#[test]
fn browser_components_downloaded_again_are_left() {
    for component in [
        "OptGuideOnDeviceModel",
        "optimization_guide_model_store",
        "screen_ai",
        "component_crx_cache",
        "extensions_crx_cache",
        "GrShaderCache",
        "ShaderCache",
    ] {
        left_by_rule_but_not_look_alike(
            &format!("AppData/Local/Google/Chrome/User Data/{component}"),
            "weights.bin",
            "browser-components",
            &format!("Documents/{component}/weights.bin"),
        );
    }
}

#[test]
fn chromium_dawn_caches_are_left() {
    left_by_rule_but_not_look_alike(
        "AppData/Local/Microsoft/Edge/User Data/Default/DawnGraphiteCache",
        "data_0",
        "chromium-cache",
        "AppData/Local/Microsoft/Edge/User Data/Default/Bookmarks",
    );
}

#[test]
fn crash_reports_are_left() {
    left_by_rule_but_not_look_alike(
        "AppData/Roaming/Code/Crashpad",
        "reports/a.dmp",
        "crash-reports",
        "Documents/Crash/report.docx",
    );
}

#[test]
fn vs_code_caches_and_logs_are_left_but_not_its_settings() {
    for folder in ["CachedData", "CachedExtensionVSIXs", "logs"] {
        left_by_rule_but_not_look_alike(
            &format!("AppData/Roaming/Code/{folder}"),
            "x/blob",
            "vscode-caches",
            "AppData/Roaming/Code/User/settings.json",
        );
    }
}

#[test]
fn downloaded_application_updates_are_left() {
    left_by_rule_but_not_look_alike(
        "AppData/Local/orca-updater",
        "installer.exe",
        "electron-updater-download",
        "AppData/Local/my-updater/notes.txt",
    );
    left_by_rule_but_not_look_alike(
        "AppData/Local/slack/packages",
        "RELEASES",
        "squirrel-update-download",
        "AppData/Local/SomeApp/packages/readme.txt",
    );
}

#[test]
fn programs_installed_for_the_user_are_left() {
    left_by_rule_but_not_look_alike(
        "AppData/Local/Programs",
        "GIMP/bin/gimp.exe",
        "user-installed-programs",
        "Documents/Programs/my-script.py",
    );
}

#[test]
fn hugging_face_models_are_left() {
    left_by_rule_but_not_look_alike(
        ".cache/huggingface/hub",
        "models--org--name/blobs/abc",
        "huggingface-hub-cache",
        "Documents/huggingface/hub/my-model.bin",
    );
}

#[test]
fn rust_toolchains_and_cargo_git_checkouts_are_left() {
    left_by_rule_but_not_look_alike(
        ".rustup/toolchains",
        "stable/bin/rustc.exe",
        "rustup-toolchains",
        "dev/toolchains/notes.md",
    );
    left_by_rule_but_not_look_alike(
        ".cargo/git",
        "checkouts/x/lib.rs",
        "cargo-git",
        ".cargo/config.toml",
    );
}

#[test]
fn the_pnpm_store_is_left() {
    left_by_rule_but_not_look_alike(
        "AppData/Local/pnpm/store",
        "v3/files/00/abc",
        "pnpm-store",
        "dev/store/index.js",
    );
}

#[test]
fn wsl_and_docker_disk_images_stay_to_review() {
    // They hold the user's Linux files and containers: never left by a rule.
    let plan = plan(
        &MemTree::new()
            .file("AppData/Local/Docker/wsl/disk/docker_data.vhdx", 9_000)
            .file("AppData/Local/wsl/{1234}/ext4.vhdx", 9_000)
            .file(
                "AppData/Local/Packages/CanonicalGroupLimited.Ubuntu_79rhkp1fndgsc/LocalState/ext4.vhdx",
                9_000,
            ),
    );
    assert_eq!(plan.totals[&Decision::Review].bytes, 27_000);
}

// Rules added after a second pass on real Windows 11 profiles.

#[test]
fn file_explorer_thumbnail_caches_are_left() {
    left_by_rule_but_not_look_alike(
        "AppData/Local/Microsoft/Windows/Explorer",
        "thumbcache_256.db",
        "explorer-thumbnail-cache",
        "Documents/Explorer/notes.txt",
    );
}

#[test]
fn python_tool_caches_are_left_only_with_their_cache_tag() {
    for cache in [".pytest_cache", ".mypy_cache", ".ruff_cache"] {
        let folder = format!("dev/app/{cache}");
        let plan = plan(
            &MemTree::new()
                .file(&format!("{folder}/CACHEDIR.TAG"), 9_000)
                .file(&format!("Documents/{cache}/notes.txt"), 40),
        );
        assert_eq!(rule_of(node(&plan, &folder)), "python-tool-caches");
        assert_eq!(plan.totals[&Decision::Review].bytes, 40, "{cache}");
    }
}

#[test]
fn the_go_module_cache_is_left_but_not_a_folder_with_the_same_path() {
    left_by_rule_but_not_look_alike(
        "go/pkg/mod",
        "cache/download/x.zip",
        "go-module-cache",
        "Documents/go/pkg/mod/my-module/main.go",
    );
}

#[test]
fn yarn_and_bun_caches_are_left() {
    left_by_rule_but_not_look_alike(
        "AppData/Local/Yarn/Cache",
        "v6/npm-a.tgz",
        "yarn-cache",
        "Documents/Yarn/Cache/pattern.pdf",
    );
    left_by_rule_but_not_look_alike(
        ".bun/install/cache",
        "a.tgz",
        "bun-cache",
        "Documents/install/cache/a.txt",
    );
}

#[test]
fn gradle_versions_downloaded_by_the_wrapper_are_left() {
    left_by_rule_but_not_look_alike(
        ".gradle/wrapper/dists",
        "gradle-8.10-bin/gradle.zip",
        "gradle-wrapper-dists",
        "Documents/wrapper/dists/plan.pdf",
    );
}

#[test]
fn jetbrains_caches_are_left_but_not_its_settings() {
    left_by_rule_but_not_look_alike(
        "AppData/Local/JetBrains/IdeaIC2026.2/caches",
        "content.dat",
        "jetbrains-caches",
        "AppData/Roaming/JetBrains/IdeaIC2026.2/options/editor.xml",
    );
}

// Recipes: applications found on the profile, and the paths their `copy`
// and `skip` steps settle.

/// A pack made of the core `pack.yaml` and the given files.
fn custom_pack(files: &[(&str, &str)]) -> bellcart::Result<Pack> {
    let meta = std::fs::read_to_string(common::repo_root().join("packs/core/pack.yaml"))
        .expect("the core pack.yaml");
    let mut all = vec![("pack.yaml".to_owned(), meta)];
    all.extend(
        files
            .iter()
            .map(|(p, c)| ((*p).to_owned(), (*c).to_owned())),
    );
    Pack::from_files(all)
}

fn recipes_of(yaml: &[&str]) -> Vec<Recipe> {
    // A recipe file is named after its id.
    let names: Vec<String> = yaml
        .iter()
        .map(|y| {
            let id = y
                .lines()
                .find_map(|l| l.strip_prefix("id: "))
                .expect("a recipe id");
            format!("recipes/{id}.yaml")
        })
        .collect();
    let files: Vec<(&str, &str)> = names
        .iter()
        .map(String::as_str)
        .zip(yaml.iter().copied())
        .collect();
    custom_pack(&files).expect("the recipes load").recipes
}

fn with_recipes_and_rules(tree: &MemTree, recipes: &[Recipe], rules: &RuleSet) -> Plan {
    let found = Recipes::detect(tree, Path::new(""), recipes, &Variables::profile());
    scan_with(
        tree,
        Path::new(""),
        rules,
        &found,
        &SplitBudget::default(),
        &Progress::default(),
    )
    .expect("the scan succeeds")
    .plan
}

fn with_recipes(tree: &MemTree, recipes: &[Recipe]) -> Plan {
    with_recipes_and_rules(tree, recipes, &common::core_rules())
}

fn with_core_recipes(tree: &MemTree) -> Plan {
    with_recipes(tree, &common::core_pack().recipes)
}

fn found(plan: &Plan) -> Vec<&str> {
    plan.applications
        .iter()
        .map(|a| a.recipe.as_str())
        .collect()
}

/// A recipe for a tool that keeps its settings in `%APPDATA%\Tool`.
const TOOL: &str = r"
id: tool
name: Tool
detect:
  any:
    - path: '%APPDATA%\Tool'
before:
  - type: copy
    path: '%APPDATA%\Tool'
  - type: skip
    path: '%APPDATA%\Tool\Cache'
";

/// A one-step recipe found when `probe` exists.
fn one_step(id: &str, probe: &str, step: &str, path: &str) -> String {
    format!(
        "id: {id}\nname: {id}\ndetect:\n  any:\n    - path: '{probe}'\nbefore:\n  - type: {step}\n    path: '{path}'\n"
    )
}

#[test]
fn an_application_is_found_when_one_of_its_probes_exists() {
    let plan = with_core_recipes(&MemTree::new().file(".gitconfig", 120));
    assert_eq!(found(&plan), ["git"]);
    assert_eq!(plan.applications[0].name, "Git");
}

#[test]
fn an_application_with_no_probe_on_disk_is_not_listed() {
    let plan = with_core_recipes(&MemTree::new().file("Documents/a.txt", 10));
    assert!(plan.applications.is_empty());
    assert!(
        plan.nodes()
            .iter()
            .all(|n| !matches!(n.source, Source::Recipe { .. }))
    );
}

#[test]
fn probes_ignore_case_as_windows_does() {
    let plan =
        with_core_recipes(&MemTree::new().file("appdata/roaming/CODE/user/settings.json", 10));
    assert_eq!(found(&plan), ["vscode"]);
}

#[test]
fn a_probe_may_find_a_link() {
    // winget.exe is an app execution alias, a link the scan never follows.
    let plan = with_core_recipes(
        &MemTree::new()
            .file("Documents/a.txt", 1)
            .link("AppData/Local/Microsoft/WindowsApps/winget.exe"),
    );
    assert_eq!(found(&plan), ["winget-apps"]);
}

#[test]
fn applications_are_listed_by_recipe_id() {
    let plan = with_core_recipes(
        &MemTree::new()
            .file(".gitconfig", 1)
            .file(".ssh/id_ed25519", 1)
            .file("AppData/Roaming/Code/User/settings.json", 1),
    );
    assert_eq!(found(&plan), ["git", "openssh", "vscode"]);
}

#[test]
fn a_copy_step_can_migrate_a_single_file() {
    let plan = with_core_recipes(&MemTree::new().file(".gitconfig", 120).file("notes.txt", 30));
    let config = node(&plan, ".gitconfig");
    assert_eq!(config.decision, Decision::Migrate);
    assert_eq!(recipe_of(config), "git");
    assert_eq!(config.stats.files, 1);
    assert_eq!(config.stats.bytes, 120);
    // The file is no longer part of what there is to review.
    assert_eq!(plan.tree.stats.bytes, 30);
}

#[test]
fn a_copy_step_migrates_a_folder_no_rule_settles() {
    let plan = with_core_recipes(
        &MemTree::new()
            .file(
                "AppData/Roaming/Mozilla/Firefox/Profiles/x.default/places.sqlite",
                5_000,
            )
            .file("AppData/Roaming/Mozilla/Firefox/profiles.ini", 100),
    );
    let firefox = node(&plan, "AppData/Roaming/Mozilla/Firefox");
    assert_eq!(firefox.decision, Decision::Migrate);
    assert_eq!(recipe_of(firefox), "mozilla-firefox");
    assert_eq!(firefox.stats.bytes, 5_100);
}

#[test]
fn a_skip_applies_inside_a_folder_a_copy_migrates() {
    let plan = with_recipes(
        &MemTree::new()
            .file("AppData/Roaming/Tool/settings.ini", 100)
            .file("AppData/Roaming/Tool/Cache/blob", 9_000),
        &recipes_of(&[TOOL]),
    );
    let tool = node(&plan, "AppData/Roaming/Tool");
    assert_eq!(tool.decision, Decision::Migrate);
    assert_eq!(tool.stats.bytes, 100);
    let cache = node(&plan, "AppData/Roaming/Tool/Cache");
    assert_eq!(cache.decision, Decision::Leave);
    assert_eq!(recipe_of(cache), "tool");
    assert_eq!(plan.totals[&Decision::Leave].bytes, 9_000);
}

#[test]
fn a_recipe_rescues_a_path_inside_a_folder_a_rule_leaves() {
    let recipe = one_step(
        "portable",
        r"%LOCALAPPDATA%\Programs\Portable",
        "copy",
        r"%LOCALAPPDATA%\Programs\Portable\profiles",
    );
    let plan = with_recipes(
        &MemTree::new()
            .file("AppData/Local/Programs/Portable/tool.exe", 9_000)
            .file("AppData/Local/Programs/Portable/profiles/me.json", 50),
        &recipes_of(&[&recipe]),
    );
    let programs = node(&plan, "AppData/Local/Programs");
    assert_eq!(programs.decision, Decision::Leave);
    assert_eq!(programs.stats.bytes, 9_000);
    let profiles = node(&plan, "AppData/Local/Programs/Portable/profiles");
    assert_eq!(profiles.decision, Decision::Migrate);
    assert_eq!(recipe_of(profiles), "portable");
}

#[test]
fn a_recipe_copy_wins_over_a_leave_rule_on_the_same_folder() {
    let recipe = one_step(
        "keeper",
        r"%LOCALAPPDATA%\Temp\keeper.lock",
        "copy",
        r"%LOCALAPPDATA%\Temp",
    );
    let plan = with_recipes(
        &MemTree::new().file("AppData/Local/Temp/keeper.lock", 10),
        &recipes_of(&[&recipe]),
    );
    let temp = node(&plan, "AppData/Local/Temp");
    assert_eq!(temp.decision, Decision::Migrate);
    assert_eq!(recipe_of(temp), "keeper");
}

#[test]
fn a_recipe_skip_never_overrides_a_rule_that_keeps_the_folder() {
    let rules = "rules:\n  - id: app-keys\n    decision: migrate\n    reason: keys-and-credentials\n    match:\n      path_glob: \"**/AppData/Roaming/Keys\"\n";
    let recipe = one_step("careless", r"%APPDATA%\Keys", "skip", r"%APPDATA%\Keys");
    let pack = custom_pack(&[
        ("rules/keys.yaml", rules),
        ("recipes/careless.yaml", &recipe),
    ])
    .expect("the pack loads");
    let rules = RuleSet::compile(&pack.rules).expect("the rules compile");
    let plan = with_recipes_and_rules(
        &MemTree::new().file("AppData/Roaming/Keys/key.pem", 10),
        &pack.recipes,
        &rules,
    );
    let keys = node(&plan, "AppData/Roaming/Keys");
    assert_eq!(keys.decision, Decision::Migrate);
    assert_eq!(rule_of(keys), "app-keys");
}

#[test]
fn a_rule_and_a_recipe_that_agree_keep_the_rule_and_its_reason() {
    let plan = with_core_recipes(
        &MemTree::new()
            .file("AppData/Roaming/Code/User/settings.json", 10)
            .file("AppData/Roaming/Code/CachedData/x/blob", 9_000),
    );
    let cached = node(&plan, "AppData/Roaming/Code/CachedData");
    assert_eq!(cached.decision, Decision::Leave);
    assert_eq!(rule_of(cached), "vscode-caches");
}

#[test]
fn when_two_recipes_settle_the_same_path_copy_wins() {
    let skipper = one_step(
        "skipper",
        r"%APPDATA%\Shared",
        "skip",
        r"%APPDATA%\Shared\data",
    );
    let keeper = one_step(
        "keeper",
        r"%APPDATA%\Shared",
        "copy",
        r"%APPDATA%\Shared\data",
    );
    for order in [[&skipper, &keeper], [&keeper, &skipper]] {
        let order: Vec<&str> = order.iter().map(|s| s.as_str()).collect();
        let plan = with_recipes(
            &MemTree::new().file("AppData/Roaming/Shared/data/a.db", 10),
            &recipes_of(&order),
        );
        let data = node(&plan, "AppData/Roaming/Shared/data");
        assert_eq!(data.decision, Decision::Migrate);
        assert_eq!(recipe_of(data), "keeper");
    }
}

#[test]
fn steps_of_an_application_not_found_do_nothing() {
    let recipe = one_step("absent", r"%APPDATA%\Absent", "copy", r"%APPDATA%\Present");
    let plan = with_recipes(
        &MemTree::new().file("AppData/Roaming/Present/a.txt", 10),
        &recipes_of(&[&recipe]),
    );
    assert!(plan.applications.is_empty());
    assert!(!has_node(&plan, "AppData/Roaming/Present"));
}

#[test]
fn recipes_never_change_the_total_size() {
    let tree = profile()
        .file(".gitconfig", 120)
        .file("AppData/Roaming/Code/User/settings.json", 40)
        .file("AppData/Roaming/Code/Cache/Cache_Data/f", 900)
        .file("AppData/Roaming/Mozilla/Firefox/profiles.ini", 30);
    let total = |plan: &Plan| -> (u64, u64) {
        plan.totals
            .values()
            .fold((0, 0), |(b, f), t| (b + t.bytes, f + t.files))
    };
    assert_eq!(total(&with_core_recipes(&tree)), total(&plan(&tree)));
}

#[test]
fn a_recipe_skip_outside_application_data_is_refused() {
    for path in [r"%USERPROFILE%\Documents", "%APPDATA%", r"%LOCALAPPDATA%\"] {
        let recipe = one_step("bad", r"%APPDATA%\Bad", "skip", path);
        let error = custom_pack(&[("recipes/bad.yaml", &recipe)])
            .expect_err("a skip outside application data must not load");
        assert!(error.to_string().contains("skip"), "{path}: {error}");
    }
}

#[test]
fn a_recipe_path_outside_the_profile_is_refused() {
    for path in [
        r"C:\Windows",
        r"%WINDIR%\System32",
        r"%APPDATA%\..\..\Documents",
    ] {
        let recipe = one_step("bad", path, "copy", r"%APPDATA%\Bad");
        assert!(
            custom_pack(&[("recipes/bad.yaml", &recipe)]).is_err(),
            "{path} must be refused as a probe"
        );
        let recipe = one_step("bad", r"%APPDATA%\Bad", "copy", path);
        assert!(
            custom_pack(&[("recipes/bad.yaml", &recipe)]).is_err(),
            "{path} must be refused as a copy"
        );
    }
}

#[test]
fn a_version_2_plan_still_parses() {
    let v2 = r#"{
        "version": 2,
        "root": "C:\\Users\\me",
        "totals": {"review": {"bytes": 2, "files": 1}},
        "tree": {
            "path": "",
            "decision": "review",
            "source": {"by": "default"},
            "stats": {"bytes": 2, "files": 1},
            "unit": {"reason": "small"}
        },
        "split": {"split_above": 1, "min_unit": 1, "homogeneous_percent": 90, "max_depth": 8}
    }"#;
    let plan: Plan = serde_json::from_str(v2).expect("a v2 plan parses");
    assert_eq!(plan.version, 2);
    assert!(plan.applications.is_empty());
}

#[test]
fn a_parallel_scan_with_recipes_gives_the_same_plan_as_a_sequential_one() {
    let tree = large_tree()
        .file(".gitconfig", 1)
        .file("AppData/Roaming/Code/User/settings.json", 2)
        .file("AppData/Roaming/Code/Cache/Cache_Data/f", 3);
    let on = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("a thread pool")
            .install(|| serde_json::to_value(with_core_recipes(&tree)).unwrap())
    };
    let sequential = on(1);
    assert_eq!(on(8), sequential);
}

fn work_tree() -> MemTree {
    MemTree::new()
        .file("Work/client-a/a.pdf", 800)
        .file("Work/client-b/b.zip", 800)
        .file("Work/client-b/node_modules/x.js", 5_000)
        .file("Workshop/c.pdf", 800)
        .file("Documents/c.docx", 600)
}

#[test]
fn a_decision_from_a_parent_covers_every_folder_below_it_even_without_a_node() {
    let mut plan = cut(&work_tree());
    assert!(!has_node(&plan, "Work"));

    let changed = plan.apply_user_below("Work", Decision::Migrate);

    assert_eq!(changed, ["Work/client-a", "Work/client-b"]);
    for path in changed {
        assert_eq!(node(&plan, &path).decision, Decision::Migrate);
        assert_eq!(node(&plan, &path).source, Source::User);
    }
    assert_eq!(plan.totals[&Decision::Migrate].bytes, 1_600);
}

#[test]
fn a_decision_from_a_parent_keeps_what_rules_settled_below_it() {
    let mut plan = cut(&work_tree());
    plan.apply_user_below("Work", Decision::Migrate);

    let deps = node(&plan, "Work/client-b/node_modules");
    assert_eq!(deps.decision, Decision::Leave);
    assert_eq!(rule_of(deps), "node-modules");
}

#[test]
fn a_decision_from_a_parent_does_not_reach_a_sibling_with_a_longer_name() {
    let mut plan = cut(&work_tree());
    plan.apply_user_below("Work", Decision::Archive);

    let workshop = node(&plan, "Workshop");
    assert_eq!(workshop.decision, Decision::Review);
    assert_eq!(workshop.source, Source::Default);
}

#[test]
fn a_decision_from_a_parent_replaces_the_users_earlier_choices_below_it() {
    let mut plan = cut(&work_tree());
    assert!(plan.apply_user("Work/client-a", Decision::Leave));

    plan.apply_user_below("Work", Decision::Archive);

    assert_eq!(node(&plan, "Work/client-a").decision, Decision::Archive);
}

#[test]
fn a_decision_from_a_folder_with_nothing_to_decide_changes_nothing() {
    let mut plan = cut(&work_tree());
    let before = plan.clone();

    assert!(plan.apply_user_below("Nowhere", Decision::Leave).is_empty());
    assert!(
        plan.apply_user_below("Work/client-b/node_modules", Decision::Migrate)
            .is_empty()
    );
    assert_eq!(plan, before);
}

/// A Synology Drive sync folder: the client puts its marker at the root.
fn synced(tree: MemTree, root: &str) -> MemTree {
    tree.dir(&format!("{root}/.SynologyWorkingDirectory"))
}

fn is_synology_unit(node: &PlanNode) -> bool {
    node.decision == Decision::Review
        && node.unit
            == Some(UnitReason::Rule {
                rule: "synology-drive-folder".to_owned(),
                key: "synced-synology-drive".to_owned(),
                propose: None,
            })
}

#[test]
fn a_synology_drive_folder_is_one_unit_however_large() {
    let tree = synced(
        MemTree::new()
            .file("SynologyDrive/clients/a.pdf", 5_000)
            .file("SynologyDrive/photos/b.jpg", 5_000),
        "SynologyDrive",
    );
    let plan = cut(&tree);

    assert!(is_synology_unit(node(&plan, "SynologyDrive")));
    assert_eq!(node(&plan, "SynologyDrive").stats.bytes, 10_000);
    assert!(!has_node(&plan, "SynologyDrive/clients"));
}

#[test]
fn a_folder_named_like_synology_drive_without_its_marker_is_cut_as_usual() {
    let plan = cut(&MemTree::new()
        .file("SynologyDrive/clients/a.pdf", 5_000)
        .file("SynologyDrive/photos/b.jpg", 5_000)
        .dir("SynologyDrive/SynologyWorkingDirectory"));

    assert!(!has_node(&plan, "SynologyDrive"));
    assert!(has_node(&plan, "SynologyDrive/clients"));
}

#[test]
fn a_synced_folder_is_recognised_wherever_the_user_put_it() {
    let tree = synced(MemTree::new().file("Work/NAS/a.pdf", 5_000), "Work/NAS");
    assert!(is_synology_unit(node(&cut(&tree), "Work/NAS")));
}

#[test]
fn a_small_synced_folder_keeps_its_own_unit_inside_a_small_profile() {
    let tree = synced(
        MemTree::new()
            .file("Documents/cv.docx", 200)
            .file("SynologyDrive/a.pdf", 300),
        "SynologyDrive",
    );
    let plan = plan(&tree);

    assert!(is_synology_unit(node(&plan, "SynologyDrive")));
    assert_eq!(plan.tree.unit, Some(UnitReason::Small));
    assert_eq!(plan.tree.stats.bytes, 200);
    assert_eq!(plan.totals[&Decision::Review].bytes, 500);
}

#[test]
fn a_synced_folder_inside_a_folder_folded_into_a_rest_keeps_its_own_unit() {
    // `Work/tiny` is below `min_unit` and joins the rest of `Work`; the
    // synced folder inside it must not disappear with it.
    let tree = synced(
        MemTree::new()
            .file("Work/big/a.zip", 2_000)
            .file("Work/notes.txt", 200)
            .file("Work/tiny/sync/a.pdf", 50),
        "Work/tiny/sync",
    );
    let scan = scan(&tree, Path::new(""), &common::core_rules(), &small_budget()).unwrap();

    assert!(is_synology_unit(node(&scan.plan, "Work/tiny/sync")));
    assert!(scan.samples.contains_key("Work/tiny/sync"));
    assert!(!scan.samples.contains_key("Work/tiny"));
    let total: u64 = scan.plan.totals.values().map(|t| t.bytes).sum();
    assert_eq!(total, 2_250);
}

#[test]
fn rules_still_apply_inside_a_synced_folder() {
    let tree = synced(
        MemTree::new()
            .file("SynologyDrive/dev/app/main.rs", 100)
            .file("SynologyDrive/dev/app/node_modules/x.js", 5_000),
        "SynologyDrive",
    );
    let plan = cut(&tree);

    assert_eq!(node(&plan, "SynologyDrive").stats.bytes, 100);
    let deps = node(&plan, "SynologyDrive/dev/app/node_modules");
    assert_eq!(deps.decision, Decision::Leave);
}

#[test]
fn samples_are_kept_only_for_the_units_of_the_plan() {
    let scan = scan(
        &profile(),
        Path::new(""),
        &common::core_rules(),
        &small_budget(),
    )
    .unwrap();
    let units: Vec<&str> = scan
        .plan
        .nodes()
        .into_iter()
        .filter(|n| n.unit.is_some())
        .map(|n| n.path.as_str())
        .collect();
    let sampled: Vec<&str> = scan.samples.keys().map(String::as_str).collect();
    let mut units_sorted = units.clone();
    units_sorted.sort_unstable();
    assert_eq!(sampled, units_sorted);
}

fn unit_of(plan: &Plan, path: &str) -> UnitReason {
    node(plan, path)
        .unit
        .clone()
        .unwrap_or_else(|| panic!("`{path}` is no unit"))
}

fn unknown_application() -> UnitReason {
    UnitReason::Part {
        rule: "appdata-applications".to_owned(),
        key: "unknown-application".to_owned(),
    }
}

fn other_applications() -> UnitReason {
    UnitReason::Rule {
        rule: "appdata-applications".to_owned(),
        key: "other-applications".to_owned(),
        propose: None,
    }
}

fn proposed(plan: &Plan, path: &str) -> Option<Decision> {
    match unit_of(plan, path) {
        UnitReason::Rule { propose, .. } => propose,
        _ => None,
    }
}

#[test]
fn each_application_folder_in_appdata_is_one_card_however_large() {
    let plan = cut(&MemTree::new()
        .file("AppData/Local/BigApp/models/a.bin", 5_000)
        .file("AppData/Local/BigApp/data/b.db", 5_000)
        .file("AppData/Roaming/Editor/c.json", 2_000));

    assert_eq!(
        unit_of(&plan, "AppData/Local/BigApp"),
        unknown_application()
    );
    assert_eq!(
        unit_of(&plan, "AppData/Roaming/Editor"),
        unknown_application()
    );
    assert!(!has_node(&plan, "AppData/Local/BigApp/models"));
}

#[test]
fn small_unknown_applications_share_one_card_per_appdata_folder() {
    let plan = cut(&MemTree::new()
        .file("AppData/Local/BigApp/a.bin", 5_000)
        .file("AppData/Local/tiny-one/x.json", 10)
        .file("AppData/Local/tiny-two/y.json", 20));

    assert_eq!(unit_of(&plan, "AppData/Local"), other_applications());
    assert_eq!(node(&plan, "AppData/Local").stats.bytes, 30);
    assert!(!has_node(&plan, "AppData/Local/tiny-one"));
}

#[test]
fn a_small_appdata_folder_keeps_its_own_card_instead_of_joining_the_profile() {
    let plan = cut(&MemTree::new()
        .file("AppData/LocalLow/3f9a/save.dat", 10)
        .file("Documents/cv.docx", 600));

    assert_eq!(unit_of(&plan, "AppData/LocalLow"), other_applications());
    assert_eq!(plan.tree.stats.bytes, 600);
}

#[test]
fn a_known_application_has_its_own_card_with_a_proposal_even_when_small() {
    let plan = cut(&MemTree::new()
        .file("AppData/Roaming/Notepad++/config.xml", 5)
        .file("AppData/Roaming/Slack/storage/s.db", 50)
        .file("AppData/Local/Docker/wsl/disk.vhdx", 5_000));

    assert_eq!(
        proposed(&plan, "AppData/Roaming/Notepad++"),
        Some(Decision::Migrate)
    );
    assert_eq!(
        proposed(&plan, "AppData/Roaming/Slack"),
        Some(Decision::Leave)
    );
    assert_eq!(
        proposed(&plan, "AppData/Local/Docker"),
        Some(Decision::Archive)
    );
}

#[test]
fn a_proposal_is_never_a_decision() {
    let plan = cut(&MemTree::new()
        .file("AppData/Roaming/Slack/storage/s.db", 50)
        .file("AppData/Local/Docker/wsl/disk.vhdx", 5_000));

    for path in ["AppData/Roaming/Slack", "AppData/Local/Docker"] {
        assert_eq!(node(&plan, path).decision, Decision::Review);
        assert_eq!(node(&plan, path).source, Source::Default);
    }
    assert_eq!(plan.totals[&Decision::Leave].bytes, 0);
}

#[test]
fn a_known_application_inside_a_vendor_folder_has_its_own_card() {
    let plan = cut(&MemTree::new()
        .file(
            "AppData/Local/Google/Chrome/User Data/Default/IndexedDB/x",
            3_000,
        )
        .file("AppData/Local/Google/Updater/u.log", 2_000));

    assert_eq!(
        proposed(&plan, "AppData/Local/Google/Chrome"),
        Some(Decision::Archive)
    );
    assert_eq!(
        unit_of(&plan, "AppData/Local/Google"),
        unknown_application()
    );
    assert_eq!(node(&plan, "AppData/Local/Google").stats.bytes, 2_000);
}

#[test]
fn an_application_folder_outside_appdata_is_not_taken_for_one() {
    let plan = cut(&MemTree::new()
        .file("Documents/Slack/exports/a.json", 5_000)
        .file("Documents/Slack/exports/b.json", 5_000));

    assert!(!has_node(&plan, "Documents/Slack") || proposed(&plan, "Documents/Slack").is_none());
    assert!(
        plan.nodes()
            .iter()
            .all(|n| n.unit != Some(unknown_application()))
    );
}

fn shared_folder_part() -> UnitReason {
    UnitReason::Part {
        rule: "appdata-shared-folders".to_owned(),
        key: "unknown-application".to_owned(),
    }
}

#[test]
fn microsofts_folder_in_appdata_is_cut_into_one_card_per_application() {
    let plan = cut(&MemTree::new()
        .file("AppData/Local/Microsoft/Windows/Fonts/my-font.ttf", 5_000)
        .file(
            "AppData/Local/Microsoft/Edge/User Data/Default/IndexedDB/x",
            3_000,
        )
        .file("AppData/Local/Microsoft/Tiny/a.json", 10)
        .file("AppData/Local/Microsoft/Small/b.json", 20));

    assert_eq!(
        unit_of(&plan, "AppData/Local/Microsoft/Windows"),
        shared_folder_part()
    );
    assert_eq!(
        proposed(&plan, "AppData/Local/Microsoft/Edge"),
        Some(Decision::Archive)
    );
    assert_eq!(
        unit_of(&plan, "AppData/Local/Microsoft"),
        UnitReason::Rule {
            rule: "appdata-shared-folders".to_owned(),
            key: "other-applications".to_owned(),
            propose: None,
        }
    );
    assert_eq!(node(&plan, "AppData/Local/Microsoft").stats.bytes, 30);
}

#[test]
fn store_application_data_is_cut_into_one_card_per_package() {
    let plan = cut(&MemTree::new()
        .file(
            "AppData/Local/Packages/5319275A.WhatsApp_cv1g1/LocalState/m.db",
            5_000,
        )
        .file(
            "AppData/Local/Packages/SpotifyAB.Spotify_zpdn/LocalState/c",
            4_000,
        ));

    for package in ["5319275A.WhatsApp_cv1g1", "SpotifyAB.Spotify_zpdn"] {
        assert_eq!(
            unit_of(&plan, &format!("AppData/Local/Packages/{package}")),
            shared_folder_part()
        );
    }
}

#[test]
fn installers_and_components_microsoft_and_windows_bring_back_are_proposed_to_leave() {
    let plan = cut(&MemTree::new()
        .file("AppData/Local/Microsoft/WinGet/Packages/tool/tool.exe", 50)
        .file(
            "AppData/Local/Microsoft/TeamsMeetingAdd-in/1.0/addin.dll",
            50,
        )
        .file(
            "AppData/Local/Microsoft/TeamsMeetingAddinMsis/setup.msi",
            50,
        )
        .file("AppData/Local/JxBrowser/7.21/chromium.dll", 50)
        .file(
            "AppData/Local/Packages/MicrosoftWindows.Client.CBS_cw5n1h2txyewy/LocalState/s",
            50,
        )
        .file(
            "AppData/Local/Packages/MicrosoftWindows.Photos_8wek/LocalState/p",
            5_000,
        ));

    for path in [
        "AppData/Local/Microsoft/WinGet",
        "AppData/Local/Microsoft/TeamsMeetingAdd-in",
        "AppData/Local/Microsoft/TeamsMeetingAddinMsis",
        "AppData/Local/JxBrowser",
        "AppData/Local/Packages/MicrosoftWindows.Client.CBS_cw5n1h2txyewy",
    ] {
        assert_eq!(proposed(&plan, path), Some(Decision::Leave), "{path}");
    }
    assert_eq!(
        proposed(&plan, "AppData/Local/Packages/MicrosoftWindows.Photos_8wek"),
        None
    );
}

fn tool_folder() -> UnitReason {
    UnitReason::Part {
        rule: "root-dot-folders".to_owned(),
        key: "tool-folder".to_owned(),
    }
}

#[test]
fn a_dot_folder_at_the_profile_root_is_one_card_however_large() {
    let plan = cut(&MemTree::new()
        .file(".config/editor/a/settings.json", 3_000)
        .file(".config/shell/b/history", 3_000)
        .file("Documents/cv.docx", 600));

    assert_eq!(unit_of(&plan, ".config"), tool_folder());
    assert!(!has_node(&plan, ".config/editor"));
}

#[test]
fn a_small_dot_folder_joins_the_profile_card() {
    let profile = || {
        MemTree::new()
            .file("Documents/big/a.pdf", 5_000)
            .file("notes.txt", 200)
    };
    let without = cut(&profile());
    let plan = cut(&profile().file(".mcp-auth/token.json", 10));

    assert!(!has_node(&plan, ".mcp-auth"));
    assert_eq!(
        node(&plan, "").stats.bytes,
        node(&without, "").stats.bytes + 10
    );
}

#[test]
fn a_dot_folder_deeper_than_the_root_is_cut_as_usual() {
    let plan = cut(&MemTree::new()
        .file("dev/app/.git/objects/a/x", 3_000)
        .file("dev/app/.git/objects/b/y", 3_000));

    assert!(plan.nodes().iter().all(|n| n.unit != Some(tool_folder())));
}

#[test]
fn keys_in_a_dot_folder_are_still_migrated() {
    let plan = cut(&MemTree::new().file(".ssh/id_ed25519", 400));

    assert_eq!(node(&plan, ".ssh").decision, Decision::Migrate);
}

#[test]
fn known_dot_folders_come_with_a_proposal_and_their_caches_stay_left() {
    let plan = cut(&MemTree::new()
        .file(
            ".vscode/extensions/rust-lang.rust-analyzer/server.exe",
            3_000,
        )
        .file(".cargo/bin/cargo-about.exe", 3_000)
        .file(".cargo/registry/cache/index/a.crate", 3_000)
        .file(".cargo/config.toml", 1)
        .file(".local/bin/tool.exe", 3_000)
        .file(".cache/puppeteer/chrome.zip", 3_000)
        .file(".docker/config.json", 1)
        .file(".rustup/settings.toml", 1));

    assert_eq!(proposed(&plan, ".vscode"), Some(Decision::Leave));
    assert_eq!(proposed(&plan, ".cargo/bin"), Some(Decision::Leave));
    assert_eq!(proposed(&plan, ".local/bin"), Some(Decision::Leave));
    assert_eq!(proposed(&plan, ".cache"), Some(Decision::Leave));
    assert_eq!(proposed(&plan, ".docker"), Some(Decision::Migrate));
    assert_eq!(proposed(&plan, ".rustup"), Some(Decision::Migrate));
    assert_eq!(node(&plan, ".cargo/registry").decision, Decision::Leave);
}

#[test]
fn caches_and_recipes_still_settle_their_folders_inside_an_application() {
    let tree = MemTree::new()
        .file("AppData/Local/Temp/a.tmp", 4_000)
        .file("AppData/Roaming/Code/User/settings.json", 2)
        .file("AppData/Roaming/Code/Cache/c", 3_000)
        .file("AppData/Roaming/Code/History/h", 100);
    let plan = with_core_recipes(&tree);

    assert_eq!(node(&plan, "AppData/Local/Temp").decision, Decision::Leave);
    assert_eq!(
        node(&plan, "AppData/Roaming/Code/Cache").decision,
        Decision::Leave
    );
    assert_eq!(
        recipe_of(node(&plan, "AppData/Roaming/Code/User/settings.json")),
        "vscode"
    );
    assert_eq!(node(&plan, "AppData/Roaming/Code").stats.bytes, 100);
}

#[test]
fn appdata_holding_only_settled_folders_is_no_card() {
    let plan = cut(&MemTree::new()
        .file("AppData/Local/Temp/a.tmp", 4_000)
        .file("Documents/cv.docx", 600));

    assert!(
        !plan
            .nodes()
            .iter()
            .any(|n| n.path.starts_with("AppData") && n.unit.is_some())
    );
    assert_eq!(node(&plan, "AppData/Local/Temp").decision, Decision::Leave);
}

#[test]
fn a_unit_does_not_list_a_sub_folder_whose_content_all_has_its_own_card() {
    let tree = MemTree::new()
        .file("AppData/Roaming/Slack/s.db", 50)
        .file("Documents/cv.docx", 600);
    let scan = scan(&tree, Path::new(""), &common::core_rules(), &small_budget()).unwrap();

    let folders: Vec<&str> = scan.samples[""]
        .folders
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    assert_eq!(folders, ["Documents"]);
}

// Programs to install again: the folders that show a program is installed.

#[test]
fn each_folder_in_the_per_user_programs_folder_is_a_program_to_install_again() {
    let plan = plan(
        &MemTree::new()
            .file("AppData/Local/Programs/Microsoft VS Code/Code.exe", 900)
            .file("AppData/Local/Programs/Python/Python312/python.exe", 300)
            .file("AppData/Local/Programs/readme.txt", 1),
    );
    let programs: Vec<(&str, &str, &str)> = plan
        .programs
        .iter()
        .map(|p| (p.path.as_str(), p.name.as_str(), p.rule.as_str()))
        .collect();
    assert_eq!(
        programs,
        [
            (
                "AppData/Local/Programs/Microsoft VS Code",
                "Microsoft VS Code",
                "user-installed-programs"
            ),
            (
                "AppData/Local/Programs/Python",
                "Python",
                "user-installed-programs"
            ),
        ]
    );
    // Listing them changes no decision: the folder is still left.
    assert_eq!(
        node(&plan, "AppData/Local/Programs").decision,
        Decision::Leave
    );
    assert!(
        plan.programs
            .iter()
            .all(|p| p.reinstall == bellcart::plan::Reinstall::Undecided)
    );
}

#[test]
fn a_known_program_folder_is_listed_under_its_name_and_keeps_its_card() {
    let plan = plan(&MemTree::new().file("AppData/Local/slack/slack.exe", 900));
    assert_eq!(plan.programs.len(), 1);
    assert_eq!(plan.programs[0].path, "AppData/Local/slack");
    assert_eq!(plan.programs[0].name, "Slack");
    let card = node(&plan, "AppData/Local/slack");
    assert_eq!(card.decision, Decision::Review);
}

#[test]
fn a_folder_named_programs_elsewhere_lists_no_program() {
    let plan = plan(&MemTree::new().file("Documents/Programs/Old tool/tool.exe", 900));
    assert!(plan.programs.is_empty());
    assert_eq!(node(&plan, "").decision, Decision::Review);
}

#[test]
fn the_program_list_does_not_depend_on_the_number_of_threads() {
    let tree = MemTree::new()
        .file("AppData/Local/Programs/b/b.exe", 10)
        .file("AppData/Local/Programs/a/a.exe", 10)
        .file("AppData/Local/Discord/Discord.exe", 10)
        .file("AppData/Local/slack/slack.exe", 10);
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap()
        .install(|| plan(&tree));
    let many = plan(&tree);
    assert_eq!(one.programs, many.programs);
    let paths: Vec<&str> = many.programs.iter().map(|p| p.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "AppData/Local/Discord",
            "AppData/Local/Programs/a",
            "AppData/Local/Programs/b",
            "AppData/Local/slack"
        ]
    );
}

#[test]
fn the_user_choice_to_install_a_program_again_is_kept_in_the_plan() {
    use bellcart::plan::Reinstall;
    let mut plan = plan(&MemTree::new().file("AppData/Local/Programs/Python/python.exe", 10));
    let totals = plan.totals.clone();
    assert!(plan.set_reinstall("AppData/Local/Programs/Python", Reinstall::Yes));
    assert_eq!(plan.programs[0].reinstall, Reinstall::Yes);
    assert!(!plan.set_reinstall("AppData/Local/Programs/Nope", Reinstall::No));
    assert_eq!(plan.totals, totals);
}

// A whole drive: the profile is one folder in it, and drive rules settle
// what Windows and installers keep outside it.

fn drive_plan(tree: &MemTree) -> Plan {
    let pack = common::core_pack();
    let found = Recipes::detect(
        tree,
        Path::new(""),
        &pack.recipes,
        &Variables::profile_at("Users/ana"),
    );
    bellcart::scan::scan_drive(
        tree,
        Path::new(""),
        "Users/ana",
        &common::core_rules(),
        &found,
        &SplitBudget::default(),
        &Progress::default(),
    )
    .expect("the scan succeeds")
    .plan
}

#[test]
fn windows_and_program_files_are_left_outside_the_profile_only() {
    let plan = drive_plan(
        &MemTree::new()
            .file("Windows/System32/kernel32.dll", 900)
            .file("Program Files/7-Zip/7z.exe", 500)
            .file("Users/ana/Documents/Windows/notes.txt", 10)
            .file("Users/ana/Documents/Program Files/plan.txt", 10),
    );
    assert_eq!(plan.profile.as_deref(), Some("Users/ana"));
    assert_eq!(rule_of(node(&plan, "Windows")), "drive-windows");
    assert_eq!(rule_of(node(&plan, "Program Files")), "drive-program-files");
    // Folders with the same names inside the profile are the person's.
    assert!(!has_node(&plan, "Users/ana/Documents/Windows"));
    assert_eq!(plan.totals[&Decision::Review].bytes, 20);
}

#[test]
fn drive_rules_never_apply_when_the_profile_is_the_root() {
    let plan = plan(&MemTree::new().file("Windows/notes.txt", 10));
    assert!(!has_node(&plan, "Windows"));
    assert_eq!(plan.totals[&Decision::Review].bytes, 10);
    assert_eq!(plan.profile, None);
}

#[test]
fn profile_rules_see_paths_from_the_profile_in_a_drive() {
    let plan = drive_plan(
        &MemTree::new()
            .file("Users/ana/AppData/Local/Temp/a.tmp", 300)
            .file("Users/ana/.cache/pip/x", 200)
            .file("Users/ana/Documents/cv.docx", 100),
    );
    assert_eq!(
        rule_of(node(&plan, "Users/ana/AppData/Local/Temp")),
        "user-temp"
    );
    assert!(matches!(
        &node(&plan, "Users/ana/.cache").unit,
        Some(UnitReason::Rule { rule, .. }) if rule == "app-dot-cache"
    ));
}

#[test]
fn a_folder_no_rule_knows_at_the_drive_root_is_reviewed_and_its_builds_are_left() {
    let plan = drive_plan(
        &MemTree::new()
            .file("dev/app/src/main.rs", 150)
            .file("dev/app/node_modules/x.js", 5_000)
            .file("Photos/2024/a.jpg", 2_000),
    );
    assert_eq!(rule_of(node(&plan, "dev/app/node_modules")), "node-modules");
    assert_eq!(plan.totals[&Decision::Leave].bytes, 5_000);
    assert_eq!(plan.totals[&Decision::Review].bytes, 2_150);
}

#[test]
fn programs_in_program_files_are_listed_once_without_windows_components() {
    let plan = drive_plan(
        &MemTree::new()
            .file("Program Files/7-Zip/7z.exe", 10)
            .file("Program Files/Google/Chrome/chrome.exe", 10)
            .file("Program Files (x86)/Google/Update/u.exe", 10)
            .file("Program Files/Windows Defender/m.exe", 10)
            .file("Program Files/Common Files/x.dll", 10)
            .file("Program Files (x86)/Mozilla Maintenance Service/m.exe", 10),
    );
    let names: Vec<&str> = plan.programs.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Google", "7-Zip"]);
}

#[test]
fn the_page_file_is_left_at_the_drive_root_only() {
    let plan = drive_plan(
        &MemTree::new()
            .file("pagefile.sys", 8_000)
            .file("hiberfil.sys", 4_000)
            .file("Users/ana/Documents/pagefile.sys", 10)
            .file("backup/pagefile.sys", 10),
    );
    assert_eq!(rule_of(node(&plan, "pagefile.sys")), "drive-system-files");
    assert_eq!(rule_of(node(&plan, "hiberfil.sys")), "drive-system-files");
    assert_eq!(plan.totals[&Decision::Leave].bytes, 12_000);
    assert_eq!(plan.totals[&Decision::Review].bytes, 20);
}

#[test]
fn a_previous_windows_installation_is_its_own_card_never_left() {
    let plan = drive_plan(&MemTree::new().file("Windows.old/Users/ana/Documents/cv.docx", 100));
    let card = node(&plan, "Windows.old");
    assert_eq!(card.decision, Decision::Review);
    assert!(matches!(
        &card.unit,
        Some(UnitReason::Rule { rule, propose: None, .. }) if rule == "drive-previous-windows"
    ));
}

#[test]
fn recipes_find_applications_in_the_profile_of_a_drive() {
    let plan = drive_plan(
        &MemTree::new()
            .file("Users/ana/.gitconfig", 10)
            .file(".gitconfig", 10),
    );
    assert_eq!(plan.applications.len(), 1);
    assert_eq!(
        node(&plan, "Users/ana/.gitconfig").decision,
        Decision::Migrate
    );
    assert!(!has_node(&plan, ".gitconfig"));
}

#[test]
fn program_data_is_cut_into_application_cards_and_windows_keeps_its_own() {
    let plan = drive_plan(
        &MemTree::new()
            .file("ProgramData/Microsoft/Search/index.edb", 900)
            .file("ProgramData/Package Cache/{guid}/setup.msi", 700)
            // Large enough for a card of its own.
            .file("ProgramData/Vendor/faxes/fax1.tif", 300 << 20),
    );
    assert_eq!(
        rule_of(node(&plan, "ProgramData/Microsoft")),
        "drive-program-data-system"
    );
    assert_eq!(
        rule_of(node(&plan, "ProgramData/Package Cache")),
        "drive-package-cache"
    );
    assert!(matches!(
        node(&plan, "ProgramData/Vendor").unit,
        Some(UnitReason::Part { .. })
    ));
}

#[test]
fn the_update_assistant_and_intel_installers_found_on_windows_10_are_left_on_the_drive_only() {
    let plan = drive_plan(
        &MemTree::new()
            .file("Windows10Upgrade/Windows10.esd", 3_000)
            .file("ProgramData/Intel Package Cache {1CEAC85D}/setup.exe", 400)
            .file("$WINRE_BACKUP_PARTITION.MARKER", 1)
            .file("Users/ana/Documents/Windows10Upgrade/notes.txt", 10),
    );
    assert_eq!(
        rule_of(node(&plan, "Windows10Upgrade")),
        "drive-system-folders"
    );
    assert_eq!(
        rule_of(node(&plan, "ProgramData/Intel Package Cache {1CEAC85D}")),
        "drive-package-cache"
    );
    assert_eq!(plan.totals[&Decision::Leave].bytes, 3_401);
    assert_eq!(plan.totals[&Decision::Review].bytes, 10);
}

#[test]
fn runtimes_and_installer_folders_in_program_files_are_not_programs() {
    let plan = drive_plan(
        &MemTree::new()
            .file("Program Files/dotnet/dotnet.exe", 10)
            .file(
                "Program Files (x86)/InstallShield Installation Information/x.ini",
                10,
            )
            .file("Program Files (x86)/Microsoft SDKs/x.dll", 10)
            .file("Program Files/RUXIM/r.exe", 10)
            .file("Program Files/Notepad++/notepad++.exe", 10),
    );
    let names: Vec<&str> = plan.programs.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Notepad++"]);
}

// Found on a Windows 10 profile.

#[test]
fn package_installer_and_web_caches_found_on_windows_10_are_left() {
    let plan = plan(
        &MemTree::new()
            .file("AppData/Roaming/npm-cache/_cacache/x", 10)
            .file("AppData/Local/pnpm-cache/metadata/x", 10)
            .file("AppData/Local/NuGet/v3-cache/x.dat", 10)
            .file("AppData/Local/SquirrelTemp/setup.exe", 10)
            .file(
                "AppData/Local/Microsoft/Windows/WebCache/WebCacheV01.dat",
                10,
            ),
    );
    for (path, rule) in [
        ("AppData/Roaming/npm-cache", "npm-cache"),
        ("AppData/Local/pnpm-cache", "pnpm-cache"),
        ("AppData/Local/NuGet/v3-cache", "nuget-http-cache"),
        ("AppData/Local/SquirrelTemp", "squirrel-temp"),
        (
            "AppData/Local/Microsoft/Windows/WebCache",
            "windows-web-cache",
        ),
    ] {
        assert_eq!(rule_of(node(&plan, path)), rule, "{path}");
        assert_eq!(node(&plan, path).decision, Decision::Leave, "{path}");
    }
}

#[test]
fn folders_named_like_those_caches_in_documents_stay_to_review() {
    let plan = plan(
        &MemTree::new()
            .file("Documents/npm-cache/notes.txt", 10)
            .file("Documents/NuGet/Cache/notes.txt", 10)
            .file("Documents/WebCache/notes.txt", 10)
            .file("Documents/Package Cache/notes.txt", 10)
            .file("AppData/Local/NuGet/NuGet.Config", 10),
    );
    assert_eq!(plan.totals[&Decision::Leave].bytes, 0);
}

#[test]
fn windows_search_activity_history_and_installers_are_proposed_to_leave() {
    let plan = plan(
        &MemTree::new()
            .file(
                "AppData/Local/Packages/Microsoft.Windows.Search_cw5n1h2txyewy/LocalState/x",
                10,
            )
            .file(
                "AppData/Local/ConnectedDevicesPlatform/L.user/ActivitiesCache.db",
                10,
            )
            .file("AppData/Local/Docker Desktop Installer/install.exe", 10)
            .file("AppData/LocalLow/Oracle/Java/jre.msi", 10),
    );
    for (path, rule) in [
        (
            "AppData/Local/Packages/Microsoft.Windows.Search_cw5n1h2txyewy",
            "app-windows-search",
        ),
        (
            "AppData/Local/ConnectedDevicesPlatform",
            "app-connected-devices",
        ),
        (
            "AppData/Local/Docker Desktop Installer",
            "app-docker-desktop-installer",
        ),
        ("AppData/LocalLow/Oracle/Java", "app-java-deployment"),
    ] {
        let card = node(&plan, path);
        assert_eq!(card.decision, Decision::Review, "{path}");
        assert!(
            matches!(&card.unit, Some(UnitReason::Rule { rule: r, propose: Some(Decision::Leave), .. }) if r == rule),
            "{path}: {:?}",
            card.unit
        );
    }
}
