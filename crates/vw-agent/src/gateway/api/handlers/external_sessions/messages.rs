//! One-shot reads resolve only records freshly discovered under an authorized instance.
use axum::{
    Extension, Json,
    http::{HeaderMap, StatusCode},
};
use std::path::Path;
use vw_agent_apps::sessions::{
    cache, providers as p,
    source::{SourceRegistry, ValidatedSource},
};
use vw_api_types::{TenantScope, external_session::*};

const MAX_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;
const MAX_MESSAGES: usize = 100_000;
pub(super) type Failure = (StatusCode, Json<ExternalSessionError>);
pub(super) fn failure(status: StatusCode, code: ExternalSessionErrorCode) -> Failure {
    (
        status,
        Json(ExternalSessionError {
            code,
            message: match code {
                ExternalSessionErrorCode::Forbidden => "Session read is not authorized",
                ExternalSessionErrorCode::NotFound => "Session is no longer available",
                ExternalSessionErrorCode::Unsupported => "Session read is unsupported",
                ExternalSessionErrorCode::InvalidInput => "Invalid session read request",
                ExternalSessionErrorCode::Conflict => "Session exceeds one-shot read limits",
                _ => "Session source is unavailable or changed; retry the read",
            }
            .into(),
            correlation_id: uuid::Uuid::new_v4().to_string(),
        }),
    )
}
pub(super) async fn read(
    Extension(policy): Extension<super::catalog::Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(request): Json<SessionMessagesRequest>,
) -> Result<Json<SessionMessagesResponse>, Failure> {
    tokio::task::spawn_blocking(move || execute(policy, tenant, headers, request)).await.map_err(
        |_| failure(StatusCode::SERVICE_UNAVAILABLE, ExternalSessionErrorCode::StorageUnavailable),
    )?
}
fn execute(
    policy: super::catalog::Policy,
    tenant: TenantScope,
    headers: HeaderMap,
    request: SessionMessagesRequest,
) -> Result<Json<SessionMessagesResponse>, Failure> {
    let (session, transcript, cached) = resolve(policy, tenant, headers, request)?;
    let id = &session.identity;
    if transcript.approx_bytes() > MAX_PAYLOAD_BYTES || transcript.messages().len() > MAX_MESSAGES {
        return Err(failure(StatusCode::PAYLOAD_TOO_LARGE, ExternalSessionErrorCode::Conflict));
    }
    Ok(Json(SessionMessagesResponse {
        turns: vw_agent_apps::sessions::turns::build_turn_index(
            &vw_agent_apps::sessions::turns::project(transcript.messages(), id.provider),
        ),
        messages: transcript.messages().to_vec(),
        session,
        cached,
        payload_bytes: transcript.approx_bytes(),
        max_payload_bytes: MAX_PAYLOAD_BYTES,
        max_messages: MAX_MESSAGES,
    }))
}
pub(super) fn resolve(
    policy: super::catalog::Policy,
    tenant: TenantScope,
    headers: HeaderMap,
    request: SessionMessagesRequest,
) -> Result<(SessionMeta, std::sync::Arc<cache::Transcript>, bool), Failure> {
    resolve_with_cancel(policy, tenant, headers, request, None)
}

pub(super) fn resolve_with_cancel(
    policy: super::catalog::Policy,
    tenant: TenantScope,
    headers: HeaderMap,
    request: SessionMessagesRequest,
    operation: Option<&vw_agent_apps::apps::lifecycle::operations::Operation>,
) -> Result<(SessionMeta, std::sync::Arc<cache::Transcript>, bool), Failure> {
    resolve_source(policy, tenant, headers, request, operation)
        .map(|(session, transcript, cached, _, _)| (session, transcript, cached))
}

