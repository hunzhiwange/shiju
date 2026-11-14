//! Lazy text reads accept only references issued by the authorized transcript.
use axum::{Extension, Json, http::{HeaderMap, StatusCode}};
use vw_agent_apps::sessions::{cache, content_file, content_sqlite};
use vw_api_types::{TenantScope, external_session::*};
use super::messages::{Failure, failure};

pub(super) async fn read(
    Extension(policy): Extension<super::catalog::Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(request): Json<SessionContentRequest>,
) -> Result<Json<SessionContentResponse>, Failure> {
    tokio::task::spawn_blocking(move || execute(policy, tenant, headers, request))
        .await.map_err(|_| failure(StatusCode::SERVICE_UNAVAILABLE, ExternalSessionErrorCode::StorageUnavailable))?
}

fn execute(
    policy: super::catalog::Policy,
    tenant: TenantScope,
    headers: HeaderMap,
    request: SessionContentRequest,
) -> Result<Json<SessionContentResponse>, Failure> {
    let (_, transcript, _, source, fingerprint) = super::messages::resolve_source(
        policy, tenant, headers, SessionMessagesRequest { identity: request.identity }, None,
    )?;
    // Never forward a client-supplied locator to a reader, even under an allowed root.
    let reference = transcript.messages().iter().flat_map(|message| &message.blocks)
        .find_map(|block| text_reference(block, &request.content_ref))
        .ok_or_else(|| failure(StatusCode::FORBIDDEN, ExternalSessionErrorCode::Forbidden))?;
    let text = match reference {
        ContentRef::Sqlite { .. } => content_sqlite::resolve_content_ref(&source, reference),
        _ => content_file::resolve_content_ref(&source, reference),
    }.map_err(content_failure)?;
    let after = cache::source_fingerprint(&source)
        .map_err(|_| failure(StatusCode::CONFLICT, ExternalSessionErrorCode::SourceChanged))?;
    if fingerprint != after {
        return Err(failure(StatusCode::CONFLICT, ExternalSessionErrorCode::SourceChanged));
    }
    let page = content_file::paginate(&text, request.offset, request.limit)
        .map_err(|_| failure(StatusCode::BAD_REQUEST, ExternalSessionErrorCode::InvalidInput))?;
    Ok(Json(SessionContentResponse {
        text: page.text, total_len: page.total_len, truncated: page.truncated,
        next_offset: page.next_offset,
    }))
}

fn text_reference<'a>(block: &'a SessionBlock, wanted: &ContentRef) -> Option<&'a ContentRef> {
    let (primary, diff) = match block {
        SessionBlock::Text { full, .. } | SessionBlock::ToolResult { full, .. }
        | SessionBlock::Event { full, .. } => (full.as_ref(), None),
        SessionBlock::Thinking { full, redacted: false, .. } => (full.as_ref(), None),
        SessionBlock::ToolCall { input_full, diff, .. } =>
            (input_full.as_ref(), diff.as_ref().and_then(|diff| diff.full.as_ref())),
        _ => (None, None),
    };
    primary.into_iter().chain(diff).find(|reference| *reference == wanted)
}

fn content_failure(error: String) -> Failure {
    // Domain diagnostics are private; expose stable codes and safe messages only.
    let (status, code) = match error.as_str() {
        "不支持的内容引用" | "引用的内容不是文本" =>
            (StatusCode::NOT_IMPLEMENTED, ExternalSessionErrorCode::Unsupported),
        "引用不属于当前会话" =>
            (StatusCode::FORBIDDEN, ExternalSessionErrorCode::Forbidden),
        "内容超过32MiB" =>
            (StatusCode::PAYLOAD_TOO_LARGE, ExternalSessionErrorCode::Conflict),
        "无法打开会话数据库" =>
            (StatusCode::SERVICE_UNAVAILABLE, ExternalSessionErrorCode::StorageUnavailable),
        _ => (StatusCode::CONFLICT, ExternalSessionErrorCode::SourceChanged),
    };
    failure(status, code)
}
