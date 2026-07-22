//! ApiQL V1 兼容入口模块。
//!
//! 本模块接入实体 schema 注册表、查询运行时与 PHP 兼容 HTTP 路由。

use axum::{Router, routing::get};

#[path = "apiql_handlers.rs"]
mod handlers;

#[path = "apiql_params.rs"]
pub(crate) mod params;

#[path = "apiql_schema.rs"]
pub(crate) mod schema;

#[path = "apiql_data_bridge.rs"]
pub(crate) mod data_bridge;

#[path = "apiql_sql.rs"]
pub(crate) mod sql;

#[path = "apiql_runtime.rs"]
pub(crate) mod runtime;

#[path = "apiql_formatters.rs"]
pub(crate) mod formatters;

#[path = "apiql_transformers.rs"]
pub(crate) mod transformers;

#[path = "apiql_functions.rs"]
pub(crate) mod functions;

#[path = "apiql_entities.rs"]
pub(crate) mod entities;

#[path = "apiql_virtuals.rs"]
pub(crate) mod virtuals;

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/apiQL/v1:{entity_class}", get(handlers::apiql_index))
        .route("/apiQL/v1:{entity_class}/", get(handlers::apiql_index))
        .route("/apiQL/v1:{entity_class}/list-only", get(handlers::apiql_index_only))
        .route("/apiQL/v1:{entity_class}/list-only/", get(handlers::apiql_index_only))
        .route("/apiQL/v1:{entity_class}/list-count", get(handlers::apiql_index_count))
        .route("/apiQL/v1:{entity_class}/list-count/", get(handlers::apiql_index_count))
        .route("/apiQL/v1:{entity_class}/list-page", get(handlers::apiql_index_page))
        .route("/apiQL/v1:{entity_class}/list-page/", get(handlers::apiql_index_page))
}

#[cfg(test)]
#[path = "apiql_handlers_tests.rs"]
mod apiql_handlers_tests;

#[cfg(test)]
#[path = "apiql_schema_tests.rs"]
mod apiql_schema_tests;

#[cfg(test)]
#[path = "apiql_params_tests.rs"]
mod apiql_params_tests;

#[cfg(test)]
#[path = "apiql_data_bridge_tests.rs"]
mod apiql_data_bridge_tests;

#[cfg(test)]
#[path = "apiql_sql_tests.rs"]
mod apiql_sql_tests;

#[cfg(test)]
#[path = "apiql_runtime_tests.rs"]
mod apiql_runtime_tests;

#[cfg(test)]
#[path = "apiql_compat_tests.rs"]
mod apiql_compat_tests;

#[cfg(test)]
#[path = "apiql_functions_tests.rs"]
mod apiql_functions_tests;
