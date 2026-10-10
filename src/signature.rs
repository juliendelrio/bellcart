//! Signatures: what an arbiter, or a person, reads about one review unit.
//!
//! The text form is the arbiter's `state`. It is fixed English and never
//! translated: it is input for a model, and the same unit must give the
//! same signature whatever the interface language. It holds names, sizes,
//! dates and extensions, never file contents.
//!
//! [`Signature`] keeps the data apart from the text, so that a redacted
//! form can mask names later without changing what is collected.

use std::fmt::Write as _;

use crate::plan::{Decision, PlanNode, Stats, UnitReason, human_bytes};
use crate::scan::Scan;

/// Most entries kept per list of names during the scan.
pub const SAMPLE_SIZE: usize = 20;
/// About 800 tokens, at a conservative 4 characters per token.
pub const MAX_CHARS: usize = 3_200;
/// Longest name shown in full.
const MAX_NAME_CHARS: usize = 60;
/// Most extensions and settled folders listed before the text is shrunk.
const MAX_EXTENSIONS: usize = 8;
const MAX_SETTLED: usize = 10;

/// A file or folder and its size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub name: String,
    pub bytes: u64,
}

/// The largest entries lying directly in a review unit.
///
/// For a unit that was not cut, `folders` are all its sub-folders still to
/// review. For the rest of a folder that was cut, they are only the small
/// sub-folders folded into that rest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sample {
    /// Largest first, at most [`SAMPLE_SIZE`].
    pub folders: Vec<Named>,
    /// How many folders there are in all.
    pub folder_count: usize,
    /// Largest first, at most [`SAMPLE_SIZE`].
    pub files: Vec<Named>,
    pub file_count: usize,
}

impl Sample {
    pub fn new(folders: Vec<Named>, files: Vec<Named>) -> Self {
        Self {
            folder_count: folders.len(),
            folders: largest(folders),
            file_count: files.len(),
            files: largest(files),
        }
    }
}

/// Sorts largest first, then by name so that ties are stable.
fn largest(mut entries: Vec<Named>) -> Vec<Named> {
    entries.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    entries.truncate(SAMPLE_SIZE);
    entries
}

/// Everything a signature says about one review unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub path: String,
    pub unit: UnitReason,
    /// What there is to review, settled folders excluded.
    pub stats: Stats,
    pub sample: Sample,
    /// Folders settled by a rule inside the unit, relative to it.
    pub settled: Vec<(String, Decision)>,
}

/// One signature per review unit, largest first.
pub fn signatures(scan: &Scan) -> Vec<Signature> {
    let mut out: Vec<Signature> = scan
        .plan
        .nodes()
        .into_iter()
        .filter(|node| node.decision == Decision::Review)
        .filter_map(|node| {
            Some(Signature {
                path: node.path.clone(),
                unit: node.unit.clone()?,
                stats: node.stats.clone(),
                sample: scan.samples.get(&node.path).cloned().unwrap_or_default(),
                settled: settled_inside(node),
            })
        })
        .collect();
    out.sort_by(|a, b| {
        b.stats
            .bytes
            .cmp(&a.stats.bytes)
            .then_with(|| a.path.cmp(&b.path))
    });
    out
}

fn settled_inside(node: &PlanNode) -> Vec<(String, Decision)> {
    node.overrides
        .iter()
        .filter(|o| o.decision != Decision::Review)
        .map(|o| (relative(&node.path, &o.path), o.decision))
        .collect()
}

fn relative(base: &str, path: &str) -> String {
    if base.is_empty() {
        return path.to_owned();
    }
    path.strip_prefix(base)
        .and_then(|p| p.strip_prefix('/'))
        .unwrap_or(path)
        .to_owned()
}

/// How many entries of each list the text shows.
struct Limits {
    extensions: usize,
    folders: usize,
    files: usize,
    settled: usize,
}

impl Limits {
    /// Shows one entry less: first from the longest list of names, then
    /// from the extensions. `false` when there is nothing left to drop.
    fn shrink(&mut self) -> bool {
        let names = [&mut self.folders, &mut self.files, &mut self.settled];
        if let Some(longest) = names.into_iter().filter(|n| **n > 0).max_by_key(|n| **n) {
            *longest -= 1;
            return true;
        }
        if self.extensions > 1 {
            self.extensions -= 1;
            return true;
        }
        false
    }
}

impl Signature {
    /// The text sent to an arbiter, at most [`MAX_CHARS`] characters.
    ///
    /// When it is too long, names are dropped from the longest list first,
    /// then extensions, always in the same order, so the same unit always
    /// gives the same text.
    pub fn to_text(&self) -> String {
        let mut limits = Limits {
            extensions: self.stats.extensions.len().min(MAX_EXTENSIONS),
            folders: self.sample.folders.len(),
            files: self.sample.files.len(),
            settled: self.settled.len().min(MAX_SETTLED),
        };
        loop {
            let text = self.render(&limits);
            if text.chars().count() <= MAX_CHARS || !limits.shrink() {
                return text.chars().take(MAX_CHARS).collect();
            }
        }
    }

