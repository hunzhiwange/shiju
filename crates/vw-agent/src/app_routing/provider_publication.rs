//! Authorized saved provider publication and its native configuration preimage.
use super::{
    RouteCommitError, RoutingListener, SharedTransport, VerifiedRouteBinding, dispatch::JsonTarget,
};
use crate::{
    app_providers::AppProviderStore,
    global_database::GlobalDatabase,
    model_relay::generation::{GenerationProtocol, GenerationTransport},
};
use axum::http::{StatusCode, header};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
};
use vw_agent_apps::providers::{
    claude::ClaudeFile,
    live::commit::{AppState, ExecutionHostId, ProviderTransactions},
    mode_apply::{ClaudeMode, RouteScope},
    mode_state::observe,
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_high_availability::FailoverQueueScope,
    app_providers::mode_apply::{ModeApplyAction, ModeApplyResult, TargetPublication},
};

/// Executor-only proof. Saved credentials are never projected in observations.
pub(crate) struct PublishedProvider {
    pub(super) database: GlobalDatabase,
    root: PathBuf,
    home: PathBuf,
    pub(super) scope: FailoverQueueScope,
    pub(crate) provider_id: String,
    pub(crate) address: std::net::SocketAddr,
    row: Value,
    pub(super) state: AppState,
    native_digest: [u8; 32],
    messages_api_key: bool,
}

impl PublishedProvider {
    pub(crate) fn matches_native(&self, state: &AppState, bytes: Option<&[u8]>) -> bool {
        let mut expected = self.state.clone();
        if state
            .stack_tenant
            .as_ref()
            .is_some_and(|tenant| tenant != self.scope.project.tenant_scope())
        {
            return false;
        }
        if !expected.stack.enabled && !state.stack.enabled {
            // Editing an inactive member plan does not change the live target.
            expected.stack = state.stack.clone();
            expected.stack_tenant = state.stack_tenant.clone();
        }
        expected == *state
            && self.native_digest == digest(bytes.unwrap_or_default())
            && AppProviderStore::preview_record(
                &self.database,
                self.scope.project.tenant_scope(),
                self.scope.app_id,
                &self.provider_id,
            )
            .ok()
            .as_ref()
                == Some(&self.row)
    }

    /// Capture the same single-target proof after the common record transaction.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn committed_record(
        database: GlobalDatabase,
        root: &Path,
        home: &Path,
        scope: FailoverQueueScope,
        provider_id: &str,
        address: std::net::SocketAddr,
        bytes: &[u8],
        messages_api_key: bool,
    ) -> Result<Self, StatusCode> {
        if !matches!(scope.app_id, AppId::Claude | AppId::Codex) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let tx = transactions(root, &scope)?;
        let state = tx.state(scope.app_id).map_err(unavailable)?;
        state.require_project(&scope.project).map_err(|_| StatusCode::FORBIDDEN)?;
        if state.stack.enabled
            || !state.mode.as_ref().is_some_and(|mode| {
                mode.attached
                    && mode.mode == vw_agent_apps::providers::live::commit::Mode::Proxy
                    && mode.proxy_route.as_deref() == Some(provider_id)
            })
            || tx.pending(scope.app_id).map_err(unavailable)?.is_some()
        {
            return Err(StatusCode::CONFLICT);
        }
        let row = AppProviderStore::preview_record(
            &database,
            scope.project.tenant_scope(),
            scope.app_id,
            provider_id,
        )
        .map_err(unavailable)?;
        Ok(Self {
            database,
            root: root.into(),
            home: home.into(),
            scope,
            provider_id: provider_id.into(),
            address,
            row,
            state,
            native_digest: digest(bytes),
            messages_api_key,
        })
    }

    pub(crate) fn matches_record(&self, payload: &str) -> bool {
        serde_json::from_str::<Value>(payload).ok().as_ref() == Some(&self.row)
    }

    pub(super) fn model(&self, model: &str) -> String {
        vw_agent_apps::routing::target_model::resolve(
            self.scope.app_id,
            self.row.get("settingsConfig").unwrap_or(&Value::Null),
            model,
        )
    }

    pub(super) fn apply_headers(&self, headers: &mut axum::http::HeaderMap) {
        if self.messages_api_key
            && let Some(key) = headers.remove(header::AUTHORIZATION)
        {
            headers.insert("x-api-key", key);
        }
        if self.scope.app_id == AppId::Claude {
            headers
                .entry("anthropic-version")
                .or_insert(axum::http::HeaderValue::from_static("2023-06-01"));
        }
    }

    pub(super) fn check_current(&self) -> Result<(), StatusCode> {
        let tx = transactions(&self.root, &self.scope)?;
        if self.scope.app_id == AppId::Codex {
            use std::ffi::OsStr;
            let directory = self.home.join(".codex");
            if !std::fs::symlink_metadata(directory).map_err(unavailable)?.file_type().is_dir() {
                return Err(StatusCode::CONFLICT);
            }
            let files = vw_agent_apps::providers::codex_catalog::CodexFiles::open(&self.home)
                .map_err(unavailable)?;
            let bytes = files.guard.read(OsStr::new("config.toml")).map_err(unavailable)?;
            let state = tx.state(self.scope.app_id).map_err(unavailable)?;
            if tx.pending(self.scope.app_id).map_err(unavailable)?.is_some()
                || !self.matches_native(&state, bytes.as_deref())
            {
                return Err(StatusCode::CONFLICT);
            }
            return Ok(());
        }
        let _directory = tx.lock_claude_directory().map_err(unavailable)?;
        let selected =
            tx.claude_directory(self.scope.project.tenant_scope()).map_err(unavailable)?;
        let file =
            ClaudeFile::open_selected(&self.home, selected.as_ref(), false).map_err(unavailable)?;
        let state = tx.state(self.scope.app_id).map_err(unavailable)?;
        if tx.pending(self.scope.app_id).map_err(unavailable)?.is_some()
            || !self.matches_native(&state, file.bytes.as_deref())
        {
            return Err(StatusCode::CONFLICT);
        }
        Ok(())
    }
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn unavailable<T>(_: T) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}

