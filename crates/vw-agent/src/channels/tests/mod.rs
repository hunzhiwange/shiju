//! # Channels 模块测试套件
//!
//! 本模块提供了 `channels` 模块的全面测试覆盖，用于验证各通道集成和调度的正确性。
//!
//! ## 模块职责
//!
//! - 测试消息分发和路由逻辑
//! - 验证命令解析和处理
//! - 测试健康检查和超时机制
//! - 验证配置加载和历史记录管理
//! - 测试工具执行和视觉功能
//!
//! ## 子模块结构
//!
//! - `approval_commands`: 审批命令测试
//! - `command_parsing`: 命令解析测试
//! - `config_loading`: 配置加载测试
//! - `health_supervisor`: 健康监督器测试
//! - `history_management`: 历史记录管理测试
//! - `identity`: 身份验证测试
//! - `message_dispatch`: 消息分发测试
//! - `routing`: 路由逻辑测试
//! - `system_prompt`: 系统提示词测试
//! - `timeout_budget`: 超时预算测试
//! - `tool_execution`: 工具执行测试
//! - `util`: 测试工具函数
//! - `vision`: 视觉功能测试

use super::*;
use crate::app::agent::memory::{Memory, MemoryCategory, SqliteMemory};
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::{ChatMessage, Provider};
use crate::app::agent::tools::{Tool, ToolResult};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tempfile::TempDir;

/// 创建临时工作区用于测试
///
/// 该函数创建一个包含标准代理配置文件的临时目录，用于隔离测试环境。
/// 每个测试运行都会获得独立的临时目录，避免测试间相互干扰。
///
/// # 返回值
///
/// 返回一个 `TempDir` 实例，包含以下预配置文件：
/// - `SOUL.md`: 代理核心行为准则
/// - `IDENTITY.md`: 代理身份定义
/// - `USER.md`: 用户配置
/// - `AGENTS.md`: 代理指令
/// - `TOOLS.md`: 工具使用指南
/// - `HEARTBEAT.md`: 心跳检查配置
/// - `MEMORY.md`: 记忆存储配置
///
/// # 示例
///
/// ```ignore
/// let workspace = make_workspace();
/// assert!(workspace.path().join("SOUL.md").exists());
/// ```
fn make_workspace() -> TempDir {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("SOUL.md"), "# Soul\nBe helpful.").unwrap();
    std::fs::write(tmp.path().join("IDENTITY.md"), "# Identity\nName: VibeWindow").unwrap();
    std::fs::write(tmp.path().join("USER.md"), "# User\nName: Test User").unwrap();
    std::fs::write(tmp.path().join("AGENTS.md"), "# Agents\nFollow instructions.").unwrap();
    std::fs::write(tmp.path().join("TOOLS.md"), "# Tools\nUse shell carefully.").unwrap();
    std::fs::write(tmp.path().join("HEARTBEAT.md"), "# Heartbeat\nCheck status.").unwrap();
    std::fs::write(tmp.path().join("MEMORY.md"), "# Memory\nUser likes Rust.").unwrap();
    tmp
}

// === 测试子模块声明 ===
// 这些模块包含针对 channels 功能不同方面的专门测试

mod approval_commands; // 审批命令处理测试
mod command_parsing; // 命令解析逻辑测试
mod config_loading; // 配置加载验证测试
mod health_supervisor; // 健康监督器功能测试
mod history_management; // 历史记录管理测试
mod identity; // 身份验证和管理测试
// message_dispatch 在 channels 根注册，以保留聚焦验收过滤器。
mod routing; // 路由算法和策略测试
mod session_history; // 会话历史角色映射测试
mod system_prompt; // 系统提示词构建测试
mod timeout_budget; // 超时预算分配测试
mod tool_execution; // 工具执行流程测试
mod util; // 测试辅助工具函数
mod vision; // 视觉功能集成测试
