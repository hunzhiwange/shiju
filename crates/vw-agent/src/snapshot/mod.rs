//! 快照模块 - 基于 Git 的文件系统快照管理
//!
//! 本模块提供了一套完整的文件系统快照管理机制，使用独立的 Git 仓库来跟踪、
//! 存储和恢复工作目录的文件状态。主要功能包括：
//!
//! - 跟踪快照：将当前工作目录的文件状态记录到 Git 树对象
//! - 恢复快照：将工作目录恢复到之前保存的快照状态
//! - 差异计算：获取文件变更的详细差异信息
//! - 补丁管理：生成和应用文件补丁
//! - 自动清理：定期清理过期的快照数据
//!
//! # 架构设计
//!
//! - 每个项目使用独立的 Git 目录（位于 data/snapshot/<project_id>）
//! - 项目 ID 基于项目 Git 仓库的根提交哈希生成，确保唯一性
//! - 支持跨平台运行，在 WASM 目标上提供空实现
//!
//! # 示例
//!
//! ```rust,no_run
//! use std::path::Path;
//!
//! // 跟踪当前文件状态
//! let hash = snapshot::track("/path/to/worktree")?;
//!
//! // 获取差异
//! if let Some(hash) = &hash {
//!     let diff = snapshot::diff("/path/to/worktree", hash)?;
//!     println!("Changes: {}", diff);
//! }
//!
//! // 恢复到快照
//! if let Some(hash) = &hash {
//!     snapshot::restore("/path/to/worktree", hash)?;
//! }
//! ```

mod common;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use common::{Error, Patch};
pub use snapshot_types::{DiffStatus, FileDiff};

#[cfg(not(target_arch = "wasm32"))]
pub use native::{cleanup, diff, diff_full, init, patch, restore, revert, track};
#[cfg(target_arch = "wasm32")]
pub use wasm::{cleanup, diff, diff_full, init, patch, restore, revert, track};
