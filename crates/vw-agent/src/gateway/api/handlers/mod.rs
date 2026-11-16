//! # API 处理器模块
//!
//! 本模块保留在网关宿主中，仅组合各领域处理器模块并重导出路由注册所需入口。
//! 业务实现由各领域模块及其下层库持有；本模块不持有共享状态或执行处理逻辑。
//! 将该入口单独建库会引入跨领域处理器依赖，因此保留宿主组装职责。
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
//! use vw_agent::gateway::api::handlers::{
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

/// Channel QR-code authorization handlers.
pub(crate) mod channel_qr;

/// Agent Scheduler 路由处理器模块
pub mod agent_scheduler;

/// Desktop 本地状态网关化处理器模块
pub mod desktop;

/// Gateway skills 目录处理器模块
pub(crate) mod desktop_skills;

// Desktop installation handlers are composed by gateway-desktop.
/// Gateway 插件目录与本地管理处理器模块
pub(crate) mod plugins;

/// Gateway plugin hook management and command audit handlers.
pub(crate) mod hooks;

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

/// 第三方集成处理器模块
///
/// 处理外部服务集成的配置、凭证和设置管理。
pub mod integrations;

/// 文件与搜索路由处理器模块
pub mod file;

/// Git 路由处理器模块（commit/merge 走 git2）
pub mod git;

// `/v1/git/command` is composed inside gateway-git.

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
mod mcp_host;

/// 杂项元数据路由处理器模块
pub mod misc;

/// 权限请求路由处理器模块
pub mod permission;

/// 项目管理路由处理器模块
pub mod project;
pub mod worktree_management;

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

/// 任务池 worktree 池路由处理器模块
pub mod task_pool;

/// 任务生命周期 workflow 路由处理器模块
pub mod task_lifecycle;

/// 聊天工作流执行（task-plan-run）SSE 路由处理器模块
pub mod task_plan_run;

/// 工具管理处理器模块
///
/// 提供 CLI 工具、成本统计和系统诊断相关的处理器。
pub mod tools;

/// Workflow 执行路由处理器模块
pub mod workflow;

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
    handle_api_cron_add, handle_api_cron_delete, handle_api_cron_history, handle_api_cron_list,
    handle_api_cron_preview, handle_api_cron_runs, handle_api_cron_update,
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
pub use mcp::{handle_api_mcp_app_call, handle_api_mcp_inspect};

/// 重新导出系统状态处理器
///
/// - `handle_api_health`: 健康检查端点（GET /api/health）
/// - `handle_api_status`: 系统运行状态（GET /api/status）
pub use status::{handle_api_health, handle_api_status};

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
/// - `handle_api_doctor`: 系统诊断（GET /api/doctor）
pub use tools::{handle_api_doctor, handle_api_tools};

#[cfg(test)]
mod dynamic_tools_tests;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod dynamic_tools;

pub mod agent_apps_error;
// Icon error tests live with gateway-agent-apps.

#[cfg(test)]
mod agent_apps_read_tests;

#[cfg(target_os = "macos")]
pub(crate) mod app_routing;

pub(crate) mod agent_apps;

#[cfg(all(test, target_os = "macos"))]
mod agent_apps_high_availability_tests;

#[cfg(all(test, target_os = "macos"))]
mod agent_apps_ha_acceptance_tests;

pub(crate) mod agent_apps_auth_read;
#[cfg(all(test, target_os = "macos"))]
mod agent_apps_auth_read_tests;

// agent_apps_auth_binding is owned by gateway-agent-apps.
#[cfg(all(test, target_os = "macos"))]
mod agent_apps_auth_binding_tests;

// agent_apps_auth_cancel is owned by gateway-agent-apps.
#[cfg(all(test, target_os = "macos"))]
mod agent_apps_auth_cancel_tests;

// agent_apps_auth_default is owned by gateway-agent-apps.
#[cfg(all(test, target_os = "macos"))]
mod agent_apps_auth_default_tests;

#[cfg(all(test, target_os = "macos"))]
mod agent_apps_auth_integration_tests;

// agent_apps_auth_logout is owned by gateway-agent-apps.
#[cfg(all(test, target_os = "macos"))]
mod agent_apps_auth_logout_tests;

// agent_apps_auth_poll is owned by gateway-agent-apps.
#[cfg(all(test, target_os = "macos"))]
mod agent_apps_auth_poll_tests;

#[cfg(all(test, target_os = "macos"))]
mod agent_apps_auth_redaction_tests;

// agent_apps_auth_remove is owned by gateway-agent-apps.
#[cfg(all(test, target_os = "macos"))]
mod agent_apps_auth_remove_tests;

#[cfg(all(test, target_os = "macos"))]
mod agent_apps_auth_security_tests;

mod agent_apps_auth_start;
#[cfg(all(test, target_os = "macos"))]
mod agent_apps_auth_start_tests;

#[cfg(test)]
mod agent_apps_add_tests;

#[cfg(test)]
mod agent_apps_rename_tests;

#[cfg(test)]
mod agent_apps_delete_tests;

