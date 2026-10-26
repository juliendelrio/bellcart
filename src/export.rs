//! The export primitives: the only steps that run a program.
//!
//! A pack cannot carry a command. It names one of these primitives, and the
//! engine builds the exact command line: a fixed program, fixed arguments,
//! and an output file in a folder the user chose. Nothing goes through a
//! shell. Front ends show each command and run it only once the user says
//! so; the engine itself never runs anything.

use std::path::{Path, PathBuf};

use crate::pack::{Recipe, Step};
use crate::plan::Application;

/// One export a recipe asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Export {
    /// `winget export`: the applications installed through winget.
    WingetPackages,
    /// `code --list-extensions`: the Visual Studio Code extensions.
    VscodeExtensions,
    /// `reg export`: one key of the user's registry.
    RegistryKey(String),
}

/// A command ready to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub program: String,
    pub args: Vec<String>,
    /// The file the export produces.
    pub output: PathBuf,
    /// The program prints the export: its standard output goes to `output`.
    /// Otherwise the program writes `output` itself.
    pub capture: bool,
}

impl Invocation {
    /// The command as a person would type it, to show before it runs.
    pub fn display(&self) -> String {
        let mut words = vec![self.program.clone()];
        words.extend(self.args.iter().map(|a| quote(a)));
        let mut line = words.join(" ");
        if self.capture {
            line.push_str(" > ");
            line.push_str(&quote(&self.output.to_string_lossy()));
        }
        line
    }
}

fn quote(word: &str) -> String {
    if word.contains(' ') {
        format!("\"{word}\"")
    } else {
        word.to_owned()
    }
}

impl Export {
    /// The file name the export is saved under.
    pub fn file_name(&self) -> String {
        match self {
            Export::WingetPackages => "winget-packages.json".to_owned(),
            Export::VscodeExtensions => "vscode-extensions.txt".to_owned(),
            Export::RegistryKey(key) => {
                let name: String = key
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                    .collect();
                format!("registry-{name}.reg")
            }
        }
    }

    /// The command that saves this export into `folder`.
    pub fn invocation(&self, folder: &Path) -> Invocation {
        let output = folder.join(self.file_name());
        let path = output.to_string_lossy().into_owned();
        match self {
            Export::WingetPackages => Invocation {
                program: "winget".to_owned(),
                args: vec!["export".to_owned(), "-o".to_owned(), path],
                output,
                capture: false,
            },
            Export::VscodeExtensions => Invocation {
                // `code` is a batch file on Windows. Rust runs it only when
                // named with its extension, and quotes arguments safely.
                program: if cfg!(windows) { "code.cmd" } else { "code" }.to_owned(),
                args: vec!["--list-extensions".to_owned()],
                output,
                capture: true,
            },
            Export::RegistryKey(key) => Invocation {
                program: "reg".to_owned(),
                args: vec!["export".to_owned(), key.clone(), path, "/y".to_owned()],
                output,
                capture: false,
            },
        }
    }
}

/// The exports asked for by the recipes of the applications found, each
/// once, in the order of the applications.
pub fn exports(applications: &[Application], recipes: &[Recipe]) -> Vec<Export> {
    let mut out = Vec::new();
    for application in applications {
        let Some(recipe) = recipes.iter().find(|r| r.id == application.recipe) else {
            continue;
        };
        for step in recipe.before.iter().chain(&recipe.after) {
            let export = match step {
                Step::ExportWingetPackages => Export::WingetPackages,
                Step::ExportVscodeExtensions => Export::VscodeExtensions,
                Step::ExportRegistryKey { key } => Export::RegistryKey(key.clone()),
                _ => continue,
            };
            if !out.contains(&export) {
                out.push(export);
            }
        }
    }
    out
}

/// Checks a registry key when a pack loads.
///
/// Only keys of the current user (`HKCU\...`): an application's settings
/// live there, and nothing else should leave the machine in a notebook.
/// Names keep to letters, digits, spaces and `_ - . \`, so a key can never
/// read as a second argument or an option.
pub fn check_registry_key(key: &str) -> Result<(), String> {
    let rest = ["HKCU\\", "HKEY_CURRENT_USER\\"]
        .iter()
        .find_map(|root| strip_prefix_ignore_case(key, root));
    let Some(rest) = rest else {
        return Err(format!(
            "registry key `{key}` must start with HKCU\\ (the user's own settings)"
        ));
    };
    // A comma for publishers such as "Plex, Inc.": the key is passed to
    // reg as one argument and quoted in the notebook.
    let allowed = |c: char| c.is_ascii_alphanumeric() || " _-.,\\".contains(c);
    let parts_ok = rest
        .split('\\')
        .all(|part| !part.trim().is_empty() && part != "." && part != "..");
    if rest.chars().all(allowed) && parts_ok {
        Ok(())
    } else {
        Err(format!(
            "registry key `{key}` may only hold letters, digits, spaces and _ - . , \\"
        ))
    }
}

fn strip_prefix_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_of_the_current_user_are_accepted() {
        assert!(check_registry_key(r"HKCU\Software\SimonTatham\PuTTY").is_ok());
        assert!(check_registry_key(r"hkcu\Software\My App 2.0").is_ok());
        assert!(check_registry_key(r"HKEY_CURRENT_USER\Software\x-y_z").is_ok());
    }

    #[test]
    fn other_keys_and_odd_names_are_refused() {
        for key in [
            r"HKLM\SOFTWARE\Microsoft",
            r"HKCU",
            r"HKCU\",
            r"HKCU\Software\\Empty",
            r#"HKCU\Software" /other"#,
            r"HKCU\Software\..\x",
            r"HKCU\Software\a&b",
            "HKCU\\Software\\new\nline",
        ] {
            assert!(check_registry_key(key).is_err(), "{key:?} must be refused");
        }
    }

    #[test]
    fn registry_files_get_a_plain_name() {
        assert_eq!(
            Export::RegistryKey(r"HKCU\Software\My App".to_owned()).file_name(),
            "registry-HKCU_Software_My_App.reg"
        );
    }

    #[test]
    fn commands_are_fixed_and_write_into_the_chosen_folder() {
        let folder = Path::new("out");
        let winget = Export::WingetPackages.invocation(folder);
        assert_eq!(winget.program, "winget");
        assert_eq!(winget.args[..2], ["export", "-o"]);
        assert_eq!(winget.output, folder.join("winget-packages.json"));
        assert!(!winget.capture);

        let code = Export::VscodeExtensions.invocation(folder);
        assert!(code.program.starts_with("code"));
        assert_eq!(code.args, ["--list-extensions"]);
        assert!(code.capture);

        let reg = Export::RegistryKey(r"HKCU\Software\X".to_owned()).invocation(folder);
        assert_eq!(reg.program, "reg");
        assert_eq!(reg.args[0], "export");
        assert_eq!(reg.args[1], r"HKCU\Software\X");
        assert_eq!(reg.args[3], "/y");
    }

    #[test]
    fn the_displayed_command_quotes_paths_with_spaces() {
        let code = Export::VscodeExtensions.invocation(Path::new("my exports"));
        assert!(
            code.display()
                .ends_with("--list-extensions > \"my exports/vscode-extensions.txt\"")
                || code
                    .display()
                    .ends_with("--list-extensions > \"my exports\\vscode-extensions.txt\"")
        );
    }
}
