//! The `bellcart` command line.

mod i18n;
mod render;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use bellcart::pack::{Pack, RuleSet};
use bellcart::plan::SplitBudget;
use bellcart::scan::scan;
use bellcart::tree::OsTree;
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
    },
    /// Show what the built-in pack contains.
    Packs,
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
    let pack = core_pack()?;

    match cli.command {
        Command::Scan { path, json, out } => {
            let rules = RuleSet::compile(&pack.rules)?;
            let plan = scan(&OsTree, &path, &rules, &SplitBudget::default())?;
            let document = serde_json::to_string_pretty(&plan)?;
            if let Some(out) = out {
                std::fs::write(&out, &document)
                    .with_context(|| format!("cannot write {}", out.display()))?;
            }
            if json {
                println!("{document}");
            } else {
                print!("{}", render::plan(&plan, &text));
            }
        }
        Command::Packs => print!("{}", render::pack(&pack, &text)),
    }
    Ok(())
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
