//! Gateway dictation: credentials stay in the authenticated tenant's configuration.
use crate::host::VoiceHost;
use agent_provider::voice::transcription::{TranscriptionError, TranscriptionProvider};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use {
    vw_api_transport::id::TenantScope,
    vw_api_voice::TranscriptionRequest,
    vw_api_voice::TranscriptionResponse,
    vw_api_voice::VoiceError,
    vw_api_voice::VoiceErrorCode as Code,
    vw_api_voice::VoiceErrorResponse,
    vw_api_voice::VoiceId,
};

use crate::voice_requests as requests;
use crate::voice_upload as upload;
use requests::{Outcome, Owner, Requests, Start};
use upload::{MAX_BODY, read_upload};

/// One shared registry per running router, including cancellation and bounded deduplication.
pub fn router<S: Clone + Send + Sync + 'static, H: VoiceHost>() -> Router<S> {
    Router::new()
        .merge(super::voice_realtime::router::<S, H>())
        .route(
            "/voice/config",
            get(voice_config)
                .patch(super::voice_settings::save)
                .layer(DefaultBodyLimit::max(64 * 1024)),
        )
        .route(
            "/voice/preview",
            post(super::voice_settings::preview::<H>).layer(DefaultBodyLimit::max(1024)),
        )
        .layer(Extension(Arc::new(tokio::sync::Semaphore::new(4))))
        .route("/voice/transcriptions", post(transcribe::<H>))
        .route("/voice/transcriptions/{request_id}", delete(cancel))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(Extension(Arc::new(Requests::default())))
}
pub(super) fn fallback_id() -> VoiceId {
    "invalid-request".to_owned().try_into().expect("static ID")
}
pub(super) fn failure(code: Code, id: VoiceId) -> Response {
    (
        StatusCode::from_u16(code.http_status()).expect("voice status"),
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        Json(VoiceErrorResponse { error: VoiceError::new(code, id) }),
    )
        .into_response()
}
async fn authenticate(
    tenant: &TenantScope,
    headers: &axum::http::HeaderMap,
) -> Result<(Owner, config_voice::VoiceConfig), Code> {
    if headers.get_all(axum::http::header::AUTHORIZATION).iter().count() != 1 {
        return Err(Code::Unauthorized);
    }
    let token = extract_auth_skey(headers).ok_or(Code::Unauthorized)?;
    let config = agent_config::runtime::get_global_for_tenant(tenant)
        .await
        .map_err(|_| Code::ProviderUnavailable)?;
    let guard = gateway_pairing::pairing::PairingGuard::from_skeys(
        config.gateway.auth_enabled,
        &config.gateway.skeys,
    );
    if !guard.auth_enabled() || !guard.is_authenticated(token) {
        return Err(Code::Unauthorized);
    }
    Ok((
        Owner { tenant: tenant.clone(), principal: Sha256::digest(token.as_bytes()).into() },
        config.app_ui.voice,
    ))
}
pub(super) async fn realtime_identity(
    tenant: &TenantScope,
    headers: &axum::http::HeaderMap,
) -> Result<([u8; 32], config_voice::VoiceConfig), Code> {
    authenticate(tenant, headers).await.map(|(owner, config)| (owner.principal, config))
}
async fn voice_config(
    Extension(tenant): Extension<TenantScope>,
    headers: axum::http::HeaderMap,
) -> Response {
    match authenticate(&tenant, &headers).await {
        Ok((_, config)) => {
            ([(axum::http::header::CACHE_CONTROL, "no-store")], Json(config)).into_response()
        }
        Err(code) => failure(code, fallback_id()),
    }
}
async fn transcribe<H: VoiceHost>(
    Extension(tenant): Extension<TenantScope>,
    Extension(requests): Extension<Arc<Requests>>,
    request: Request,
) -> Response {
    let (owner, config) = match authenticate(&tenant, request.headers()).await {
        Ok(v) => v,
        Err(code) => return failure(code, fallback_id()),
    };
    let _upload_slot = match requests.upload_slot() {
        Ok(slot) => slot,
        Err(code) => return failure(code, fallback_id()),
    };
    let (metadata, audio) =
        match tokio::time::timeout(Duration::from_secs(30), read_upload(request)).await {
            Ok(Ok(v)) => v,
            Ok(Err(code)) => return failure(code, fallback_id()),
            Err(_) => return failure(Code::Timeout, fallback_id()),
        };
    let id = metadata.request_id.clone();
    let outcome = H::with_tenant_scope(
        tenant,
        execute_transcription::<H>(owner, config, requests, metadata, audio),
    )
    .await;
    match outcome {
        Ok(response) => {
            ([(axum::http::header::CACHE_CONTROL, "no-store")], Json(response)).into_response()
        }
        Err(code) => failure(code, id),
    }
}
async fn execute_transcription<H: VoiceHost>(
    owner: Owner,
    config: config_voice::VoiceConfig,
    requests: Arc<Requests>,
    metadata: TranscriptionRequest,
    audio: Vec<u8>,
) -> Outcome {
    // No user-supplied tenant, path, URL or key participates in ownership lookup.
    let session_tenant = owner.tenant.clone();
    let session_id = metadata.chat_session_id.as_ref().to_owned();
    let session = H::load_session(session_tenant, session_id)
        .await.map_err(|_| Code::ProviderUnavailable)?;
    session.ok_or(Code::NotFound)?;
    if config.provider_id.as_deref() != Some(metadata.provider_id.as_ref()) {
        return Err(Code::Forbidden);
    }
    let mut hash = Sha256::new();
    hash.update(serde_json::to_vec(&metadata).map_err(|_| Code::InvalidRequest)?);
    hash.update(&audio);
    let start = requests.start(owner, metadata.request_id.as_ref(), hash.finalize().into())?;
    match start {
        Start::Wait(mut receiver) => loop {
            if let Some(outcome) = receiver.borrow_and_update().clone() {
                return outcome;
            }
            receiver.changed().await.map_err(|_| Code::Cancelled)?;
        },
        Start::Run(pending) => {
            let result = async {
                let provider =
                    TranscriptionProvider::from_config(&config).await.map_err(service_error)?;
                let transcript = provider
                    .transcribe(audio, "audio/wav", &config, &pending.cancel)
                    .await
                    .map_err(service_error)?;
                Ok(TranscriptionResponse {
                    request_id: metadata.request_id,
                    chat_session_id: metadata.chat_session_id,
                    generation: metadata.generation,
                    text: transcript.text,
                    detected_languages: transcript.detected_languages,
                })
            };
            let outcome = tokio::select! { biased; _=pending.cancel.cancelled()=>Err(Code::Cancelled), outcome=result=>outcome };
            pending.finish(outcome.clone());
            outcome
        }
    }
}
fn service_error(error: TranscriptionError) -> Code {
    match error {
        TranscriptionError::Service(code) => code,
        TranscriptionError::EmptyTranscript => Code::InvalidRequest,
    }
}
async fn cancel(
    Extension(tenant): Extension<TenantScope>,
    Extension(requests): Extension<Arc<Requests>>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
) -> Response {
    let owner = match authenticate(&tenant, &headers).await {
        Ok((owner, _)) => owner,
        Err(code) => return failure(code, fallback_id()),
    };
    if VoiceId::try_from(id.clone()).is_err() {
        return failure(Code::InvalidRequest, fallback_id());
    }
    requests.cancel(owner, id);
    StatusCode::NO_CONTENT.into_response()
}

fn extract_auth_skey(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers.get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok()).map(str::trim)
        .and_then(|v| v.strip_prefix("Bearer ")).map(str::trim).filter(|v| !v.is_empty())
}
