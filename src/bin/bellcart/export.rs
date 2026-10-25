//! `bellcart export`: runs the export primitives of the applications found,
//! one at a time, each only after the user says yes.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use bellcart::export::{Export, Invocation, exports};
use bellcart::pack::Library;
use bellcart::recipe::{Recipes, Variables};
use bellcart::tree::OsTree;

use crate::i18n::Localizer;

pub fn run(profile: &Path, folder: &Path, library: &Library, text: &Localizer) -> Result<()> {
    crate::outside(profile, folder)?;
    let inside = crate::profile_in(profile);
    let found = Recipes::detect(
        &OsTree,
        profile,
        &library.recipes,
        &Variables::profile_at(&inside),
    );
    let wanted = exports(found.applications(), &library.recipes);
    if wanted.is_empty() {
        say(&format!("{}\n", text.get("export-none")))?;
        return Ok(());
    }
    std::fs::create_dir_all(folder)
        .with_context(|| format!("cannot create {}", folder.display()))?;
    say(&format!(
        "{}\n",
        text.with(
            "export-intro",
            &[("folder", folder.display().to_string().into())]
        )
    ))?;

    let mut answers = std::io::stdin().lock();
    let mut failed = 0;
    for export in wanted {
        let invocation = export.invocation(folder);
        say(&format!(
            "\n{}\n  {}\n{} ",
            label(&export, text),
            invocation.display(),
            text.get("export-ask")
        ))?;
        if !crate::ask::confirmed(&mut answers)? {
            say(&format!("{}\n", text.get("export-skipped")))?;
            continue;
        }
        match execute(&invocation, text) {
            Ok(()) => say(&format!(
                "{}\n",
                text.with(
                    "export-done",
                    &[("file", invocation.output.display().to_string().into())]
                )
            ))?,
            Err(error) => {
                failed += 1;
                say(&format!(
                    "{}\n",
                    text.with("export-failed", &[("error", format!("{error:#}").into())])
                ))?;
            }
        }
    }
    if failed > 0 {
        bail!("{failed} export(s) failed");
    }
    Ok(())
}

fn label(export: &Export, text: &Localizer) -> String {
    match export {
        Export::WingetPackages => text.get("export-winget"),
        Export::VscodeExtensions => text.get("export-vscode"),
        Export::RegistryKey(key) => text.with("export-registry", &[("key", key.as_str().into())]),
    }
}

/// Runs one command, without a shell.
fn execute(invocation: &Invocation, text: &Localizer) -> Result<()> {
    let mut command = Command::new(&invocation.program);
    command.args(&invocation.args);
    let missing = |error: std::io::Error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            anyhow!(text.with(
                "export-missing",
                &[("program", invocation.program.as_str().into())]
            ))
        } else {
            anyhow!(error)
        }
    };
    if invocation.capture {
        let output = command
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()
            .map_err(missing)?;
        if !output.status.success() {
            bail!("{} ended with {}", invocation.program, output.status);
        }
        std::fs::write(&invocation.output, output.stdout)
            .with_context(|| format!("cannot write {}", invocation.output.display()))?;
    } else {
        // The program may ask the user something itself (winget asks to
        // accept a source's terms): it gets the terminal.
        let status = command.status().map_err(missing)?;
        if !status.success() {
            bail!("{} ended with {status}", invocation.program);
        }
    }
    Ok(())
}

fn say(text: &str) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()?;
    Ok(())
}
