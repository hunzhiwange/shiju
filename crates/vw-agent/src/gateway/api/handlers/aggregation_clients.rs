//! Read-only Codex process hints and revision-bound acknowledgement.
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Query},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use std::path::PathBuf;
use vw_api_types::{
    agent_apps::{AppId, aggregation::*},
    id::TenantScope,
};
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/{app}/aggregation/codex/clients", get(read))
        .route("/agent-apps/{app}/aggregation/codex/ack", post(ack))
        .layer(Extension(Policy(path)))
        .layer(DefaultBodyLimit::max(4096))
}
type Failure = (StatusCode, &'static str);
fn respond(result: Result<Result<CodexClientsState, Failure>, tokio::task::JoinError>) -> Response {
    use vw_api_types::error::{ApiErrorBody, ApiErrorDetail};
    let result = result.unwrap_or(Err((StatusCode::BAD_GATEWAY, "target_failed")));
    let mut response = match result {
        Ok(state) => Json(state).into_response(),
        Err((status, code)) => (
            status,
            Json(ApiErrorBody {
                error: ApiErrorDetail { code: code.into(), message: code.into(), details: None },
            }),
        )
            .into_response(),
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, axum::http::HeaderValue::from_static("no-store"));
    response
}
async fn read(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    app: Result<Path<AppId>, axum::extract::rejection::PathRejection>,
    query: Result<Query<AggregationQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let (Ok(Path(app)), Ok(Query(query))) = (app, query) else {
        return respond(Ok(Err((StatusCode::BAD_REQUEST, "invalid_request"))));
    };
    respond(
        tokio::task::spawn_blocking(move || execute(policy, tenant, headers, app, query, None))
            .await,
    )
}
async fn ack(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    app: Result<Path<AppId>, axum::extract::rejection::PathRejection>,
    request: Result<Json<CodexClientsAckRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let (Ok(Path(app)), Ok(Json(request))) = (app, request) else {
        return respond(Ok(Err((StatusCode::BAD_REQUEST, "invalid_request"))));
    };
    let query =
        AggregationQuery { target_host_id: request.target_host_id, project_id: request.project_id };
    respond(
        tokio::task::spawn_blocking(move || {
            execute(policy, tenant, headers, app, query, Some(request.expected_revision))
        })
        .await,
    )
}
fn failure(status: StatusCode) -> Failure {
    (
        status,
        match status {
            StatusCode::BAD_REQUEST => "invalid_request",
            StatusCode::UNAUTHORIZED => "unauthenticated",
            StatusCode::FORBIDDEN => "forbidden",
            StatusCode::NOT_FOUND => "not_found",
            StatusCode::CONFLICT => "revision_conflict",
            StatusCode::UNPROCESSABLE_ENTITY => "unsupported_target",
            _ => "target_unavailable",
        },
    )
}
#[cfg(not(target_os = "macos"))]
fn execute(
    _: Policy,
    _: TenantScope,
    _: HeaderMap,
    _: AppId,
    _: AggregationQuery,
    _: Option<String>,
) -> Result<CodexClientsState, Failure> {
    Err((StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}
#[cfg(target_os = "macos")]
fn native_error(error: vw_agent_apps::providers::live::write_guard::WriteError) -> Failure {
    use vw_agent_apps::providers::live::write_guard::WriteError;
    match error {
        WriteError::Conflict => failure(StatusCode::CONFLICT),
        WriteError::Recovered => (StatusCode::CONFLICT, "recovery_pending"),
        WriteError::InvalidResource => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"),
        WriteError::Io => (StatusCode::BAD_GATEWAY, "target_failed"),
    }
}
#[cfg(target_os = "macos")]
fn execute(
    policy: Policy,
    tenant: TenantScope,
    headers: HeaderMap,
    app: AppId,
    query: AggregationQuery,
    expected: Option<String>,
) -> Result<CodexClientsState, Failure> {
    use vw_agent_apps::{
        aggregation::{catalog_ack, catalog_generation, stale_clients},
        providers::{
            codex_catalog::CodexFiles,
            live::{
                commit::{ExecutionHostId, ProviderTransactions},
                write_guard::lock_app,
            },
        },
    };
    let project = vw_api_types::id::ProjectScope::new(tenant.clone(), query.project_id)
        .map_err(|_| failure(StatusCode::BAD_REQUEST))?;
    vw_agent_apps::providers::validate_identity(&query.target_host_id)
        .map_err(|_| failure(StatusCode::BAD_REQUEST))?;
    let (home, root, _) = if expected.is_some() {
        // ACK changes only the reminder record, never routing or published configuration.
        super::agent_apps::authorize_aggregation_write(
            policy.0.as_deref(),
            &headers,
            &project,
            &query.target_host_id,
            app,
            false,
        )
    } else {
        super::agent_apps::authorize_aggregation_read(
            policy.0.as_deref(),
            &headers,
            &project,
            &query.target_host_id,
            app,
        )
    }
    .map_err(failure)?;
    if app != AppId::Codex {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "unsupported_app"));
    }
    if expected.as_ref().is_some_and(|r| r.trim().is_empty()) {
        return Err(failure(StatusCode::BAD_REQUEST));
    }
    let root = root.ok_or(failure(StatusCode::SERVICE_UNAVAILABLE))?;
    let tx = ProviderTransactions::open_existing(
        &root,
        ExecutionHostId::new(&query.target_host_id)
            .map_err(|_| failure(StatusCode::BAD_REQUEST))?,
    )
    .map_err(native_error)?
    .ok_or(failure(StatusCode::CONFLICT))?;
    // Do not use CodexFiles::open: GET must not create the native directory.
    let mut files =
        CodexFiles { guard: lock_app(app, &home.join(".codex")).map_err(native_error)? };
    let state = tx.state(app).map_err(native_error)?;
    if state.project_scope.as_ref() != Some(&project) {
        return Err(failure(if state.project_scope.is_some() {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::CONFLICT
        }));
    }
    if let Some(pending) = tx.pending(app).map_err(native_error)? {
        if pending.tenant != tenant
            || pending.target.project_scope.as_ref().is_some_and(|p| p != &project)
        {
            return Err(failure(StatusCode::FORBIDDEN));
        }
        return Err((StatusCode::CONFLICT, "recovery_pending"));
    }
    let snapshot = catalog_generation::read(&files, &tx, &project).map_err(native_error)?;
    if expected.as_ref().is_some_and(|r| r != &snapshot.revision()) {
        return Err(failure(StatusCode::CONFLICT));
    }
    let identity = daemon_identity(&home);
    let plugins = home.join(".codex/plugins");
    // Exiting aggregation can leave old managed catalogs in running clients.
    // Only an attached route's external catalog suppresses catalog-only checks.
    let check_catalog = !(state.stack.enabled
        && state.mode.as_ref().is_some_and(|mode| {
            mode.mode == vw_agent_apps::providers::live::commit::Mode::Proxy && mode.attached
        })
        && snapshot.catalog_is_external());
    let mut clients = tokio::runtime::Handle::current().block_on(stale_clients::read(
        &snapshot,
        check_catalog,
        stale_clients::ProcessIdentity {
            daemon_pid: identity.as_ref().ok().copied().flatten(),
            plugins_dir: &plugins,
        },
    ));
    clients.unverified |= identity.is_err();
    if let Some(expected) = expected {
        catalog_ack::acknowledge(&mut files, &tx, &project, &expected).map_err(native_error)?;
    }
    Ok(CodexClientsState {
        revision: clients.revision,
        daemon: clients.daemon,
        others: clients.others,
        auth: clients.auth,
        unverified: clients.unverified,
    })
}
/// Fixed, descriptor-relative, no-follow host observation. No request paths or raw output.
#[cfg(target_os = "macos")]
fn daemon_identity(home: &std::path::Path) -> Result<Option<u32>, ()> {
    use std::{
        fs::{File, OpenOptions},
        io::Read,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        },
    };
    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(home)
        .map_err(|_| ())?;
    for (index, name) in [c".codex", c"app-server-daemon", c"daemon.pid"].into_iter().enumerate() {
        let last = index == 2;
        // SAFETY: retained parent fd and a fixed single-component name; owns the returned fd.
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK
                    | if last { 0 } else { libc::O_DIRECTORY },
            )
        };
        if fd < 0 {
            return if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(())
            };
        }
        // SAFETY: fresh openat descriptor, consumed exactly once.
        let file = unsafe { File::from_raw_fd(fd) };
        if last {
            let metadata = file.metadata().map_err(|_| ())?;
            if !metadata.is_file() || metadata.len() > 4096 {
                return Err(());
            }
            let mut bytes = Vec::new();
            file.take(4097).read_to_end(&mut bytes).map_err(|_| ())?;
            if bytes.len() > 4096 {
                return Err(());
            }
            let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| ())?;
            return value
                .get("pid")
                .and_then(|p| p.as_u64())
                .and_then(|p| u32::try_from(p).ok())
                .filter(|p| *p != 0)
                .map(Some)
                .ok_or(());
        }
        directory = file;
    }
    Err(())
}
