//! The migration notebook as Markdown or HTML, in the interface language.

use std::fmt::Write as _;

use bellcart::notebook::{Notebook, Task};
use fluent_bundle::FluentValue;

use crate::i18n::Localizer;

/// Stands for a path or a command in a translated sentence, so that the
/// format can mark it as code after the sentence is escaped.
const SLOT: &str = "\u{1}";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Markdown,
    Html,
}

impl Format {
    /// HTML for `.html` and `.htm` files, Markdown otherwise.
    pub fn for_file(path: &std::path::Path) -> Self {
        let extension = path.extension().map(|e| e.to_string_lossy().to_lowercase());
        match extension.as_deref() {
            Some("html" | "htm") => Format::Html,
            _ => Format::Markdown,
        }
    }
}

/// One line of a checklist: a sentence, and the code it holds in [`SLOT`].
struct Line {
    sentence: String,
    code: Option<String>,
}

pub fn render(notebook: &Notebook, root: &str, text: &Localizer, format: Format) -> String {
    let title = text.with("notebook-title", &[("root", root.into())]);
    // Each section holds blocks: one per application, then the programs.
    let mut sections: Vec<(String, Vec<Block>)> = Vec::new();
    for (key, chapters, programs, block_key, line_key) in [
        (
            "notebook-before",
            &notebook.before,
            &notebook.undecided,
            "notebook-programs-undecided",
            "notebook-program-decide",
        ),
        (
            "notebook-after",
            &notebook.after,
            &notebook.reinstall,
            "notebook-programs-reinstall",
            "notebook-program-install",
        ),
    ] {
        let mut blocks: Vec<Block> = chapters
            .iter()
            .map(|chapter| Block {
                title: chapter.name.get(text.lang()).to_owned(),
                lines: chapter.tasks.iter().map(|task| line(task, text)).collect(),
            })
            .collect();
        if !programs.is_empty() {
            blocks.push(Block {
                title: text.get(block_key),
                lines: programs
                    .iter()
                    .map(|name| Line {
                        sentence: text.with(line_key, &[("name", name.as_str().into())]),
                        code: None,
                    })
                    .collect(),
            });
        }
        if !blocks.is_empty() {
            sections.push((text.get(key), blocks));
        }
    }

    match format {
        Format::Markdown => {
            let mut out = format!(
                "# {}

",
                markdown(&title)
            );
            if notebook.is_empty() {
                let _ = writeln!(out, "{}", markdown(&text.get("notebook-empty")));
                return out;
            }
            let _ = writeln!(out, "{}", markdown(&text.get("notebook-intro")));
            for (heading, blocks) in sections {
                let _ = write!(
                    out,
                    "
## {}
",
                    markdown(&heading)
                );
                for block in blocks {
                    let _ = write!(
                        out,
                        "
### {}

",
                        markdown(&block.title)
                    );
                    for line in block.lines {
                        let code = line.code.map(|c| format!("`{c}`")).unwrap_or_default();
                        let _ = writeln!(
                            out,
                            "- [ ] {}",
                            markdown(&line.sentence).replace(SLOT, &code)
                        );
                    }
                }
            }
            out
        }
        Format::Html => {
            let mut out = String::new();
            let _ = write!(
                out,
                "<!doctype html>
<html lang=\"{}\">
<head>
<meta charset=\"utf-8\">
                 <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
                 <title>{}</title>
<style>{STYLE}</style>
</head>
<body>
<h1>{}</h1>
",
                text.lang(),
                html(&title),
                html(&title)
            );
            if notebook.is_empty() {
                let _ = writeln!(out, "<p>{}</p>", html(&text.get("notebook-empty")));
            } else {
                let _ = writeln!(out, "<p>{}</p>", html(&text.get("notebook-intro")));
            }
            for (heading, blocks) in sections {
                let _ = writeln!(out, "<h2>{}</h2>", html(&heading));
                for block in blocks {
                    let _ = writeln!(
                        out,
                        "<h3>{}</h3>
<ul>",
                        html(&block.title)
                    );
                    for line in block.lines {
                        let code = line
                            .code
                            .map(|c| format!("<code>{}</code>", html(&c)))
                            .unwrap_or_default();
                        let _ = writeln!(
                            out,
                            "<li><label><input type=\"checkbox\"> {}</label></li>",
                            html(&line.sentence).replace(SLOT, &code)
                        );
                    }
                    let _ = writeln!(out, "</ul>");
                }
            }
            out.push_str(
                "</body>
</html>
",
            );
            out
        }
    }
}

