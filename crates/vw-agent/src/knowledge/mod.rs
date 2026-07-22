//! Local knowledge base runtime.
//!
//! The first implementation keeps the operational surface narrow: SQLite stores
//! datasets, documents and chunks, while SQLite FTS5 provides deterministic
//! full-text retrieval for gateway and workflow callers.

mod chunker;
mod fusion;
pub(crate) mod import;
mod store;

pub use store::SqliteKnowledgeStore;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "import_tests.rs"]
mod import_tests;
