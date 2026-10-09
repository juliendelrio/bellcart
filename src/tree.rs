//! Read-only view of a directory tree.
//!
//! The engine only sees the disk through [`Tree`]. Production code uses
//! [`OsTree`]; tests use [`MemTree`], so scan and rule behaviour can be
//! specified with plain lists of paths on any OS.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// One entry of a directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    File {
        size: u64,
        /// Last modification, seconds since the Unix epoch.
        modified: Option<u64>,
    },
    Dir,
    /// Symbolic links, junctions and anything else we must not follow.
    Other,
}

pub trait Tree {
    /// Lists `dir`. Entries are returned sorted by name.
    fn read_dir(&self, dir: &Path) -> io::Result<Vec<Entry>>;
}

/// The real file system.
///
/// Links are reported as [`Kind::Other`] and never followed: legacy profile
/// junctions such as `Application Data` point back into the profile and
/// would loop. Sizes come from the directory listing itself, so files are
/// never opened.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsTree;

impl Tree for OsTree {
    fn read_dir(&self, dir: &Path) -> io::Result<Vec<Entry>> {
        let mut entries = Vec::new();
        for item in std::fs::read_dir(dir)? {
            let item = item?;
            let name = item.file_name().to_string_lossy().into_owned();
            let file_type = item.file_type()?;
            let kind = if file_type.is_symlink() {
                Kind::Other
            } else if file_type.is_dir() {
                Kind::Dir
            } else if file_type.is_file() {
                let meta = item.metadata()?;
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs());
                Kind::File {
                    size: meta.len(),
                    modified,
                }
            } else {
                Kind::Other
            };
            entries.push(Entry { name, kind });
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }
}

/// An in-memory tree for tests.
///
/// ```
/// use bellcart::tree::MemTree;
/// let tree = MemTree::new()
///     .file("Documents/taxes.pdf", 120_000)
///     .file("dev/app/node_modules/left-pad/index.js", 900);
/// ```
#[derive(Debug, Default, Clone)]
pub struct MemTree {
    dirs: BTreeMap<PathBuf, BTreeMap<String, Kind>>,
}

impl MemTree {
    pub fn new() -> Self {
        let mut dirs = BTreeMap::new();
        dirs.insert(PathBuf::new(), BTreeMap::new());
        Self { dirs }
    }

    /// Adds a file, creating parent directories. Paths use `/`.
    pub fn file(self, path: &str, size: u64) -> Self {
        self.file_at(path, size, None)
    }

    /// Adds a file with a modification time (seconds since the Unix epoch).
    pub fn file_at(mut self, path: &str, size: u64, modified: Option<u64>) -> Self {
        let (parent, name) = self.ensure_parent(path);
        self.dirs
            .entry(parent)
            .or_default()
            .insert(name, Kind::File { size, modified });
        self
    }

    /// Adds an empty directory.
    pub fn dir(mut self, path: &str) -> Self {
        let (parent, name) = self.ensure_parent(path);
        self.dirs.entry(parent.join(&name)).or_default();
        self.dirs.entry(parent).or_default().insert(name, Kind::Dir);
        self
    }

    /// Adds a link-like entry that the scan must not follow.
    pub fn link(mut self, path: &str) -> Self {
        let (parent, name) = self.ensure_parent(path);
        self.dirs
            .entry(parent)
            .or_default()
            .insert(name, Kind::Other);
        self
    }

    fn ensure_parent(&mut self, path: &str) -> (PathBuf, String) {
        let mut parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let name = parts.pop().unwrap_or_default().to_owned();
        let mut current = PathBuf::new();
        for part in parts {
            self.dirs
                .entry(current.clone())
                .or_default()
                .insert(part.to_owned(), Kind::Dir);
            current.push(part);
            self.dirs.entry(current.clone()).or_default();
        }
        (current, name)
    }
}

impl Tree for MemTree {
    fn read_dir(&self, dir: &Path) -> io::Result<Vec<Entry>> {
        let children = self
            .dirs
            .get(dir)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such directory"))?;
        Ok(children
            .iter()
            .map(|(name, kind)| Entry {
                name: name.clone(),
                kind: kind.clone(),
            })
            .collect())
    }
}
