//! Bellcart engine.
//!
//! The command line lives in `src/bin/bellcart`; this library holds
//! everything that decides.
//!
//! The pipeline is: [`tree`] (what is on disk) -> [`scan`] (apply the rules
//! of a [`pack`]) -> [`plan`] (what goes where). Folders no rule settles are
//! left as `review` for an [`arbiter`] or for the user.
//!
//! This crate never touches the Windows API and never writes to the scanned
//! tree. Everything here must stay testable on any OS through
//! [`tree::MemTree`].

pub mod arbiter;
pub mod decision_log;
pub mod error;
pub mod export;
pub mod notebook;
pub mod openai_chat;
pub mod pack;
pub mod plan;
pub mod recipe;
pub mod scan;
pub mod schema;
pub mod signature;
pub mod tree;
pub mod verdict;

pub use error::{Error, Result};
