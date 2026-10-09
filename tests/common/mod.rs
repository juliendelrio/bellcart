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
