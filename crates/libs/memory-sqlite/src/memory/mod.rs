//! SQLite 后端与其使用的记忆契约。

#[cfg(not(target_arch = "wasm32"))]
pub mod sqlite;

pub use memory_types::memory::{MemoryCategory, embeddings, traits};
