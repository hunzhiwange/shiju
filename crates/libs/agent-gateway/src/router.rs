//! Gateway CORS, authentication and tenant middleware composition.
use std::sync::Arc;
use axum::{Router, http::HeaderValue, middleware};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use crate::middleware::{basic_auth_middleware, cors_origin_allowed, tenant_scope_middleware};

pub fn build_router_with_auth(
    cors_whitelist: Vec<String>, auth: Router, handlers: Router, versioned_handlers: Router, voice: Router,
) -> Router {
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
        .merge(handlers.layer(middleware::from_fn(tenant_scope_middleware)))
        .nest("/v1", versioned_handlers.layer(middleware::from_fn(tenant_scope_middleware)))
        .nest("/api", voice)
        // Basic 认证在租户解析前执行；OPTIONS 由各中间件放行。
        .layer(middleware::from_fn(basic_auth_middleware))
        .merge(auth)
        // CORS 中间件：处理跨域请求（最外层）
        .layer(cors)
}
