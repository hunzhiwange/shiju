//! Only host-issued image references may reach the bounded execution-host reader.
use axum::{Extension, Json, http::{HeaderMap, StatusCode, header}, response::{IntoResponse, Response}};
use vw_agent_apps::sessions::{cache, images};
use vw_api_types::{TenantScope, external_session::*};
use super::messages::{Failure, failure};

pub(super) async fn read(
    Extension(policy): Extension<super::catalog::Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(request): Json<SessionImageRequest>,
) -> Result<Response, Failure> {
    tokio::task::spawn_blocking(move || execute(policy, tenant, headers, request))
        .await.map_err(|_| failure(StatusCode::SERVICE_UNAVAILABLE, ExternalSessionErrorCode::StorageUnavailable))?
}

fn execute(
    policy: super::catalog::Policy,
    tenant: TenantScope,
    headers: HeaderMap,
    request: SessionImageRequest,
) -> Result<Response, Failure> {
    let (_, transcript, _, source, fingerprint) = super::messages::resolve_source(
        policy, tenant, headers, SessionMessagesRequest { identity: request.identity }, None,
    )?;
    // Never pass a request's locator to the filesystem, even within the authorized root.
    let image = transcript.messages().iter().flat_map(|message| &message.blocks)
        .find_map(|block| match block {
            SessionBlock::Image { image } if image == &request.image => Some(image),
            SessionBlock::ToolResult { images, .. } => images.iter().find(|image| *image == &request.image),
            _ => None,
        })
        .ok_or_else(|| failure(StatusCode::FORBIDDEN, ExternalSessionErrorCode::Forbidden))?;
    let bytes = images::load_image(&source, image).map_err(image_failure)?;
    let after = cache::source_fingerprint(&source)
        .map_err(|_| failure(StatusCode::CONFLICT, ExternalSessionErrorCode::SourceChanged))?;
    if fingerprint != after {
        return Err(failure(StatusCode::CONFLICT, ExternalSessionErrorCode::SourceChanged));
    }
    let media_type = match image.media_type.as_str() {
        "image/jpg" => "image/jpeg",
        "image/x-ms-bmp" => "image/bmp",
        mime => mime,
    };
    Ok(([
        (header::CONTENT_TYPE, media_type),
        (header::CACHE_CONTROL, "no-store"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
    ], bytes).into_response())
}

fn image_failure(error: String) -> Failure {
    // Do not publish filesystem errors, SQL or source paths.
    let (status, code, message) = match error.as_str() {
        "图片超过20MiB" | "内容超过32MiB" => (StatusCode::PAYLOAD_TOO_LARGE, ExternalSessionErrorCode::Conflict, "Image exceeds 20 MiB limit"),
        "不支持的图片格式（包括SVG）" => (StatusCode::UNSUPPORTED_MEDIA_TYPE, ExternalSessionErrorCode::Unsupported, "Unsupported image format (including SVG)"),
        "图片不属于当前会话授权目录" | "引用不属于当前会话" => (StatusCode::FORBIDDEN, ExternalSessionErrorCode::Forbidden, "Image is outside the authorized session root"),
        "图片数据格式无效" => (StatusCode::UNSUPPORTED_MEDIA_TYPE, ExternalSessionErrorCode::Unsupported, "Invalid bitmap image data"),
        "无法读取图片" | "无法打开会话数据库" => (StatusCode::SERVICE_UNAVAILABLE, ExternalSessionErrorCode::StorageUnavailable, "Image storage is unavailable"),
        _ => (StatusCode::CONFLICT, ExternalSessionErrorCode::SourceChanged, "Image source is unavailable or changed; retry the read"),
    };
    let mut result = failure(status, code);
    result.1.0.message = message.into();
    result
}
