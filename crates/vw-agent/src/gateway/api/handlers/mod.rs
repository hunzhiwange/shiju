//! # API 处理器模块
//!
//! 本模块提供网关层的 HTTP API 请求处理器集合，作为 `gateway/api` 子系统的核心入口。
//!
//! ## 模块职责
//!
//! - 组织和管理各类 API 端点的处理器实现
//! - 提供统一的处理器导出接口，供上层路由注册使用
//! - 按功能域划分子模块，保持代码的模块化和可维护性
//!
//! ## 子模块结构
//!
//! | 模块 | 功能描述 |
//! |------|----------|
//! | `config` | 配置管理相关处理器（获取/更新配置） |
//! | `cron` | 定时任务管理处理器（添加/删除/列出任务） |
//! | `integrations` | 第三方集成处理器（集成列表/凭证/设置） |
//! | `memory` | 记忆存储处理器（存储/列出/删除记忆） |
//! | `status` | 系统状态处理器（健康检查/运行状态） |
//! | `tools` | 工具管理处理器（CLI工具/成本/诊断） |
//!
//! ## 使用示例
//!
//! ```ignore
//! use crate::app::agent::gateway::api::handlers::{
//!     handle_api_health,
//!     handle_api_config_get,
//!     handle_api_cron_list,
//! };
//!
//! // 在路由中注册处理器
//! router.route("/health", get(handle_api_health));
//! router.route("/config", get(handle_api_config_get));
//! router.route("/cron", get(handle_api_cron_list));
//! ```

// ============================================================================
// 子模块声明
// ============================================================================

/// 配置管理处理器模块
///
/// 提供配置的获取（GET）和更新（PUT）操作处理器。
pub mod config;

/// 身份认证路由处理器模块
pub mod auth;

/// Agent Scheduler 路由处理器模块
pub mod agent_scheduler;

/// Desktop 本地状态网关化处理器模块
pub mod desktop;

/// Gateway skills 目录处理器模块
pub(crate) mod desktop_skills;

/// Gateway 插件目录与本地管理处理器模块
pub(crate) mod plugins;

/// 统一技能市场只读 API 处理器模块
pub(crate) mod skill_market;

/// SHIJU account gateway API handlers.
pub(crate) mod shiju_account;
pub(crate) mod shiju_apps;
pub(crate) mod shiju_skills;

/// 定时任务（Cron）管理处理器模块
///
/// 提供定时任务的添加、删除和列表查询操作处理器。
pub mod cron;

/// AI-DATA 路由处理器模块
pub mod data;

/// ApiQL V1 兼容路由模块
pub(crate) mod apiql;

/// 第三方集成处理器模块
///
/// 处理外部服务集成的配置、凭证和设置管理。
pub mod integrations;

/// 文件与搜索路由处理器模块
pub mod file;

/// Git 路由处理器模块（commit/merge 走 git2）
pub mod git;

/// `/v1/git/command` 通用 Git CLI 协议（仍依赖系统 Git）
pub(crate) mod git_command;

/// 全局网关路由处理器模块
pub mod global;

/// 知识库路由处理器模块
pub mod knowledge;

/// 实例相关路由处理器模块
pub mod instance;

/// 记忆存储处理器模块
///
/// 提供代理记忆的存储、检索和管理操作处理器。
pub mod memory;

/// MCP 检查处理器模块
pub mod mcp;

/// 杂项元数据路由处理器模块
pub mod misc;

/// 权限请求路由处理器模块
pub mod permission;

/// 项目管理路由处理器模块
pub mod project;

/// Provider 路由处理器模块
pub mod provider;

/// PTY 路由处理器模块
pub mod pty;

/// 问题请求路由处理器模块
pub mod question;

/// 可恢复 SSE 运行存储。
pub(super) mod resumable_stream;

/// Session 路由处理器模块
pub mod session;

/// 系统状态处理器模块
///
/// 提供系统健康检查和运行状态查询处理器。
pub mod status;

