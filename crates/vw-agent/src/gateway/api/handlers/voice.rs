//! Gateway dictation: credentials stay in the authenticated tenant's configuration.
use crate::provider::voice::transcription::{TranscriptionError, TranscriptionProvider};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, post},
};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use vw_api_types::{
    id::TenantScope,
    voice::{
        TranscriptionRequest, TranscriptionResponse, VoiceError, VoiceErrorCode as Code,
        VoiceErrorResponse, VoiceId,
    },
};

#[path = "voice_requests.rs"]
mod requests;
#[path = "voice_upload.rs"]
mod upload;
use requests::{Outcome, Owner, Requests, Start};
use upload::{MAX_BODY, read_upload};

/// One shared registry per running router, including cancellation and bounded deduplication.
pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/voice/transcriptions", post(transcribe))
        .route("/voice/transcriptions/{request_id}", delete(cancel))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(Extension(Arc::new(Requests::default())))
}
fn fallback_id() -> VoiceId {
    "invalid-request".to_owned().try_into().expect("static ID")
}
fn failure(code: Code, id: VoiceId) -> Response {
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
) -> Result<(Owner, vw_config_types::voice::VoiceConfig), Code> {
    if headers.get_all(axum::http::header::AUTHORIZATION).iter().count() != 1 {
        return Err(Code::Unauthorized);
    }
    let token = super::super::auth::extract_auth_skey(headers).ok_or(Code::Unauthorized)?;
    let config = crate::config::get_global_for_tenant(tenant)
        .await
        .map_err(|_| Code::ProviderUnavailable)?;
    let guard = crate::security::pairing::PairingGuard::from_skeys(
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
async fn transcribe(
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
    let outcome = crate::session::ui_store::with_tenant_scope(
        tenant,
        execute_transcription(owner, config, requests, metadata, audio),
    )
    .await;
    match outcome {
        Ok(response) => {
            ([(axum::http::header::CACHE_CONTROL, "no-store")], Json(response)).into_response()
        }
        Err(code) => failure(code, id),
    }
}
async fn execute_transcription(
    owner: Owner,
    config: vw_config_types::voice::VoiceConfig,
    requests: Arc<Requests>,
    metadata: TranscriptionRequest,
    audio: Vec<u8>,
) -> Outcome {
    // No user-supplied tenant, path, URL or key participates in ownership lookup.
    let session_tenant = owner.tenant.clone();
    let session_id = metadata.chat_session_id.as_ref().to_owned();
    let session = crate::global_database::run_isolated(move || {
        crate::session::ui_store::load_agent_session_any_for_tenant_fallible(
            &session_tenant,
            &session_id,
        )
    })
    .await
    .map_err(|_| Code::ProviderUnavailable)?
    .map_err(|_| Code::ProviderUnavailable)?;
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
#[cfg(test)]
#[path = "voice_tests.rs"]
mod tests;
