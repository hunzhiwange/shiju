//! Market discovery uses the existing project skill read grant; it never installs.
use axum::{Extension, Json, Router, extract::{Query, rejection::QueryRejection},
    http::{HeaderMap, StatusCode}, response::{IntoResponse, Response}, routing::get};
use vw_api_types::{agent_apps_skills::search_api::*, id::{TenantScope, ProjectScope}};
use vw_agent_apps::skills::market_search::{self, MarketSearchError};

pub(crate) fn router() -> Router {
    Router::new().route("/agent-apps/skills/search", get(search))
        .layer(axum::middleware::map_response(async |mut response: Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}

async fn search(Extension(tenant): Extension<TenantScope>, headers: HeaderMap,
    query: Result<Query<MarketSearchQuery>, QueryRejection>,
) -> Result<Json<MarketSearchResponse>, SearchError> {
    let Query(query) = query.map_err(|_| SearchError(MarketSearchFailure::InvalidInput))?;
    if !query.is_valid() { return Err(SearchError(MarketSearchFailure::InvalidInput)); }
    let scope = ProjectScope::new(tenant, query.project_id.clone())
        .map_err(|_| SearchError(MarketSearchFailure::InvalidInput))?;
    let target = query.target_host_id.clone();
    tokio::task::spawn_blocking(move || {
        let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
        super::super::agent_apps::authorize_skill_repos(policy.as_deref(), &headers,
            &scope, &target, false).map_err(|status| SearchError(match status {
                StatusCode::UNAUTHORIZED => MarketSearchFailure::Unauthenticated,
                StatusCode::FORBIDDEN => MarketSearchFailure::Forbidden,
                StatusCode::NOT_FOUND => MarketSearchFailure::NotFound,
                StatusCode::NOT_IMPLEMENTED => MarketSearchFailure::UnsupportedTarget,
                _ => MarketSearchFailure::Unavailable,
            }))
    }).await.map_err(|_| SearchError(MarketSearchFailure::Unavailable))??;
    let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build()
        .map_err(|_| SearchError(MarketSearchFailure::Unavailable))?;
    let result = market_search::search_skills_sh(&client, &query.query, query.limit, query.offset)
        .await.map_err(|error| SearchError(market_failure(error)))?;
    Ok(Json(MarketSearchResponse { query: result.query, total_count: result.total_count,
        skills: result.skills.into_iter().map(|s| MarketSkill {
            key: s.key, name: s.name, directory: s.directory, repo_owner: s.repo_owner,
            repo_name: s.repo_name, repo_branch: s.repo_branch, installs: s.installs,
            readme_url: s.readme_url,
        }).collect() }))
}

fn market_failure(error: MarketSearchError) -> MarketSearchFailure {
    match error {
        MarketSearchError::Forbidden => MarketSearchFailure::Forbidden,
        MarketSearchError::NotFound => MarketSearchFailure::NotFound,
        MarketSearchError::RateLimited => MarketSearchFailure::RateLimited,
        MarketSearchError::Timeout => MarketSearchFailure::Timeout,
        MarketSearchError::Transport => MarketSearchFailure::Transport,
        MarketSearchError::InvalidResponse => MarketSearchFailure::InvalidResponse,
        MarketSearchError::HttpStatus { status } => MarketSearchFailure::HttpStatus { status },
    }
}
struct SearchError(MarketSearchFailure);
impl IntoResponse for SearchError {
    fn into_response(self) -> Response {
        let status = match self.0 {
            MarketSearchFailure::InvalidInput => StatusCode::BAD_REQUEST,
            MarketSearchFailure::Unauthenticated => StatusCode::UNAUTHORIZED,
            MarketSearchFailure::Forbidden => StatusCode::FORBIDDEN,
            MarketSearchFailure::NotFound => StatusCode::NOT_FOUND,
            MarketSearchFailure::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            MarketSearchFailure::UnsupportedTarget => StatusCode::NOT_IMPLEMENTED,
            MarketSearchFailure::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            MarketSearchFailure::Timeout => StatusCode::GATEWAY_TIMEOUT,
            _ => StatusCode::BAD_GATEWAY,
        };
        (status, Json(self.0)).into_response()
    }
}
