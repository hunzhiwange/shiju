//! Markdown export accepts only a host-discovered session and optional reader turn.
use axum::{Extension, Json, http::{HeaderMap, StatusCode}};
use vw_agent_apps::sessions::markdown::{MarkdownOptions, export_session_markdown};
use vw_api_types::{TenantScope, external_session::*};
use super::messages::{Failure, failure};

pub(super) async fn read(
    Extension(policy): Extension<super::catalog::Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(request): Json<SessionMarkdownRequest>,
) -> Result<Json<SessionMarkdownResponse>, Failure> {
    tokio::task::spawn_blocking(move || execute(policy, tenant, headers, request))
        .await.map_err(|_| failure(StatusCode::SERVICE_UNAVAILABLE, ExternalSessionErrorCode::StorageUnavailable))?
}

fn execute(
    policy: super::catalog::Policy,
    tenant: TenantScope,
    headers: HeaderMap,
    request: SessionMarkdownRequest,
) -> Result<Json<SessionMarkdownResponse>, Failure> {
    let (session, transcript, _, source, fingerprint) = super::messages::resolve_source(
        policy, tenant, headers, SessionMessagesRequest { identity: request.identity }, None,
    )?;
    if !session.capabilities.export_markdown {
        return Err(failure(StatusCode::NOT_IMPLEMENTED, ExternalSessionErrorCode::Unsupported));
    }
    let markdown = export_session_markdown(
        &source, &transcript, &fingerprint,
        session.title.as_deref().unwrap_or("会话"), request.turn_key.as_deref(),
        &MarkdownOptions { include_thinking: request.include_thinking },
    ).map_err(export_failure)?;
    Ok(Json(SessionMarkdownResponse { markdown }))
}

fn export_failure(error: String) -> Failure {
    // Keep reader diagnostics, paths and source contents private.
    let (status, code) = match error.as_str() {
        "轮次不存在" => (StatusCode::BAD_REQUEST, ExternalSessionErrorCode::InvalidInput),
        "Markdown超过32MiB" | "内容超过32MiB" =>
            (StatusCode::PAYLOAD_TOO_LARGE, ExternalSessionErrorCode::Conflict),
        "不支持的内容引用" | "引用的内容不是文本" =>
            (StatusCode::NOT_IMPLEMENTED, ExternalSessionErrorCode::Unsupported),
        "引用不属于当前会话" => (StatusCode::FORBIDDEN, ExternalSessionErrorCode::Forbidden),
        "会话已更新，请重新读取" => (StatusCode::CONFLICT, ExternalSessionErrorCode::SourceChanged),
        "工具输入不完整，无法导出" | "工具输出不完整，无法导出" =>
            (StatusCode::CONFLICT, ExternalSessionErrorCode::Conflict),
        _ => (StatusCode::SERVICE_UNAVAILABLE, ExternalSessionErrorCode::StorageUnavailable),
    };
    failure(status, code)
}
