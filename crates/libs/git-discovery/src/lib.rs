//! 基于 git2 的只读仓库发现。

#[cfg(not(target_arch = "wasm32"))]
pub mod git_repository;
