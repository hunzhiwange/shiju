//! Git 状态、提交与远端同步契约。

pub mod git;
pub use git::*;

#[cfg(test)]
mod git_tests;
