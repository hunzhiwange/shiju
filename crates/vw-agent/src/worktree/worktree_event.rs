use crate::app::agent::bus;

/// Worktree 已就绪事件
///
/// 当 checkout、bootstrap 和项目初始化全部成功后触发
pub const READY: bus::Definition = bus::Definition { r#type: "worktree.ready" };

/// Worktree 失败事件
///
/// 当 worktree 创建或启动过程中发生错误时触发
pub const FAILED: bus::Definition = bus::Definition { r#type: "worktree.failed" };
