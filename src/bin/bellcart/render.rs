//! Human-readable output. No decision is taken here.

use std::fmt::Write as _;

use bellcart::pack::{Pack, Recipe};
use bellcart::plan::{Decision, Plan, PlanNode, Source, UnitReason, human_bytes};

use crate::i18n::Localizer;

/// `recipes` give the names of the applications found in the interface
/// language; the plan only records them in English.
pub fn plan(plan: &Plan, recipes: &[Recipe], text: &Localizer) -> String {
    let name_of = |id: &str| application_name(id, plan, recipes, text);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{}\n",
        text.with("plan-title", &[("root", plan.root.as_str().into())])
    );

    let labels: Vec<(Decision, String)> = Decision::ALL
        .into_iter()
        .map(|d| (d, text.get(&format!("decision-{}", d.as_str()))))
        .collect();
    let width = labels
        .iter()
        .map(|(_, l)| l.chars().count())
        .max()
        .unwrap_or(0);

    for (decision, label) in &labels {
        let totals = plan.totals.get(decision).copied().unwrap_or_default();
        let files = text.with("plan-files", &[("count", totals.files.into())]);
        let _ = writeln!(
            out,
            "  {label:<width$}  {:>10}  {files}",
            human_bytes(totals.bytes)
        );
    }

    if !plan.applications.is_empty() {
        let names: Vec<String> = plan
            .applications
            .iter()
            .map(|a| name_of(&a.recipe))
            .collect();
        let _ = writeln!(
            out,
            "\n{}",
            text.with("plan-applications", &[("names", names.join(", ").into())])
        );
    }

    let mut settled: Vec<&PlanNode> = plan
        .nodes()
        .into_iter()
        .filter(|n| !matches!(n.source, Source::Default))
        .collect();
    settled.sort_by_key(|node| std::cmp::Reverse(node.stats.bytes));

    out.push('\n');
    if settled.is_empty() {
        let _ = writeln!(out, "{}", text.get("plan-nothing-settled"));
    } else {
        let _ = writeln!(out, "{}", text.get("plan-settled"));
        // Empty folders a rule settled are right, but a line each is noise.
        let (empty, settled): (Vec<&PlanNode>, Vec<&PlanNode>) = settled
            .into_iter()
            .partition(|n| n.stats.bytes == 0 && n.stats.files == 0);
        for node in settled {
            let Some(why) = why(node, plan, recipes, text) else {
                continue;
            };
            let label = text.get(&format!("decision-{}", node.decision.as_str()));
            let _ = writeln!(
                out,
                "  {:>10}  {label:<width$}  {}  ({why})",
                human_bytes(node.stats.bytes),
                node.path,
            );
        }
        if !empty.is_empty() {
            let _ = writeln!(
                out,
                "  {}",
                text.with("plan-settled-empty", &[("count", empty.len().into())])
            );
        }
    }

    let mut units: Vec<&PlanNode> = plan
        .nodes()
        .into_iter()
        .filter(|n| n.decision == Decision::Review && n.unit.is_some())
        .collect();
    units.sort_by_key(|node| std::cmp::Reverse(node.stats.bytes));
    if !units.is_empty() {
        let _ = writeln!(out, "\n{}", text.get("plan-to-review"));
        for node in units {
            let Some(unit) = &node.unit else {
                continue;
            };
            let path = if node.path.is_empty() {
                "."
            } else {
                node.path.as_str()
            };
            let _ = writeln!(
                out,
                "  {:>10}  {path}  ({})",
                human_bytes(node.stats.bytes),
                unit_reason(unit, text),
            );
        }
    }

    let unreadable: u64 = plan.nodes().iter().map(|n| n.stats.unreadable).sum();
    if unreadable > 0 {
        let _ = writeln!(
            out,
            "\n{}",
            text.with("plan-unreadable", &[("count", unreadable.into())])
        );
    }
    out
}

pub fn pack(pack: &Pack, text: &Localizer) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{}",
        text.with(
            "packs-title",
            &[
                ("name", pack.meta.name.as_str().into()),
                ("version", pack.meta.version.as_str().into()),
            ],
        )
    );
    let questions = pack.questions.as_ref().map_or(0, |q| q.questions.len());
    for (key, count) in [
        ("packs-rules", pack.rules.len()),
        ("packs-recipes", pack.recipes.len()),
        ("packs-questions", questions),
    ] {
        let _ = writeln!(out, "  {}", text.with(key, &[("count", count.into())]));
    }
    for recipe in &pack.recipes {
        let _ = writeln!(out, "    {}", recipe.name.get(text.lang()));
    }
    out
}

/// Why a node has its decision, in the interface language. `None` for a
/// node no one decided.
pub fn why(node: &PlanNode, plan: &Plan, recipes: &[Recipe], text: &Localizer) -> Option<String> {
    Some(match &node.source {
        Source::Rule { reason, .. } => text.get(&format!("reason-{reason}")),
        Source::Recipe { recipe } => text.with(
            "source-recipe",
            &[("name", application_name(recipe, plan, recipes, text).into())],
        ),
        Source::Arbiter { backend, .. } => {
            text.with("source-arbiter", &[("backend", backend.as_str().into())])
        }
        Source::User => text.get("source-user"),
        Source::Default => return None,
    })
}

/// The name of an application found, in the interface language when its
/// recipe is loaded, else as the plan recorded it.
pub fn application_name(id: &str, plan: &Plan, recipes: &[Recipe], text: &Localizer) -> String {
    recipes.iter().find(|r| r.id == id).map_or_else(
        || {
            plan.applications
                .iter()
                .find(|a| a.recipe == id)
                .map_or(id.to_owned(), |a| a.name.clone())
        },
        |r| r.name.get(text.lang()).to_owned(),
    )
}

pub fn unit_reason(unit: &UnitReason, text: &Localizer) -> String {
    match unit {
        UnitReason::Small => text.get("unit-small"),
        UnitReason::Homogeneous { extension, percent } if extension.is_empty() => text.with(
            "unit-homogeneous-no-extension",
            &[("percent", u64::from(*percent).into())],
        ),
        UnitReason::Homogeneous { extension, percent } => text.with(
            "unit-homogeneous",
            &[
                ("extension", extension.as_str().into()),
                ("percent", u64::from(*percent).into()),
            ],
        ),
        UnitReason::DepthLimit => text.get("unit-depth-limit"),
        UnitReason::Leaf => text.get("unit-leaf"),
        UnitReason::Rest => text.get("unit-rest"),
        UnitReason::Rule { key, .. } | UnitReason::Part { key, .. } => {
            text.get(&format!("reason-{key}"))
        }
    }
}