    fn render(&self, limits: &Limits) -> String {
        let stats = &self.stats;
        let mut out = String::new();
        let path = if self.path.is_empty() {
            "."
        } else {
            self.path.as_str()
        };
        let _ = writeln!(out, "path: {path}");
        let _ = writeln!(out, "unit: {}", describe(&self.unit));
        let _ = writeln!(
            out,
            "size: {} in {}",
            human_bytes(stats.bytes),
            count(stats.files, "file")
        );
        if let (Some(oldest), Some(newest)) = (stats.oldest, stats.newest) {
            let _ = writeln!(
                out,
                "modified: {} to {}",
                year_month(oldest),
                year_month(newest)
            );
        }
        if stats.unreadable > 0 {
            let _ = writeln!(out, "unreadable: {}", count(stats.unreadable, "folder"));
        }
        if !stats.extensions.is_empty() {
            let _ = writeln!(out, "extensions: {}", extensions(stats, limits.extensions));
        }
        let sample = &self.sample;
        if sample.folder_count > 0 {
            let shown = sample.folders.iter().take(limits.folders);
            let _ = writeln!(
                out,
                "folders ({}): {}",
                sample.folder_count,
                names(shown, sample.folder_count)
            );
        }
        if sample.file_count > 0 {
            let shown = sample.files.iter().take(limits.files);
            let _ = writeln!(
                out,
                "files ({}): {}",
                sample.file_count,
                names(shown, sample.file_count)
            );
        }
        if !self.settled.is_empty() {
            let mut items: Vec<String> = self
                .settled
                .iter()
                .take(limits.settled)
                .map(|(path, decision)| format!("{} ({})", shorten(path), decision.as_str()))
                .collect();
            push_more(&mut items, self.settled.len());
            let _ = writeln!(out, "settled inside: {}", items.join(", "));
        }
        out
    }
}

fn describe(unit: &UnitReason) -> String {
    match unit {
        UnitReason::Small => "small enough to judge as a whole".to_owned(),
        UnitReason::Homogeneous { extension, percent } if extension.is_empty() => {
            format!("mostly files without an extension ({percent}%)")
        }
        UnitReason::Homogeneous { extension, percent } => {
            format!("mostly .{} files ({percent}%)", shorten(extension))
        }
        UnitReason::DepthLimit => "deep folder, not cut further".to_owned(),
        UnitReason::Leaf => "no sub-folder large enough to judge apart".to_owned(),
        UnitReason::Rest => {
            "files directly in a folder that was cut, and its small sub-folders".to_owned()
        }
    }
}

fn count(n: u64, what: &str) -> String {
    if n == 1 {
        format!("1 {what}")
    } else {
        format!("{n} {what}s")
    }
}

fn names<'a>(shown: impl Iterator<Item = &'a Named>, total: usize) -> String {
    let mut items: Vec<String> = shown
        .map(|n| format!("{} {}", shorten(&n.name), human_bytes(n.bytes)))
        .collect();
    push_more(&mut items, total);
    items.join(", ")
}

/// Ends a list with "N more" when it shows fewer than `total` entries.
fn push_more(items: &mut Vec<String>, total: usize) {
    let hidden = total.saturating_sub(items.len());
    if hidden > 0 {
        items.push(format!("{hidden} more"));
    }
}

/// Extensions by share of the bytes, largest first, the others summed up.
fn extensions(stats: &Stats, limit: usize) -> String {
    let mut by_size: Vec<(&String, u64)> = stats.extensions.iter().map(|(e, b)| (e, *b)).collect();
    by_size.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));

    let shown = limit.min(by_size.len());
    let mut items: Vec<String> = by_size[..shown]
        .iter()
        .map(|(extension, bytes)| {
            let label = if extension.is_empty() {
                "(none)".to_owned()
            } else {
                shorten(extension)
            };
            format!("{label} {}", share(*bytes, stats.bytes))
        })
        .collect();
    let others = &by_size[shown..];
    if !others.is_empty() {
        let bytes = others.iter().map(|(_, b)| b).sum();
        items.push(format!(
            "other {} ({} more)",
            share(bytes, stats.bytes),
            others.len()
        ));
    }
    items.join(", ")
}

/// A share in whole percent, rounded down. A share that rounds to zero
/// is shown as `<1%` so it does not look empty.
fn share(part: u64, whole: u64) -> String {
    let percent = (u128::from(part) * 100)
        .checked_div(u128::from(whole))
        .unwrap_or(0);
    if percent == 0 && part > 0 {
        "<1%".to_owned()
    } else {
        format!("{percent}%")
    }
}

fn shorten(name: &str) -> String {
    if name.chars().count() <= MAX_NAME_CHARS {
        name.to_owned()
    } else {
        let start: String = name.chars().take(MAX_NAME_CHARS - 3).collect();
        format!("{start}...")
    }
}

/// `YYYY-MM` for seconds since the Unix epoch, in UTC.
///
/// The civil-from-days algorithm by Howard Hinnant: it turns a day count
/// into a date with integer arithmetic only, so no date library is needed.
fn year_month(secs: u64) -> String {
    let z = secs / 86_400 + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    format!("{year:04}-{month:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn year_month_matches_known_dates() {
        assert_eq!(year_month(0), "1970-01");
        assert_eq!(year_month(951_782_400), "2000-02");
        assert_eq!(year_month(1_000_000_000), "2001-09");
        assert_eq!(year_month(1_767_225_599), "2025-12");
        assert_eq!(year_month(1_767_225_600), "2026-01");
    }

    #[test]
    fn shares_never_look_empty() {
        assert_eq!(share(1, 1_000), "<1%");
        assert_eq!(share(0, 1_000), "0%");
        assert_eq!(share(0, 0), "0%");
        assert_eq!(share(999, 1_000), "99%");
    }
}
