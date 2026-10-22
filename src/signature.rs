//! Signatures: what an arbiter, or a person, reads about one review unit.
//!
//! The text form is the arbiter's `state`. It is fixed English and never
//! translated: it is input for a model, and the same unit must give the
//! same signature whatever the interface language. It holds names, sizes,
//! dates and extensions, never file contents.
//!
//! [`Signature`] keeps the data apart from the text, so that a redacted
//! form can mask names later without changing what is collected.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::plan::{Decision, PlanNode, Stats, UnitReason, extension_of, human_bytes};
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

    /// Takes in the sample of a sub-folder's rest that joins this one, its
    /// names prefixed with that sub-folder so they stay unambiguous.
    pub fn absorb(&mut self, other: Sample, folder: &str) {
        let prefixed = |entries: Vec<Named>| {
            entries.into_iter().map(|n| Named {
                name: format!("{folder}/{}", n.name),
                bytes: n.bytes,
            })
        };
        self.folder_count += other.folder_count;
        self.file_count += other.file_count;
        let mut folders = std::mem::take(&mut self.folders);
        folders.extend(prefixed(other.folders));
        self.folders = largest(folders);
        let mut files = std::mem::take(&mut self.files);
        files.extend(prefixed(other.files));
        self.files = largest(files);
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

/// How much of a signature may leave the machine (SPEC-V1 section 9).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Privacy {
    /// Names under personal folders are masked. Meant for cloud endpoints.
    #[default]
    Redacted,
    /// The signature as built. Meant for a local or trusted endpoint.
    Full,
}

/// Folders every Windows profile has. Their names say nothing about the
/// person, so a redacted signature keeps them as the first part of a path.
const KNOWN_FOLDERS: &[&str] = &[
    "3D Objects",
    "Contacts",
    "Desktop",
    "Documents",
    "Downloads",
    "Favorites",
    "Links",
    "Music",
    "OneDrive",
    "Pictures",
    "Saved Games",
    "Searches",
    "Videos",
];

fn is_known_folder(name: &str) -> bool {
    KNOWN_FOLDERS.iter().any(|k| k.eq_ignore_ascii_case(name))
}

/// Whether a path belongs to applications rather than to the person:
/// `AppData` and the dot folders at the profile root (`.cache`, `.vscode`).
fn is_application_path(path: &str) -> bool {
    let first = path.split('/').next().unwrap_or_default();
    first.eq_ignore_ascii_case("AppData") || first.starts_with('.')
}

/// Placeholders for masked names: the same name always gets the same one
/// within a signature, in order of appearance.
#[derive(Default)]
struct Masks {
    given: BTreeMap<String, usize>,
}

impl Masks {
    fn number(&mut self, name: &str) -> usize {
        let next = self.given.len() + 1;
        *self.given.entry(name.to_owned()).or_insert(next)
    }

    fn folder(&mut self, name: &str) -> String {
        format!("<{}>", self.number(name))
    }

    /// A masked file keeps its extension, which is not personal and says
    /// what kind of file it is.
    fn file(&mut self, name: &str) -> String {
        let number = self.number(name);
        match extension_of(name) {
            ext if ext.is_empty() => format!("<{number}>"),
            ext => format!("<{number}>.{ext}"),
        }
    }

    /// Masks every component of a relative path; the last is a file when
    /// `file` is set.
    fn path(&mut self, path: &str, file: bool) -> String {
        let parts: Vec<&str> = path.split('/').collect();
        let last = parts.len().saturating_sub(1);
        parts
            .iter()
            .enumerate()
            .map(|(i, part)| {
                if file && i == last {
                    self.file(part)
                } else {
                    self.folder(part)
                }
            })
            .collect::<Vec<_>>()
            .join("/")
    }
}

impl Signature {
    /// This signature as `privacy` allows it to be sent.
    pub fn for_privacy(&self, privacy: Privacy) -> Signature {
        match privacy {
            Privacy::Full => self.clone(),
            Privacy::Redacted => self.redacted(),
        }
    }

    /// Masks the names of a unit under personal folders. Sizes, counts,
    /// dates and extensions stay, and so do application paths.
    pub fn redacted(&self) -> Signature {
        if is_application_path(&self.path) {
            return self.clone();
        }
        let mut masks = Masks::default();
        let path = if self.path.is_empty() {
            String::new()
        } else {
            let (first, rest) = match self.path.split_once('/') {
                Some((first, rest)) => (first, Some(rest)),
                None => (self.path.as_str(), None),
            };
            let head = if is_known_folder(first) {
                first.to_owned()
            } else {
                masks.folder(first)
            };
            match rest {
                Some(rest) => format!("{head}/{}", masks.path(rest, false)),
                None => head,
            }
        };
        let at_root = self.path.is_empty();
        let rename = |entries: &[Named], masks: &mut Masks, file: bool| -> Vec<Named> {
            entries
                .iter()
                .map(|n| {
                    // At the profile root, its standard folders keep their
                    // names: `Documents` or `AppData` says nothing personal.
                    let generic = at_root
                        && !file
                        && (is_known_folder(&n.name) || is_application_path(&n.name));
                    Named {
                        name: if generic {
                            n.name.clone()
                        } else {
                            masks.path(&n.name, file)
                        },
                        bytes: n.bytes,
                    }
                })
                .collect()
        };
        let folders = rename(&self.sample.folders, &mut masks, false);
        let files = rename(&self.sample.files, &mut masks, true);
        // What a rule settled is named by the rule (`node_modules`,
        // `.venv`): that last name stays, the folders above it do not.
        let settled = self
            .settled
            .iter()
            .map(|(rel, decision)| {
                let masked = match rel.rsplit_once('/') {
                    Some((above, last)) => format!("{}/{last}", masks.path(above, false)),
                    None => rel.clone(),
                };
                (masked, *decision)
            })
            .collect();
        Signature {
            path,
            unit: self.unit.clone(),
            stats: self.stats.clone(),
            sample: Sample {
                folders,
                folder_count: self.sample.folder_count,
                files,
                file_count: self.sample.file_count,
            },
            settled,
        }
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
        // The key is pack data, never a name from the disk.
        UnitReason::Rule { key, .. } => format!("recognised by a rule: {key}"),
        UnitReason::Part { key, .. } => format!("one folder judged whole: {key}"),
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
