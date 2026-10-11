//! Runs a scan on its own thread pool and shows how far it is.

use std::io::{IsTerminal, Write};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use bellcart::pack::RuleSet;
use bellcart::plan::{SplitBudget, human_bytes};
use bellcart::scan::{Progress, Scan, scan_with};
use bellcart::tree::OsTree;

use crate::i18n::Localizer;

/// How often the progress line is redrawn.
const TICK: Duration = Duration::from_millis(100);
/// A scan shorter than this shows no progress line at all.
const QUIET_START: Duration = Duration::from_millis(300);
/// Threads of the scan pool get a larger stack than the default: the walk
/// recurses once per folder level, and some trees are very deep.
const STACK_SIZE: usize = 8 << 20;

/// Scans `path` with `jobs` threads (`None`: one per processor).
///
/// The progress line goes to stderr, and only when stderr is a terminal, so
/// that `--json` piped into another tool or a redirected output stay clean.
pub fn scan(
    path: &Path,
    rules: &RuleSet,
    budget: &SplitBudget,
    jobs: Option<usize>,
    text: &Localizer,
) -> Result<Scan> {
    let pool = rayon::ThreadPoolBuilder::new()
        // 0 lets rayon pick one thread per processor.
        .num_threads(jobs.unwrap_or(0))
        .stack_size(STACK_SIZE)
        .build()?;
    let progress = Progress::default();
    let show = std::io::stderr().is_terminal();
    let started = Instant::now();

    // A scoped thread may borrow `progress` and the rules: Rust checks that
    // it ends before they go away. The scan runs there while this thread
    // draws the line, because the localizer cannot be shared across threads.
    thread::scope(|scope| {
        let worker =
            scope.spawn(|| pool.install(|| scan_with(&OsTree, path, rules, budget, &progress)));
        let mut drawn = 0;
        while !worker.is_finished() {
            thread::sleep(TICK);
            if show && started.elapsed() >= QUIET_START {
                drawn = draw(&line(&progress, text), drawn);
            }
        }
        if drawn > 0 {
            draw("", drawn);
        }
        match worker.join() {
            Ok(result) => Ok(result?),
            Err(_) => Err(anyhow!("the scan stopped unexpectedly")),
        }
    })
}

fn line(progress: &Progress, text: &Localizer) -> String {
    let counts = progress.counts();
    text.with(
        "scan-progress",
        &[
            ("folders", counts.folders.into()),
            ("size", human_bytes(counts.bytes).into()),
        ],
    )
}

/// Redraws the line in place: back to its start, then spaces over what
/// the previous one left. Plain `\r`, which every Windows console knows,
/// rather than escape codes that older ones print as is. Returns how wide
/// the line is now.
fn draw(line: &str, previous: usize) -> usize {
    let width = line.chars().count();
    let padding = previous.saturating_sub(width);
    let mut stderr = std::io::stderr().lock();
    let _ = write!(stderr, "\r{line}{}\r", " ".repeat(padding));
    if width > 0 {
        let _ = write!(stderr, "{line}");
    }
    let _ = stderr.flush();
    width
}
