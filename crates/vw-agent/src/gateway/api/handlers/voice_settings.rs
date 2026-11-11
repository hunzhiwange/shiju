//! Voice-only configuration and isolated preview. Never returns provider credentials.
use super::voice::{failure, fallback_id, realtime_identity};
use crate::provider::voice::realtime::RealtimeSession;
use axum::{
    Extension, Json,
    body::{Body, Bytes},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures_util::stream;
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use vw_api_types::{
    id::TenantScope,
    voice::{VoiceErrorCode as Code, VoiceEvent},
};
use vw_config_types::voice::{VoiceConfig, VoiceName};

pub(super) async fn save(
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(patch): Json<serde_json::Value>,
) -> Response {
    let result = async {
        let (_, current) = realtime_identity(&tenant, &headers).await?;
        validate_patch(&current, &patch)?;
        // Existing gateway update serializes, recursively merges, validates and persists per tenant.
        crate::config::update_global_for_tenant(
            &tenant,
            serde_json::json!({"app_ui":{"voice":patch}}),
        )
        .await
        .map_err(|_| Code::InvalidRequest)?;
        Ok::<_, Code>(())
    }
    .await;
    match result {
        Ok(()) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Err(e) => failure(e, fallback_id()),
    }
}
fn validate_patch(current: &VoiceConfig, patch: &serde_json::Value) -> Result<(), Code> {
    let fields = patch.as_object().ok_or(Code::InvalidRequest)?;
    let mut value = serde_json::to_value(current).map_err(|_| Code::InvalidRequest)?;
    for (key, field) in fields {
        if !value.as_object().is_some_and(|v| v.contains_key(key)) {
            return Err(Code::InvalidRequest);
        }
        value[key] = field.clone();
    }
    let config: VoiceConfig = serde_json::from_value(value).map_err(|_| Code::InvalidRequest)?;
    config.validate().map_err(|_| Code::InvalidRequest)
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PreviewRequest {
    voice: VoiceName,
}
/// Four bounded previews per router. Dropping the body drops the upstream socket.
pub(super) async fn preview(
    Extension(tenant): Extension<TenantScope>,
    Extension(capacity): Extension<Arc<Semaphore>>,
    headers: HeaderMap,
    Json(request): Json<PreviewRequest>,
) -> Response {
    let result = async {
        let permit = capacity.try_acquire_owned().map_err(|_| Code::Backpressure)?;
        let (_, mut config) = realtime_identity(&tenant, &headers).await?;
        config.voice = request.voice;
        config.validate().map_err(|_| Code::InvalidRequest)?;
        if !config.realtime_enabled {
            return Err(Code::UnsupportedCapability);
        }
        Ok::<_, Code>((config, permit))
    }
    .await;
    let (config, permit) = match result {
        Ok(v) => v,
        Err(e) => return failure(e, fallback_id()),
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    let audio = stream::try_unfold(
        (None::<RealtimeSession>, permit, 0usize, false),
        move |(session, permit, mut samples, finished)| {
            let config = config.clone();
            let tenant = tenant.clone();
            async move {
                if finished {
                    return Ok(None);
                }
                // Connection setup belongs to the response body so disconnect/cancel also
                // drops an in-flight handshake, rather than leaving a detached handler.
                let mut session = match session {
                    Some(session) => session,
                    None => tokio::time::timeout_at(
                        deadline,
                        crate::session::ui_store::with_tenant_scope(
                            tenant,
                            RealtimeSession::preview_from_config(&config),
                        ),
                    )
                    .await
                    .map_err(|_| std::io::Error::other("voice preview timed out"))?
                    .map_err(|_| std::io::Error::other("voice preview failed"))?,
                };
                loop {
                    let frames = tokio::time::timeout_at(deadline, session.next())
                        .await
                        .map_err(|_| std::io::Error::other("voice preview timed out"))?
                        .map_err(|_| std::io::Error::other("voice preview failed"))?;
                    let mut pcm = Vec::new();
                    let mut done = false;
                    for frame in frames {
                        match frame.event {
                            VoiceEvent::AudioDelta { audio, samples_per_channel, .. } => {
                                samples += samples_per_channel;
                                if samples > 24000 * 10 {
                                    return Err(std::io::Error::other("voice preview too long"));
                                }
                                pcm.extend(
                                    STANDARD.decode(audio).map_err(|_| {
                                        std::io::Error::other("invalid preview audio")
                                    })?,
                                );
                            }
                            VoiceEvent::ResponseDone { status, .. } => {
                                if status != vw_api_types::voice::ResponseStatus::Completed {
                                    return Err(std::io::Error::other("voice preview incomplete"));
                                }
                                done = true;
                            }
                            VoiceEvent::Error(_) | VoiceEvent::SessionClosed { .. } => {
                                return Err(std::io::Error::other("voice preview stopped"));
                            }
                            _ => {}
                        }
                    }
                    if done {
                        session.stop().await;
                        if pcm.is_empty() {
                            return Ok(None);
                        }
                    }
                    if !pcm.is_empty() {
                        return Ok(Some((
                            Bytes::from(pcm),
                            (Some(session), permit, samples, done),
                        )));
                    }
                }
            }
        },
    );
    (
        [("content-type", "application/octet-stream"), ("cache-control", "no-store")],
        Body::from_stream(audio),
    )
        .into_response()
}
#[cfg(test)]
#[path = "voice_settings_tests.rs"]
mod tests;
