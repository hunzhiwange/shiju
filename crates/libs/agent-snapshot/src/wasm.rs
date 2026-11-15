//! 快照模块的 WASM 平台桩实现。
//!
//! WASM 目标当前不支持本地 Git 快照，因此对外接口保持一致，
//! 但返回空结果或 no-op，以保证调用方在跨平台编译时无需额外分支。

use super::{Error, FileDiff, Patch};
use std::path::Path;

/// 初始化快照清理任务。
pub fn init(_worktree: impl AsRef<Path>) {}

/// 清理过期的快照数据。
pub fn cleanup(_worktree: impl AsRef<Path>) -> Result<(), Error> {
    Ok(())
}

/// 跟踪当前工作目录状态。
pub fn track(_worktree: impl AsRef<Path>) -> Result<Option<String>, Error> {
    Ok(None)
}

/// 获取快照与当前状态之间的文件补丁信息。
pub fn patch(_worktree: impl AsRef<Path>, hash: &str) -> Result<Patch, Error> {
    Ok(Patch { hash: hash.to_string(), files: Vec::new() })
}

/// 恢复工作目录到指定快照状态。
pub fn restore(_worktree: impl AsRef<Path>, _snapshot: &str) -> Result<(), Error> {
    Ok(())
}

/// 回退指定的文件补丁。
pub fn revert(_worktree: impl AsRef<Path>, _patches: &[Patch]) -> Result<(), Error> {
    Ok(())
}

/// 获取快照与当前状态之间的差异。
pub fn diff(_worktree: impl AsRef<Path>, _hash: &str) -> Result<String, Error> {
    Ok(String::new())
}

/// 获取两个快照之间的完整差异信息。
pub fn diff_full(
    _worktree: impl AsRef<Path>,
    _from: &str,
    _to: &str,
) -> Result<Vec<FileDiff>, Error> {
    Ok(Vec::new())
}
#[cfg(test)]
#[path = "wasm_tests.rs"]
mod wasm_tests;
