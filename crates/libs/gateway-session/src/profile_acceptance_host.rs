//! Concrete process, request, configuration and admission ports for profile acceptance.
use std::future::Future;
use axum::{Router, Json, extract::Query, http::HeaderMap, response::Response};
use gateway_instance::host::InstanceQuery;
use gateway_types::ApiError;
use vw_api_chat::chat::GatewayChatStreamRequest;
pub trait ProfileAcceptanceHost: crate::session_host::SessionHost + crate::acp_host::AcpHost {
    type Fixture;
    type Admission: session_engine::admission_host::AdmissionHost;
    fn persist_turn(session: &str, query: &str, text: &str, agent: &str, error: Option<&str>) -> impl Future<Output = Result<(String, String), ApiError>> + Send;
    fn env_lock() -> &'static tokio::sync::Mutex<()>;
    fn recovery(api: &str, key: Option<&str>) -> impl Future<Output = Self::Fixture>;
    fn config() -> impl Future<Output = anyhow::Result<agent_config::runtime::Config>> + Send;
    fn invalidate_cache() -> impl Future<Output = ()>;
    fn catalog_router() -> Router;
    fn config_router() -> Router;
    fn chat(query: Query<InstanceQuery>, headers: HeaderMap, body: Json<GatewayChatStreamRequest>) -> impl Future<Output = Result<Response, ApiError>>;
}
