use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot read {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid pack file {file}: {message}")]
    PackFile { file: String, message: String },

    #[error("invalid rule `{rule}`: {message}")]
    Rule { rule: String, message: String },

    #[error("arbiter: {0}")]
    Arbiter(String),
}
