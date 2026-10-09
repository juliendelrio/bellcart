//! Human-readable output. No decision is taken here.

use std::fmt::Write as _;

use bellcart::pack::Pack;
use bellcart::plan::{Decision, Plan, PlanNode, Source};

use crate::i18n::Localizer;

pub fn plan(plan: &Plan, text: &Localizer) -> String {
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

    let mut settled: Vec<&PlanNode> = plan
        .nodes()
        .into_iter()
        .filter(|n| matches!(n.source, Source::Rule { .. }))
        .collect();
    settled.sort_by_key(|node| std::cmp::Reverse(node.stats.bytes));

    out.push('\n');
    if settled.is_empty() {
        let _ = writeln!(out, "{}", text.get("plan-nothing-settled"));
    } else {
        let _ = writeln!(out, "{}", text.get("plan-settled"));
        for node in settled {
            let Source::Rule { reason, .. } = &node.source else {
                continue;
            };
            let label = text.get(&format!("decision-{}", node.decision.as_str()));
            let _ = writeln!(
                out,
                "  {:>10}  {label:<width$}  {}  ({})",
                human_bytes(node.stats.bytes),
                node.path,
                text.get(&format!("reason-{reason}")),
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
        let _ = writeln!(out, "    {}", recipe.name);
    }
    out
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_are_readable() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1023), "1023 B");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes(42 * 1024 * 1024 * 1024), "42.0 GB");
    }
}
