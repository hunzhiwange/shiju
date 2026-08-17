//! REST API 处理器模块 - Web 仪表盘的后端 API 接口
//!
//! 本模块为 VibeWindow 的 Web 仪表盘提供完整的 RESTful API 接口，
//! 实现代理系统的远程管理和监控能力。
//!
//! # 模块架构
//!
//! 本模块按功能领域划分子模块：
//! - [`auth`] - 身份认证与授权相关接口
//! - [`handlers`] - 核心 API 处理函数集合
//! - [`integrations`] - 第三方集成配置接口
//! - [`secrets`] - 敏感信息（密钥、凭证）管理接口
//! - [`types`] - API 请求/响应的数据结构定义
//!
//! # 认证机制
//!
//! 当网关启用鉴权时，`/api/*` 路由要求持有有效 skey，并通过
//! `Authorization: Bearer <skey>` 进行身份验证。
//!
//! # 公开接口
//!
//! ## 配置管理
//! - [`handle_api_config_get`] - 获取当前代理配置
//! - [`handle_api_config_put`] - 更新代理配置
//!
//! ## 系统状态与诊断
//! - [`handle_api_status`] - 获取代理运行状态
//! - [`handle_api_health`] - 健康检查端点
//! - [`handle_api_doctor`] - 系统诊断与问题检测
//!
//! ## 工具管理
//! - [`handle_api_tools`] - 列出可用工具
//!
//! ## 定时任务（Cron）
//! - [`handle_api_cron_list`] - 列出所有定时任务
//! - [`handle_api_cron_add`] - 添加新的定时任务
//! - [`handle_api_cron_delete`] - 删除指定定时任务
//!
//! ## 记忆存储
//! - [`handle_api_memory_list`] - 查询记忆条目
//! - [`handle_api_memory_store`] - 存储新记忆
//! - [`handle_api_memory_delete`] - 删除记忆条目
//!
//! ## 集成管理
//! - [`handle_api_integrations`] - 列出已配置的集成
//! - [`handle_api_integrations_settings`] - 获取/更新集成设置
//! - [`handle_api_integration_credentials_put`] - 更新集成凭证
//!
//! # 数据类型
//!
//! - [`CronAddBody`] - 添加定时任务的请求体
//! - [`MemoryQuery`] - 记忆查询参数
//! - [`MemoryStoreBody`] - 存储记忆的请求体
//! - [`IntegrationCredentialsUpdateBody`] - 更新集成凭证的请求体

/// 身份认证与授权子模块
///
/// 提供 skey Bearer 验证等安全相关功能。
pub mod auth;

/// 核心 API 处理函数子模块
///
/// 包含所有 REST API 端点的处理函数实现。
pub mod handlers;

/// 第三方集成配置子模块
///
/// 管理 Telegram、Slack、Discord 等外部通道的集成配置。
pub mod integrations;

/// 敏感信息管理子模块
///
/// 提供密钥、凭证等敏感数据的安全存储与访问接口。
pub mod secrets;

/// API 数据类型定义子模块
///
/// 定义 API 请求体、响应体及共享数据结构。
pub mod types;

/// 重新导出核心 API 处理函数
///
/// 将各子模块中的处理函数统一导出，便于路由注册和外部调用。
/// 调用方可通过 `crate::app::agent::gateway::api::handle_api_*` 直接访问。
pub use handlers::{
    handle_api_config_get, handle_api_config_put, handle_api_cron_add,
    handle_api_cron_delete, handle_api_cron_list, handle_api_cron_runs, handle_api_cron_update,
    handle_api_doctor, handle_api_health, handle_api_integration_credentials_put,
    handle_api_integrations, handle_api_integrations_settings, handle_api_mcp_inspect,
    handle_api_memory_delete, handle_api_memory_list, handle_api_memory_store, handle_api_status,
    handle_api_task_pool_matrix_events, handle_api_task_pool_matrix_publish,
    handle_api_task_pool_worktree_acquire, handle_api_task_pool_worktree_maintain,
    handle_api_task_pool_worktree_merge_lock, handle_api_task_pool_worktree_merge_unlock,
    handle_api_task_pool_worktree_recycle, handle_api_task_pool_worktree_release,
    handle_api_task_pool_worktree_slot_recycle, handle_api_task_pool_worktree_snapshot,
    handle_api_tools,
};

/// 重新导出 API 数据类型
///
/// 导出请求体结构体，供路由处理器和客户端代码使用。
pub use types::{
    CronAddBody, CronUpdateBody, IntegrationCredentialsUpdateBody, MemoryQuery, MemoryStoreBody,
};

/// 单元测试模块
///
/// 测试文件位于 `tests.rs`，与主模块同级目录。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
