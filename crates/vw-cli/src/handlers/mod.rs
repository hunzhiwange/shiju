//! # 请求处理器模块
//!
//! 本模块负责组织和导出 VibeWindow 代理的各种请求处理器。
//!
//! ## 模块结构
//!
//! - [`estop`] - 紧急停止（Emergency Stop）处理器，用于处理紧急停止请求
//! - [`security`] - 安全策略处理器，用于处理安全相关的请求和验证
//! - [`api`] - 远程 Gateway API CLI 的共享边界
//!
//! ## 设计原则
//!
//! 所有处理器都遵循以下原则：
//! - 快速失败：对不安全或无效状态立即返回错误
//! - 最小权限：默认拒绝访问，仅在明确授权时允许
//! - 可观测性：记录所有重要的安全相关操作
//!
//! ## 使用示例
//!
//! ```ignore
//! use vibe_agent::handlers;
//!
//! // 在路由配置中使用处理器
//! // handlers::estop::handle_estop_request(...)
//! // handlers::security::handle_security_request(...)
//! ```

/// 紧急停止（Emergency Stop）处理器模块
///
/// 提供紧急停止功能的请求处理，用于在检测到危险或异常情况时
/// 立即中断代理的执行流程。
pub(crate) mod estop;

/// 远程 Gateway API CLI 处理器模块。
pub(crate) mod api;

/// 安全策略处理器模块
///
/// 提供安全相关的请求处理，包括：
/// - 访问控制验证
/// - 权限检查
/// - 安全策略执行
pub(crate) mod security;
