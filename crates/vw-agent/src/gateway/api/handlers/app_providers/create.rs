use super::*;
use vw_api_app_provider_base::create::AppProviderCreate;
#[allow(clippy::too_many_arguments)]
pub async fn create(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
    policy: Option<Extension<super::claude::Policy>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(query): axum::extract::Query<
        vw_api_app_aggregation::AggregationWriteQuery,
    >,
    #[cfg(target_os = "macos")] control: Option<
        Extension<super::super::app_routing::RoutingControl>,
    >,
    body: Result<
        Json<vw_api_app_provider_base::claude::ClaudeOr<AppProviderCreate>>,
        JsonRejection,
    >,
) -> Response {
    gateway_app_providers::create::create(Extension(tenant), database, Path((app, id)), policy,
        headers, axum::extract::Query(query),
        #[cfg(target_os = "macos")]
        Some(Extension(super::publication_host::control(control.map(|Extension(c)| c)))), body).await
}
