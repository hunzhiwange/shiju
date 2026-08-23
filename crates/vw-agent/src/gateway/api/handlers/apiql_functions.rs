//! 与 PHP ApiQL 辅助函数兼容的 Rust 内部调用入口。
//!
//! ApiQL PHP helper compatible internal functions.
//!
//! These helpers mirror PHP `api_ql*` functions but execute directly in Rust:
//! no HTTP loopback, no request signing, and no route dispatch.

use vw_api_types::apiql::ApiQlListMode;
use vw_api_types::id::TenantScope;

use crate::app::agent::gateway::ApiError;

use super::handlers::load_connection;
use super::params::{ApiQlReadParams, normalize_apiql_query};
use super::runtime::{ApiQlRuntimeResponse, run_apiql_query};
use super::schema::load_entity_registry;

pub(crate) async fn api_ql(
    tenant: TenantScope,
    entity_class: &str,
    raw_query: &str,
) -> Result<ApiQlRuntimeResponse, ApiError> {
    api_ql_with_mode(tenant, entity_class, raw_query, ApiQlListMode::Index).await
}

pub(crate) async fn api_ql_list_count(
    tenant: TenantScope,
    entity_class: &str,
    raw_query: &str,
) -> Result<ApiQlRuntimeResponse, ApiError> {
    api_ql_with_mode(tenant, entity_class, raw_query, ApiQlListMode::IndexCount).await
}

pub(crate) async fn api_ql_list_only(
    tenant: TenantScope,
    entity_class: &str,
    raw_query: &str,
) -> Result<ApiQlRuntimeResponse, ApiError> {
    api_ql_with_mode(tenant, entity_class, raw_query, ApiQlListMode::IndexOnly).await
}

pub(crate) async fn api_ql_list_page(
    tenant: TenantScope,
    entity_class: &str,
    raw_query: &str,
) -> Result<ApiQlRuntimeResponse, ApiError> {
    api_ql_with_mode(tenant, entity_class, raw_query, ApiQlListMode::IndexPage).await
}

pub(crate) async fn api_ql_with_mode(
    tenant: TenantScope,
    entity_class: &str,
    raw_query: &str,
    mode: ApiQlListMode,
) -> Result<ApiQlRuntimeResponse, ApiError> {
    let params = normalize_apiql_query(raw_query, mode)?;
    api_ql_with_params(tenant, entity_class, &params).await
}

pub(crate) async fn api_ql_with_params(
    tenant: TenantScope,
    entity_class: &str,
    params: &ApiQlReadParams,
) -> Result<ApiQlRuntimeResponse, ApiError> {
    let registry = load_entity_registry(tenant.clone()).await?;
    let entity = registry.resolve(entity_class)?;
    let connection = load_connection(tenant.clone(), &entity.config().connection_id).await?;
    run_apiql_query(tenant, connection, &registry, entity, params).await
}
