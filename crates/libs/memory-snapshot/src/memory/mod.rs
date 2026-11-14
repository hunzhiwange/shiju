//! 记忆快照及其使用的存储路径。

#[cfg(not(target_arch = "wasm32"))]
pub mod snapshot;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use project_storage_paths::paths;
