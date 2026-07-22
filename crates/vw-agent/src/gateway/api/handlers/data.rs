//! AI-DATA 路由模块。
//!
//! 本模块为 `/v1/data` 提供基础 REST 接口，覆盖：
//! - 全局设置读取与更新
//! - 数据连接增删改查与激活
//! - 报表配置增删改查
//! - 统一查询与 AI 查询入口

use axum::{
    Router,
    routing::{get, post},
};

use crate::app::agent::gateway::state::AppState;

#[path = "data_ai_support.rs"]
mod ai_support;
#[path = "data_handlers.rs"]
mod handlers;
#[path = "data_runtime.rs"]
pub(crate) mod runtime;
#[path = "data_storage_support.rs"]
pub(crate) mod storage_support;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/data/settings", get(handlers::data_settings_get).put(handlers::data_settings_put))
        .route(
            "/data/connections",
            get(handlers::data_connections_list).post(handlers::data_connection_create),
        )
        .route(
            "/data/connections/{connection_id}",
            get(handlers::data_connection_get)
                .put(handlers::data_connection_update)
                .delete(handlers::data_connection_delete),
        )
        .route(
            "/data/connections/{connection_id}/activate",
            post(handlers::data_connection_activate),
        )
        .route("/data/connections/{connection_id}/test", post(handlers::data_connection_test))
        .route("/data/connections/{connection_id}/catalog", get(handlers::data_connection_catalog))
        .route("/data/reports", get(handlers::data_reports_list).post(handlers::data_report_create))
        .route(
            "/data/reports/{report_id}",
            get(handlers::data_report_get)
                .put(handlers::data_report_update)
                .delete(handlers::data_report_delete),
        )
        .route("/data/query", post(handlers::data_query))
        .route("/data/ai/query", post(handlers::data_ai_query))
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod data_tests;
