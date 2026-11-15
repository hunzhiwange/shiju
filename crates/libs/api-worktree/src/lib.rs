//! 工作树生命周期、保护与恢复契约。

pub mod worktree;
pub use worktree::*;

#[cfg(test)]
mod worktree_tests;
