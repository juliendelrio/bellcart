use std::path::PathBuf;

use bellcart::pack::{Pack, RuleSet};

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn core_pack() -> Pack {
    Pack::from_dir(&repo_root().join("packs/core")).expect("the core pack loads")
}

#[allow(dead_code)]
pub fn core_rules() -> RuleSet {
    RuleSet::compile(&core_pack().rules).expect("the core rules compile")
}

/// A budget scaled to tiny test trees: folders above 1000 bytes are cut,
/// sub-folders under 100 bytes stay in their parent's rest.
#[allow(dead_code)]
pub fn small_budget() -> bellcart::plan::SplitBudget {
    bellcart::plan::SplitBudget {
        split_above: 1_000,
        min_unit: 100,
        homogeneous_percent: 90,
        max_depth: 4,
    }
}