pub(crate) fn transactions(
    root: &Path,
    scope: &FailoverQueueScope,
) -> Result<ProviderTransactions, StatusCode> {
    let host = ExecutionHostId::new(&scope.target_host_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    ProviderTransactions::open_existing(root, host)
        .map_err(unavailable)?
        .ok_or(StatusCode::CONFLICT)
}

/// Caller has verified both provider editing and project routing management.
pub(crate) fn apply(
    database: GlobalDatabase,
    routing: &RoutingListener,
    home: &Path,
    root: &Path,
    scope: &FailoverQueueScope,
    action: &ModeApplyAction,
) -> Result<ModeApplyResult, StatusCode> {
    if scope.app_id != AppId::Claude {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let tx = transactions(root, scope)?;
    let _directory = tx.lock_claude_directory().map_err(unavailable)?;
    let tenant = scope.project.tenant_scope();
    let state = tx.state(scope.app_id).map_err(unavailable)?;
    observe(&state, tenant).map_err(|_| StatusCode::FORBIDDEN)?;
    if state.project_scope.as_ref().is_some_and(|project| project != &scope.project) {
        return Err(StatusCode::FORBIDDEN);
    }
    if state.stack.enabled || state.claude_managed_login.is_some() {
        return Err(StatusCode::CONFLICT);
    }
    if tx.pending(scope.app_id).map_err(unavailable)?.as_ref().is_some_and(|pending| {
        pending.tenant != *tenant || pending.target.project_scope.as_ref() != Some(&scope.project)
    }) {
        return Err(StatusCode::FORBIDDEN);
    }
    let selected = tx.claude_directory(tenant).map_err(unavailable)?;
    let mut file = ClaudeFile::open_selected(home, selected.as_ref(), true).map_err(unavailable)?;
    if tx.state(scope.app_id).map_err(unavailable)? != state {
        return Err(StatusCode::CONFLICT);
    }
    if tx.pending(scope.app_id).map_err(unavailable)?.is_some() {
        let route_scope = RouteScope { tenant, host: &scope.target_host_id, app: scope.app_id };
        let mut mode =
            ClaudeMode { file: &mut file, transactions: &tx, scope: route_scope, runtime: None };
        routing.restore_provider(scope, || {
            mode.recover_route(&scope.project).map_err(|_| RouteCommitError {
                status: StatusCode::CONFLICT,
                recovery_pending: true,
            })
        })?;
        // Recovery can move the durable mode. Replan from a fresh caller request.
        return Err(StatusCode::CONFLICT);
    }
    let direct = state.pointer.as_ref().ok_or(StatusCode::CONFLICT)?;
    let read = |id: &str| {
        AppProviderStore::preview_record(&database, tenant, scope.app_id, id).map_err(|error| {
            if error == "application provider not found" {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            }
        })
    };
    let direct_row = read(direct.id.as_str())?;
    let qualify = |id: &str, row: &Value| {
        let provider = vw_agent_apps::providers::failover::snapshot(scope.app_id, id, row, false)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        vw_agent_apps::high_availability::queue::Queue::new(scope.app_id, vec![])
            .add(id, &[provider])
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
    };
    qualify(direct.id.as_str(), &direct_row)?;
    let id = match action {
        ModeApplyAction::Proxy { provider_id } => provider_id.as_str(),
        _ => direct.id.as_str(),
    };
    let row = read(id)?;
    qualify(id, &row)?;
    let settings = row.get("settingsConfig").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let changed = Cell::new(false);
    let current = || {
        read(id).ok().as_ref() == Some(&row)
            && read(direct.id.as_str()).ok().as_ref() == Some(&direct_row)
    };
    let commit_error = |error| RouteCommitError {
        status: match error {
            vw_agent_apps::providers::mode_apply::ModeApplyError::Ownership => {
                StatusCode::FORBIDDEN
            }
            vw_agent_apps::providers::mode_apply::ModeApplyError::InvalidInput => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            _ => StatusCode::CONFLICT,
        },
        recovery_pending: tx.pending(scope.app_id).map_or(true, |pending| pending.is_some())
            || tx.state(scope.app_id).map_or(true, |current| current != state),
    };
    if matches!(action, ModeApplyAction::Proxy { .. }) {
        let profile = crate::app_providers::generation_target::SavedProfile::read_for_host(
            scope.app_id,
            &scope.target_host_id,
            &id.into(),
            &row,
        )
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        if profile.protocol != GenerationProtocol::Messages {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let url = profile.protocol.url(
            &profile.endpoint,
            "route",
            row.pointer("/meta/isFullUrl") == Some(&Value::Bool(true)),
        )?;
        crate::model_relay::upstream::validate_target(
            &url,
            &reqwest::Request::new(reqwest::Method::POST, url.clone()),
        )?;
        let headers = profile
            .protocol
            .headers(&profile.credential, profile.messages_bearer)
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        let authorization = headers
            .get(header::AUTHORIZATION)
            .or_else(|| headers.get("x-api-key"))
            .cloned()
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        let request_options = row
            .get("requestOptions")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?
            .unwrap_or_default();
        let target = JsonTarget {
            #[cfg(target_os = "macos")]
            managed: None,
            url,
            authorization,
            transport: SharedTransport::new().map_err(unavailable)?,
            request_options,
        };
        vw_agent_apps::providers::validation::validate(scope.app_id, &target.request_options)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let binding = VerifiedRouteBinding::new(
            scope.project.clone(),
            scope.target_host_id.clone(),
            scope.app_id,
            format!("saved-provider:{id}"),
            1,
        );
        routing.publish_provider(binding, target, |address, token| {
            let invalid = || RouteCommitError {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                recovery_pending: false,
            };
            let mut env = vw_agent_apps::providers::mode_state::ClientConfiguration::saved(
                scope.app_id,
                settings,
                false,
            )
            .map_err(|_| invalid())?
            .contract()
            .exclusive;
            let saved_env = settings.get("env").and_then(Value::as_object).ok_or_else(invalid)?;
            env.extend(
                saved_env
                    .iter()
                    .filter(|(key, _)| {
                        key.as_str() == "ANTHROPIC_MODEL" || key.starts_with("ANTHROPIC_DEFAULT_")
                    })
                    .map(|(key, value)| (key.clone(), value.clone())),
            );
            env.insert("ANTHROPIC_BASE_URL".into(), json!(format!("http://{address}")));
            env.insert("ANTHROPIC_AUTH_TOKEN".into(), json!(token));
            let target = json!({"env":env});
            let route_scope = RouteScope { tenant, host: &scope.target_host_id, app: scope.app_id };
            let mut mode = ClaudeMode {
                file: &mut file,
                transactions: &tx,
                scope: route_scope,
                runtime: None,
            };
            changed.set(
                mode.commit_route(&scope.project, &state, action, &target, current)
                    .map_err(commit_error)?,
            );
            Ok(PublishedProvider {
                database: database.clone(),
                root: root.into(),
                home: home.into(),
                scope: scope.clone(),
                provider_id: id.into(),
                address,
                row: row.clone(),
                state: tx.state(scope.app_id).map_err(|_| RouteCommitError {
                    status: StatusCode::SERVICE_UNAVAILABLE,
                    recovery_pending: true,
                })?,
                messages_api_key: !profile.messages_bearer,
                native_digest: digest(file.bytes.as_deref().unwrap_or_default()),
            })
        })?;
    } else {
        routing.restore_provider(scope, || {
            let route_scope = RouteScope { tenant, host: &scope.target_host_id, app: scope.app_id };
            let mut mode = ClaudeMode {
                file: &mut file,
                transactions: &tx,
                scope: route_scope,
                runtime: None,
            };
            changed.set(
                mode.commit_route(&scope.project, &state, action, settings, current)
                    .map_err(commit_error)?,
            );
            Ok(())
        })?;
    }
    Ok(ModeApplyResult {
        configuration_changed: changed.get(),
        attached: matches!(action, ModeApplyAction::Proxy { .. }),
        target_publication: TargetPublication::Published,
    })
}