/// 员工任务流本地 IPC 消息路由处理器模块
pub mod staff_ipc;

/// 任务池 worktree 池路由处理器模块
pub mod task_pool;

/// 任务生命周期 workflow 路由处理器模块
pub mod task_lifecycle;

/// 聊天工作流执行（task-plan-run）SSE 路由处理器模块
pub mod task_plan_run;

/// 任务池 Matrix 协同事件路由处理器模块
pub mod task_pool_matrix;

/// 工具管理处理器模块
///
/// 提供 CLI 工具、成本统计和系统诊断相关的处理器。
pub mod tools;

/// Workflow 执行路由处理器模块
pub mod workflow;
pub(crate) mod workflow_store;

// ============================================================================
// 处理器重新导出
// ============================================================================

/// 重新导出配置处理器
///
/// - `handle_api_config_get`: 获取当前配置（GET /api/config）
/// - `handle_api_config_put`: 更新配置（PUT /api/config）
pub use config::{handle_api_config_get, handle_api_config_put};

/// 重新导出定时任务处理器
///
/// - `handle_api_cron_add`: 添加新的定时任务（POST /api/cron）
/// - `handle_api_cron_delete`: 删除指定定时任务（DELETE /api/cron/:id）
/// - `handle_api_cron_list`: 列出所有定时任务（GET /api/cron）
/// - `handle_api_cron_runs`: 列出任务执行历史（GET /api/cron/:id/runs）
/// - `handle_api_cron_update`: 更新指定定时任务（PATCH /api/cron/:id）
pub use cron::{
    handle_api_cron_add, handle_api_cron_delete, handle_api_cron_list, handle_api_cron_runs,
    handle_api_cron_update,
};

/// 重新导出集成管理处理器
///
/// - `handle_api_integrations`: 获取集成列表（GET /api/integrations）
/// - `handle_api_integrations_settings`: 获取/更新集成设置
/// - `handle_api_integration_credentials_put`: 更新集成凭证（PUT /api/integrations/:id/credentials）
pub use integrations::{
    handle_api_integration_credentials_put, handle_api_integrations,
    handle_api_integrations_settings,
};

/// 重新导出记忆存储处理器
///
/// - `handle_api_memory_store`: 存储新记忆（POST /api/memory）
/// - `handle_api_memory_list`: 列出记忆条目（GET /api/memory）
/// - `handle_api_memory_delete`: 删除指定记忆（DELETE /api/memory/:id）
pub use memory::{handle_api_memory_delete, handle_api_memory_list, handle_api_memory_store};

/// 重新导出 MCP 检查处理器
pub use mcp::handle_api_mcp_inspect;

/// 重新导出系统状态处理器
///
/// - `handle_api_health`: 健康检查端点（GET /api/health）
/// - `handle_api_status`: 系统运行状态（GET /api/status）
pub use status::{handle_api_health, handle_api_status};

/// 重新导出任务池 Matrix 协同事件处理器
pub use task_pool_matrix::{
    handle_api_task_pool_matrix_events, handle_api_task_pool_matrix_publish,
};

/// 重新导出员工任务流 IPC 消息处理器

/// 重新导出任务池 worktree pool 处理器
pub use task_pool::{
    handle_api_task_pool_worktree_acquire, handle_api_task_pool_worktree_maintain,
    handle_api_task_pool_worktree_merge_lock, handle_api_task_pool_worktree_merge_unlock,
    handle_api_task_pool_worktree_recycle, handle_api_task_pool_worktree_release,
    handle_api_task_pool_worktree_slot_recycle, handle_api_task_pool_worktree_snapshot,
};

/// 重新导出工具管理处理器
///
/// - `handle_api_tools`: 获取可用工具列表（GET /api/tools）
/// - `handle_api_cli_tools`: CLI 工具管理（GET /api/cli-tools）
/// - `handle_api_doctor`: 系统诊断（GET /api/doctor）
pub use tools::{handle_api_cli_tools, handle_api_doctor, handle_api_tools};

#[cfg(test)]
mod tests;
