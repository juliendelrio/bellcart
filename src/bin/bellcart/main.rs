//! The `bellcart` command line.

mod export;
mod i18n;
mod notebook;
mod progress;
mod render;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use bellcart::pack::{Pack, RuleSet, combine};
use bellcart::plan::SplitBudget;
use bellcart::signature::{self, Privacy};
use clap::{Parser, Subcommand};
use include_dir::{Dir, include_dir};

/// The pack shipped inside the binary, so one file is all a user needs.
static CORE_PACK: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/packs/core");

#[derive(Parser)]
#[command(name = "bellcart", version, about)]
struct Cli {
    /// Interface language (`en`, `fr`). Defaults to the system language.
    #[arg(long, global = true, env = "BELLCART_LANG")]
    lang: Option<String>,

    /// Also load the pack in this folder, after the built-in one. For pack
    /// authors; may be given several times.
    #[arg(long = "pack", global = true, value_name = "FOLDER")]
    packs: Vec<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan a folder and print its migration plan. Nothing is modified.
    Scan {
        /// The folder to scan, typically a user profile.
        path: PathBuf,
        /// Print the plan as JSON instead of a summary.
        #[arg(long)]
        json: bool,
        /// Also write the JSON plan to this file.
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
        /// Write the migration notebook to this file: HTML if it ends in
        /// `.html`, Markdown otherwise.
        #[arg(long, value_name = "FILE")]
        notebook: Option<PathBuf>,
        /// Print the signature of each folder to review instead of a
        /// summary: what an arbiter would be sent.
        #[arg(long, conflicts_with = "json")]
        signatures: bool,
        /// With `--signatures`: `redacted` shows what a cloud arbiter would
        /// be sent, names under personal folders masked.
        #[arg(long, value_enum, default_value_t = PrivacyArg::Full, requires = "signatures")]
        privacy: PrivacyArg,
        /// How many folders to list at once. Defaults to one per
        /// processor; `1` scans one folder at a time, which can be faster
        /// on a spinning hard disk.
        #[arg(long, value_name = "N", value_parser = clap::value_parser!(u16).range(1..))]
        jobs: Option<u16>,
    },
    /// Run the exports the recipes ask for (winget list, VS Code extensions,
    /// registry keys), each shown and run only once you say yes.
    Export {
        /// The profile whose applications to look for.
        path: PathBuf,
        /// The folder to save the exports in, outside the profile.
        #[arg(long, value_name = "FOLDER")]
        to: PathBuf,
    },
    /// Show what the built-in pack contains.
    Packs,
}

/// How much of a signature leaves the machine.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum PrivacyArg {
    /// The signature as built, for a local or trusted endpoint.
    Full,
    /// Names under personal folders masked, for a cloud endpoint.
    Redacted,
}

impl From<PrivacyArg> for Privacy {
    fn from(arg: PrivacyArg) -> Self {
        match arg {
            PrivacyArg::Full => Privacy::Full,
            PrivacyArg::Redacted => Privacy::Redacted,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // `{:#}` prints the whole cause chain on one line.
            eprintln!("bellcart: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let text = i18n::Localizer::new(cli.lang.as_deref());
    let mut packs = vec![core_pack()?];
    for folder in &cli.packs {
        packs.push(
            Pack::from_dir(folder)
                .with_context(|| format!("cannot load the pack in {}", folder.display()))?,
        );
    }
    let library = combine(&packs)?;

    match cli.command {
        Command::Scan {
            path,
            json,
            out,
            notebook,
            signatures,
            privacy,
            jobs,
        } => {
            // Checked before the scan, so a mistake costs no waiting.
            for file in out.iter().chain(&notebook) {
                outside(&path, file)?;
            }
            let rules = RuleSet::compile(&library.rules)?;
            let scan = progress::scan(
                &path,
                &rules,
                &library.recipes,
                &SplitBudget::default(),
                jobs.map(usize::from),
                &text,
            )?;
            let document = serde_json::to_string_pretty(&scan.plan)?;
            if let Some(out) = out {
                std::fs::write(&out, &document)
                    .with_context(|| format!("cannot write {}", out.display()))?;
            }
            if let Some(file) = notebook {
                let book = bellcart::notebook::notebook(&scan.plan.applications, &library.recipes);
                let content = notebook::render(
                    &book,
                    &scan.plan.root,
                    &text,
                    notebook::Format::for_file(&file),
                );
                std::fs::write(&file, content)
                    .with_context(|| format!("cannot write {}", file.display()))?;
            }
            if json {
                write_out(&format!("{document}\n"))?;
            } else if signatures {
                let texts: Vec<String> = signature::signatures(&scan)
                    .iter()
                    .map(|s| s.for_privacy(privacy.into()).to_text())
                    .collect();
                write_out(&texts.join("\n"))?;
            } else {
                write_out(&render::plan(&scan.plan, &library.recipes, &text))?;
            }
        }
        Command::Export { path, to } => export::run(&path, &to, &library, &text)?,
        Command::Packs => {
            let listing: Vec<String> = packs.iter().map(|p| render::pack(p, &text)).collect();
            write_out(&listing.join(
                "
",
            ))?;
        }
    }
    Ok(())
}

/// Refuses a file Bellcart would write inside the tree it scans: the scan
/// is read-only, its outputs included.
pub fn outside(given: &Path, file: &Path) -> Result<()> {
    // Canonical forms resolve `..`, links and case. They are only compared,
    // never shown: on Windows they read `\\?\C:\...`.
    let root =
        std::fs::canonicalize(given).with_context(|| format!("cannot read {}", given.display()))?;
    // The file, or the folders above it, may not exist yet: compare the
    // closest one that does.
    let mut folder = file;
    while !folder.exists() {
        folder = match folder.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
    }
    let folder = std::fs::canonicalize(folder)
        .with_context(|| format!("cannot write {}", file.display()))?;
    if folder.starts_with(&root) {
        anyhow::bail!(
            "cannot write {} inside the scanned folder {}: Bellcart never writes there. \
             Choose another folder.",
            file.display(),
            given.display()
        );
    }
    Ok(())
}

/// Writes to stdout. `print!` panics when stdout is a pipe that was closed,
/// as with `bellcart scan . | more` left early: here that is not an error,
/// the reader simply wanted no more.
fn write_out(text: &str) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    match stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Err(error) if error.kind() != std::io::ErrorKind::BrokenPipe => {
            Err(error).context("cannot write the output")
        }
        _ => Ok(()),
    }
}

fn core_pack() -> Result<Pack> {
    let mut files = Vec::new();
    collect(&CORE_PACK, &mut files);
    Pack::from_files(files).context("the built-in pack is invalid")
}

fn collect(dir: &Dir<'_>, out: &mut Vec<(String, String)>) {
    for file in dir.files() {
        if let Some(content) = file.contents_utf8() {
            out.push((
                file.path().to_string_lossy().into_owned(),
                content.to_owned(),
            ));
        }
    }
    for sub in dir.dirs() {
        collect(sub, out);
    }
}