pub(super) fn resolve_source(
    policy: super::catalog::Policy,
    tenant: TenantScope,
    headers: HeaderMap,
    request: SessionMessagesRequest,
    operation: Option<&vw_agent_apps::apps::lifecycle::operations::Operation>,
) -> Result<(SessionMeta, std::sync::Arc<cache::Transcript>, bool, ValidatedSource, cache::Fingerprint), Failure> {
    let checkpoint = || {
        if operation.is_some_and(|op| op.is_cancelled()) {
            Err(failure(StatusCode::CONFLICT, ExternalSessionErrorCode::Cancelled))
        } else {
            Ok(())
        }
    };
    checkpoint()?;
    let id = request.identity;
    if id.scope.tenant_scope() != &tenant {
        return Err(failure(StatusCode::FORBIDDEN, ExternalSessionErrorCode::Forbidden));
    }
    vw_agent_apps::sessions::identity::validate_identity(&id)
        .map_err(|_| failure(StatusCode::BAD_REQUEST, ExternalSessionErrorCode::InvalidInput))?;
    let (_, home, instances) = super::super::agent_apps::authorize_external_history(
        policy.0.as_deref(),
        &headers,
        &id.scope,
        &id.target_host,
    )
    .map_err(|s| {
        failure(
            s,
            if s == StatusCode::FORBIDDEN {
                ExternalSessionErrorCode::Forbidden
            } else {
                ExternalSessionErrorCode::HostUnreachable
            },
        )
    })?;
    let instance = instances
        .into_iter()
        .find(|i| i.provider == id.provider && i.instance_id == id.instance_id)
        .ok_or_else(|| failure(StatusCode::FORBIDDEN, ExternalSessionErrorCode::Forbidden))?;
    checkpoint()?;
    let (sessions, _) = super::scan::scan(&instance.root, &home, &id).map_err(|_| {
        failure(StatusCode::SERVICE_UNAVAILABLE, ExternalSessionErrorCode::StorageUnavailable)
    })?;
    checkpoint()?;
    let session = sessions
        .into_iter()
        .find(|m| m.identity == id)
        .ok_or_else(|| failure(StatusCode::NOT_FOUND, ExternalSessionErrorCode::NotFound))?;
    if !session.capabilities.read {
        return Err(failure(StatusCode::NOT_IMPLEMENTED, ExternalSessionErrorCode::Unsupported));
    }
    let pi = p::pi::PiSessionContext {
        home: &home,
        configured_agent_dir: Some(&instance.root),
        env_agent_dir: None,
        env_session_dir: None,
        project_dir: None,
    };
    let omp = p::omp::OmpSessionContext {
        agent_dir: &instance.root,
        native_agent_dir: &instance.root,
        profile: None,
        xdg_data_home: None,
        env_session_dir: None,
    };
    let root = match id.provider {
        SessionProvider::Pi => match p::pi::session_roots(&pi) {
            p::pi::SessionRootResolution::Available { root, .. } => root,
            _ => {
                return Err(failure(
                    StatusCode::NOT_IMPLEMENTED,
                    ExternalSessionErrorCode::Unsupported,
                ));
            }
        },
        SessionProvider::Omp => match p::omp::session_roots(&omp) {
            p::pi::SessionRootResolution::Available { root, .. } => root,
            _ => {
                return Err(failure(
                    StatusCode::NOT_IMPLEMENTED,
                    ExternalSessionErrorCode::Unsupported,
                ));
            }
        },
        _ => instance.root.clone(),
    };
    // Only the scan's matched identity supplies this relative locator.
    let relative = match &session.identity.source {
        SessionSource::File { relative_path } => relative_path,
        SessionSource::Sqlite { database, .. } => database,
    };
    let mut registry = SourceRegistry::new(
        id.scope.clone(),
        id.target_host.clone(),
        id.instance_id.clone(),
        id.provider,
        vec![root.clone()],
    )
    .map_err(|_| failure(StatusCode::FORBIDDEN, ExternalSessionErrorCode::Forbidden))?;
    registry
        .register(&session, root.join(relative))
        .map_err(|_| failure(StatusCode::FORBIDDEN, ExternalSessionErrorCode::Forbidden))?;
    let source = registry
        .resolve(&id)
        .map_err(|_| failure(StatusCode::FORBIDDEN, ExternalSessionErrorCode::Forbidden))?;
    checkpoint()?;
    let fingerprint = cache::source_fingerprint(&source).map_err(|_| {
        failure(StatusCode::CONFLICT, ExternalSessionErrorCode::SourceChanged)
    })?;
    let (transcript, cached) = cache::global()
        .get_or_load(&source, || load(&source, &instance.root, &home))
        .map_err(|_| {
            failure(StatusCode::SERVICE_UNAVAILABLE, ExternalSessionErrorCode::StorageUnavailable)
        })?;
    checkpoint()?;
    Ok((session, transcript, cached, source, fingerprint))
}

fn load(
    source: &ValidatedSource,
    instance: &Path,
    home: &Path,
) -> Result<Vec<SessionMessage>, String> {
    let path = source.path();
    let id = &source.identity().session_id.0;
    match source.identity().provider {
        SessionProvider::Claude => p::claude::load_messages(path),
        SessionProvider::Codex => p::codex::load_messages(path),
        SessionProvider::Opencode if source.record_id().is_some() => {
            p::opencode_sqlite::load_messages(instance, id)
        }
        SessionProvider::Opencode => {
            p::opencode::load_messages(instance, id).map_err(|e| e.to_string())
        }
        SessionProvider::Hermes if source.record_id().is_some() => {
            p::hermes_sqlite::load_messages(instance, id)
        }
        SessionProvider::Hermes => p::hermes::load_messages(path),
        SessionProvider::Pi => p::pi::load_messages(
            &p::pi::PiSessionContext {
                home,
                configured_agent_dir: Some(instance),
                env_agent_dir: None,
                env_session_dir: None,
                project_dir: None,
            },
            path,
        ),
        SessionProvider::Omp => p::omp::load_messages(
            &p::omp::OmpSessionContext {
                agent_dir: instance,
                native_agent_dir: instance,
                profile: None,
                xdg_data_home: None,
                env_session_dir: None,
            },
            path,
        ),
        SessionProvider::Gemini => p::gemini::load_messages(
            instance,
            path.strip_prefix(instance).map_err(|_| "Invalid Gemini source")?,
        ),
        SessionProvider::Openclaw => p::openclaw::load_messages(instance, path),
        SessionProvider::Grokbuild => p::grokbuild::load_messages(path),
        SessionProvider::Kimi => p::kimi::load_messages(path),
        SessionProvider::Mcode => p::mcode::load_messages(instance, id),
        SessionProvider::Dsh => p::dsh::load_messages(path),
        SessionProvider::Agy => p::agy::load_messages(path),
    }
}
