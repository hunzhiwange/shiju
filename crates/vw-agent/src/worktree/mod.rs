//! Git Worktree 管理模块
//!
//! 本模块提供 Git worktree（工作树）的创建、删除、重置和查询功能。
//! Worktree 允许在同一仓库下同时检出多个分支，实现并行开发而无需频繁切换分支。
//!
//! # 主要功能
//!
//! - **创建 worktree**：基于现有项目创建独立的开发环境
//! - **删除 worktree**：清理不再需要的开发环境
//! - **重置 worktree**：将 worktree 重置到指定的基准引用
//! - **列出目录**：查询所有 worktree 的目录路径
//!
//! # 架构设计
//!
//! 本模块采用 trait 驱动的设计模式，与项目实例（instance）、存储（storage）、
//! 事件总线（bus）等模块深度集成，提供完整的 worktree 生命周期管理。
//!
//! # 使用场景
//!
//! - 并行特性开发：在不同 worktree 中同时开发多个独立特性
//! - 隔离测试环境：创建干净的 worktree 进行测试
//! - 快速上下文切换：在不同 worktree 间快速切换，无需 stash 或 commit

/// 声明并公开 worktree 子模块
///
/// worktree 子模块包含所有 worktree 操作的具体实现，
/// 包括错误处理、数据结构、异步操作等。
pub mod worktree;

/// 重新导出 worktree 子模块的所有公开项
///
/// 通过此重导出，外部代码可以直接使用 `worktree::Info`、
/// `worktree::create` 等，而无需显式引用 `worktree::worktree::*`。
pub use worktree::*;

#[cfg(test)]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
mod host_adapter;