#[cfg(all(test, target_os = "macos"))]
mod agent_apps_acceptance_tests;

pub(crate) mod app_providers;

// Real host routing, authentication, tenant isolation and launch contracts.
#[cfg(test)]
mod app_providers_tests;

pub(crate) mod model_catalog;
#[cfg(all(test, target_os = "macos"))]
mod model_catalog_acceptance_tests;
// Model catalog edits are composed inside gateway-model.
#[cfg(all(test, target_os = "macos"))]
mod model_catalog_tests;

// Discovery routes live in gateway-model; host integration tests stay here.
#[cfg(all(test, target_os = "macos"))]
mod model_discovery_tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-mcp/src/host_tests/mcp_management_create_tests.rs"]
mod mcp_management_create_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-mcp/src/host_tests/mcp_management_delete_tests.rs"]
mod mcp_management_delete_tests;
mod mcp_management_read;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-mcp/src/host_tests/mcp_management_read_tests.rs"]
mod mcp_management_read_tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-mcp/src/host_tests/mcp_management_update_tests.rs"]
mod mcp_management_update_tests;

mod aggregation_members;
#[cfg(all(test, target_os = "macos"))]
mod aggregation_members_tests;
mod aggregation_omo;
#[cfg(all(test, target_os = "macos"))]
mod aggregation_omo_tests;
mod aggregation_omp;
mod aggregation_pi;
mod aggregation_read;
#[cfg(all(test, target_os = "macos"))]
mod aggregation_read_tests;

#[cfg(target_os = "macos")]
pub(crate) mod aggregation_read_snapshot;

/// Authenticated recording transcription.
pub(crate) mod voice;
#[cfg(test)]
pub(crate) mod voice_handoff;
#[cfg(test)]
#[path = "../../../../../libs/gateway-voice/src/host_tests/voice_handoff_session_tests.rs"]
mod voice_handoff_session_tests;

#[cfg(all(test, target_os = "macos"))]
mod aggregation_codex_settings_tests;

#[cfg(all(test, target_os = "macos"))]
mod aggregation_omp_tests;

#[cfg(all(test, target_os = "macos"))]
mod aggregation_desktop_tests;

#[cfg(target_os = "macos")]
pub(crate) mod aggregation_resync;

mod aggregation_codex_settings;

#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-mcp/src/host_tests/mcp_management_bindings_tests.rs"]
mod mcp_management_bindings_tests;


#[cfg(all(test, target_os = "macos"))]
mod aggregation_resync_tests;

mod aggregation_desktop;

#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-aggregation/tests/host/aggregation_resync_codex_tests.rs"]
mod aggregation_resync_codex_tests;

#[cfg(target_os = "macos")]
pub(crate) mod agent_apps_diagnostics;
#[cfg(all(test, target_os = "macos"))]
mod agent_apps_diagnostics_acceptance_tests;
#[cfg(all(test, target_os = "macos"))]
mod agent_apps_diagnostics_tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-mcp/src/host_tests/mcp_management_import_tests.rs"]
mod mcp_management_import_tests;

#[cfg(test)]
mod voice_settings;

#[cfg(all(test, target_os = "macos"))]
mod aggregation_publish_tests;

mod aggregation_publish;

#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-mcp/src/host_tests/mcp_management_resync_tests.rs"]
mod mcp_management_resync_tests;

mod mcp_management_resync;

#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-mcp/src/host_tests/mcp_management_probe_tests.rs"]
mod mcp_management_probe_tests;

#[cfg(all(test, target_os = "macos"))]
mod aggregation_clients_tests;

#[cfg(target_os = "macos")]
mod mcp_management_probe;

mod aggregation_clients;

#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-aggregation/tests/host/aggregation_restart_tests.rs"]
mod aggregation_restart_tests;

mod aggregation_restart;

mod aggregation_exit;

#[cfg(target_os = "macos")]
mod mcp_management_tools;

#[cfg(all(test, target_os = "macos"))]
#[path = "../../../../../libs/gateway-mcp/src/host_tests/mcp_management_tools_tests.rs"]
mod mcp_management_tools_tests;

pub(crate) mod agent_apps_skills;

pub(crate) mod agent_app_prompt_files;

pub(crate) mod agent_app_prompts_read;

pub(crate) mod agent_app_prompts_write;

pub(crate) mod agent_app_prompts_activation;

pub(crate) mod agent_app_prompt_templates;

#[cfg(target_os = "macos")]
pub(crate) mod agent_app_prompt_sync;

pub(crate) mod agent_app_prompt_shared_import;

pub(crate) mod computer_control;
// Computer-control policy tests are registered by gateway-computer-control.

pub(crate) mod project_profiles;

pub(crate) mod external_sessions;
mod backup_inventory;

pub(crate) mod agent_apps_data;

pub(crate) mod device_sync_jobs;
mod device_sync_download;
mod device_sync_projection;

mod device_sync_config;

mod device_sync_probe;

mod device_sync_tasks;

pub(crate) mod usage;
