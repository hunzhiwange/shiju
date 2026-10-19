//! 路由器构建模块
//!
//! 本模块提供 HTTP 路由器的统一构建功能，负责将所有 API 路由模块整合为完整的
//! 应用路由器实例。作为网关层的核心入口点，该模块协调路由注册、中间件配置
//! 以及跨域资源共享（CORS）策略的设置。
//!
//! # 主要职责
//!
//! - 整合来自各子模块的路由定义（auth、config、instance 等）
//! - 配置全局 CORS 中间件，基于白名单进行来源验证
//! - 注入基础认证中间件，保护所有 API 端点
//!
//! # 路由模块结构
//!
//! | 模块 | 路径前缀 | 用途 |
//! |------|---------|------|
//! | global | `/` | 全局健康检查与状态探针 |
//! | auth | `/auth` | 认证相关接口 |
//! | misc | `/misc` | 杂项工具接口 |
//! | instance | `/instance` | 实例管理接口 |
//! | config | `/config` | 配置管理接口 |
//! | provider | `/provider` | Provider 管理接口 |
//! | file | `/file` | 文件操作接口 |
//! | project | `/project` | 项目管理接口 |
//! | permission | `/permission` | 权限管理接口 |
//! | question | `/question` | 问答接口 |
//! | pty | `/pty` | 伪终端接口 |
//! | session | `/session` | 会话管理接口 |
//!
//! # 安全说明
//!
//! 所有路由默认受基础认证中间件保护。CORS 策略采用白名单机制，
//! 仅允许配置中的来源域进行跨域请求。

use std::sync::Arc;

use axum::Router;
use axum::http::HeaderValue;
use axum::middleware;
use tower_http::cors::AllowOrigin;
use tower_http::cors::Any;
use tower_http::cors::CorsLayer;

use crate::app::agent::gateway::api::handlers;
use crate::app::agent::gateway::middleware::basic_auth_middleware;
use crate::app::agent::gateway::middleware::cors_origin_allowed;
use crate::app::agent::gateway::middleware::tenant_scope_middleware;

/// 构建 HTTP 路由器实例
///
/// 创建并配置完整的 Axum 路由器，整合所有业务路由模块，并应用必要的中间件层。
/// 路由器采用洋葱式中间件架构，请求按从外到内的顺序依次经过 CORS 和认证中间件。
///
/// # 参数
///
/// - `cors_whitelist`: CORS 白名单列表，包含允许跨域请求的来源域字符串。
///   该列表会被共享所有权包装（`Arc`），以便在中间件闭包中安全使用。
///
/// # 返回值
///
/// 返回配置完成的 `Router` 实例，可直接绑定到 Axum 服务器使用。
///
/// # 中间件执行顺序
///
/// 请求处理流程（从外到内）：
/// 1. **CORS 中间件**: 验证请求来源是否在白名单中，处理预检请求
/// 2. **认证中间件**: 执行基础认证，拒绝未授权请求
/// 3. **业务路由**: 分发到具体的处理函数
///
/// # CORS 配置
///
/// - `allow_origin`: 使用谓词模式，通过 `cors_origin_allowed` 函数验证来源
/// - `allow_methods`: 允许所有 HTTP 方法（GET、POST、PUT、DELETE 等）
/// - `allow_headers`: 允许所有请求头
///
/// # 示例
///
/// ```rust,ignore
/// use crate::app::agent::gateway::router::build_router;
///
/// // 创建允许本地开发服务器的 CORS 白名单
/// let whitelist = vec![
///     "http://localhost:3000".to_string(),
///     "http://127.0.0.1:3000".to_string(),
/// ];
///
/// // 构建路由器
/// let router = build_router(whitelist);
///
/// // 绑定到服务器
/// let app = router;
/// let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
/// axum::serve(listener, app).await?;
/// ```
///
/// # 路由合并说明
///
/// 使用 `Router::merge` 方法将各子模块的路由合并到主路由器。
/// 每个子模块负责定义自己的路径前缀和具体路由处理器。
/// 合并顺序不影响路由匹配逻辑，但建议按功能域组织。
pub(crate) fn build_router(cors_whitelist: Vec<String>) -> Router {
    // 将 CORS 白名单包装为 Arc，以便在 CORS 谓词闭包中安全共享
    // 避免每次请求时克隆整个列表
    let whitelist = Arc::new(cors_whitelist);

    // 构建 CORS 中间件层
    // - 使用谓词模式进行来源验证，支持动态判断
    // - allow_methods(Any) 允许任意 HTTP 方法
    // - allow_headers(Any) 允许任意请求头
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin: &HeaderValue, _| {
            cors_origin_allowed(origin, whitelist.as_ref())
        }))
        .allow_methods(Any)
        .allow_headers(Any);

    // 同时保留无版本前缀路由，并挂载客户端实际使用的 /v1 前缀路由。
    // 中间件按从下到上的顺序应用（洋葱模型）。
    Router::new()
        // CORS（最外层）-> Basic 认证 -> 租户解析 -> 客户端业务路由。
        .merge(handler_router().layer(middleware::from_fn(tenant_scope_middleware)))
        .nest("/v1", handler_router().layer(middleware::from_fn(tenant_scope_middleware)))
        // Basic 认证在租户解析前执行；OPTIONS 由各中间件放行。
        .layer(middleware::from_fn(basic_auth_middleware))
        // CORS 中间件：处理跨域请求（最外层）
        .layer(cors)
}

fn handler_router() -> Router {
    Router::new()
        // 全局路由：健康检查、系统状态等
        .merge(handlers::global::router())
        // 认证路由：登录、令牌管理等
        .merge(handlers::auth::router())
        // 杂项路由：工具类接口
        .merge(handlers::misc::router())
        // 实例路由：Agent 实例生命周期管理
        .merge(handlers::instance::router())
        // 配置路由：运行时配置读写
        .merge(handlers::config::router())
        // Provider 路由：模型提供者管理
        .merge(handlers::provider::router())
        // Channel QR-code authorization routes.
        .merge(handlers::channel_qr::router())
        // 知识库路由：数据集、文档入库与召回
        .merge(handlers::knowledge::router())
        // 文件路由：文件系统操作
        .merge(handlers::file::router())
        // Git 路由：选择性暂存与提交
        .merge(handlers::git::router())
        // 项目路由：项目管理接口
        .merge(handlers::project::router())
        // 权限路由：访问控制管理
        .merge(handlers::permission::router())
        // 问答路由：问答交互接口
        .merge(handlers::question::router())
        // PTY 路由：伪终端会话
        .merge(handlers::pty::router())
        // 任务池 worktree 池路由
        .merge(handlers::task_pool::router())
        // Agent Scheduler 路由：任务、wake、事件与 outbox 测试入口
        .merge(handlers::agent_scheduler::router())
        // 任务生命周期 workflow 路由
        .merge(handlers::task_lifecycle::router())
        // Workflow 应用路由：Dify Workflow 保存与列表
        .merge(handlers::workflow::applications_router())
        // Skills 与桌面端轻量 API：偏好、工具内容、服务生命周期等
        .merge(handlers::shiju_account::router())
        .merge(handlers::shiju_apps::router())
        .merge(handlers::shiju_skills::router())
        .merge(handlers::skill_market::router())
        .merge(handlers::desktop_skills::router())
        .merge(handlers::plugins::router())
        .merge(handlers::desktop::router())
}

#[cfg(test)]
#[path = "router_tests.rs"]
mod router_tests;