/// A titled checklist: an application's tasks, or the programs.
struct Block {
    title: String,
    lines: Vec<Line>,
}

const STYLE: &str = "body{font-family:system-ui,sans-serif;max-width:46rem;margin:2rem auto;\
padding:0 1rem;line-height:1.5;color:#1d1d1f;background:#fff}\
h2{margin-top:2rem;border-bottom:1px solid #ccc}ul{list-style:none;padding-left:0}\
li{margin:.4rem 0}code{background:#f2f2f2;padding:0 .2rem;border-radius:3px}\
@media (prefers-color-scheme:dark){body{color:#eee;background:#1d1d1f}code{background:#333}}\
@media print{input{transform:scale(1.2)}}";

fn line(task: &Task, text: &Localizer) -> Line {
    let lang = text.lang();
    let with_code = |key: &str, code: String, args: &[(&str, FluentValue<'_>)]| {
        let mut all = vec![("command", SLOT.into())];
        all.extend(args.iter().cloned());
        Line {
            sentence: text.with(key, &all),
            code: Some(code),
        }
    };
    match task {
        Task::Copy { path, note } => {
            let sentence = match note {
                Some(note) => text.with(
                    "notebook-copy-note",
                    &[("path", SLOT.into()), ("note", note.get(lang).into())],
                ),
                None => text.with("notebook-copy", &[("path", SLOT.into())]),
            };
            Line {
                sentence,
                code: Some(path.clone()),
            }
        }
        Task::Manual(manual) => Line {
            sentence: manual.get(lang).to_owned(),
            code: None,
        },
        Task::ExportWingetPackages => with_code(
            "notebook-export-winget",
            "winget export -o winget-packages.json".to_owned(),
            &[],
        ),
        Task::ExportVscodeExtensions => with_code(
            "notebook-export-vscode",
            "code --list-extensions > vscode-extensions.txt".to_owned(),
            &[],
        ),
        Task::ExportRegistryKey { key } => with_code(
            "notebook-export-registry",
            format!("reg export \"{key}\" registry.reg"),
            &[("key", key.as_str().into())],
        ),
        Task::Install { winget } => with_code(
            "notebook-install",
            format!("winget install --id {winget} -e"),
            &[],
        ),
    }
}

/// Escapes text for HTML.
fn html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// Keeps a backslash visible in Markdown: before punctuation, Markdown
/// would read `\.` as an escaped dot and drop the backslash, so a Windows
/// path like `%USERPROFILE%\.ssh` would lose it.
fn markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c == '\\' && chars.peek().is_some_and(|n| n.is_ascii_punctuation()) {
            out.push('\\');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_is_escaped() {
        assert_eq!(
            html(r#"<b>"Tom" & 'Jerry'</b>"#),
            "&lt;b&gt;&quot;Tom&quot; &amp; &#39;Jerry&#39;&lt;/b&gt;"
        );
    }

    #[test]
    fn markdown_keeps_backslashes_before_punctuation() {
        assert_eq!(markdown(r"%USERPROFILE%\.ssh"), r"%USERPROFILE%\\.ssh");
        assert_eq!(markdown(r"%APPDATA%\Mozilla"), r"%APPDATA%\Mozilla");
    }

    #[test]
    fn the_format_follows_the_file_extension() {
        use std::path::Path;
        assert_eq!(Format::for_file(Path::new("a.HTML")), Format::Html);
        assert_eq!(Format::for_file(Path::new("a.htm")), Format::Html);
        assert_eq!(Format::for_file(Path::new("a.md")), Format::Markdown);
        assert_eq!(Format::for_file(Path::new("notes")), Format::Markdown);
    }
}
