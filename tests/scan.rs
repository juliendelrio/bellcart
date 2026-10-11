//! What a scan must decide. These cases are the specification: when
//! behaviour changes, a case here changes first.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::path::Path;

use bellcart::plan::{Decision, Plan, PlanNode, Source, SplitBudget, UnitReason};
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
            .file("report.2024", 40),
    );

    let extensions = &plan.tree.stats.extensions;
    // Split archives number their parts with three digits.
    assert_eq!(extensions["001"], 20);
    // Longer runs of digits are counters, process numbers or years.
    assert_eq!(extensions[""], 50);
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
