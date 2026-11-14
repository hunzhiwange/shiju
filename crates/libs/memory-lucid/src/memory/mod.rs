//! Lucid 后端与其使用的记忆契约。

#[cfg(not(target_arch = "wasm32"))]
pub mod lucid;

pub use memory_types::memory::traits;
