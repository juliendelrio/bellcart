//! JSON Schemas for the pack files and the plan, generated from the types.
//!
//! The files in `schemas/` are checked in so editors can use them. A test
//! fails when they drift from the types; regenerate them with
//! `BELLCART_UPDATE_SCHEMAS=1 cargo test --test schemas`.

use schemars::schema_for;

use crate::pack::{PackMeta, QuestionSet, Recipe, RuleFile};
use crate::plan::Plan;

/// `(file name, pretty-printed schema)` for every published format.
pub fn all() -> Vec<(&'static str, String)> {
    vec![
        ("pack.schema.json", render(&schema_for!(PackMeta))),
        ("rules.schema.json", render(&schema_for!(RuleFile))),
        ("recipe.schema.json", render(&schema_for!(Recipe))),
        ("questions.schema.json", render(&schema_for!(QuestionSet))),
        ("plan.schema.json", render(&schema_for!(Plan))),
    ]
}

fn render(schema: &schemars::Schema) -> String {
    let mut text = serde_json::to_string_pretty(schema).unwrap_or_default();
    text.push('\n');
    text
}
