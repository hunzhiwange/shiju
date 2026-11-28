//! Authorized saved-record re-publication through the original native journal.
use {
    app_provider_services::AppProviderStore,
    app_routing_services::{
        RouteCommitError, RoutingListener, SharedTransport, VerifiedRouteBinding,
        dispatch::JsonTarget,
    },
    agent_database::GlobalDatabase,
    model_protocol::generation::{GenerationProtocol, GenerationTransport},
};
use axum::http::{StatusCode, header};
use serde_json::Value;
use std::{ffi::OsStr, path::PathBuf, sync::Arc};
use vw_agent_apps::{
    providers::{
        claude::ClaudeFile,
        codex_catalog::{CATALOG_FILENAME, CodexFiles},
        live::{
            commit::*,
            record_publication::ProviderRecordChange,
            stage::{FileChange, WholeFile},
            write_guard::FilePrivacy,
        },
        mode_state::ClientConfiguration,
    },
};
use {
    vw_api_app_identity::AppId,
    vw_api_transport::id::ProjectScope,
};

/// Construct only after project aggregation-write and (when attached) route grants.
/// No user-selected paths or raw provider payloads cross an HTTP boundary here.
#[derive(Clone)]
pub struct RecordPublicationContext {
    pub database: GlobalDatabase,
    pub project: ProjectScope,
    pub host: String,
    pub app: AppId,
    pub home: PathBuf,
    pub root: PathBuf,
    pub routing: Option<Arc<RoutingListener>>,
    pub expected: AppState,
    pub principal: String,
    pub revision: String,
    pub failure: Arc<std::sync::Mutex<Option<StatusCode>>>,
}
fn failed(_: impl Sized) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
fn commit_status(error: vw_agent_apps::providers::live::write_guard::WriteError) -> StatusCode {
    use vw_agent_apps::providers::live::write_guard::WriteError;
    match error {
        WriteError::Conflict | WriteError::Recovered => StatusCode::CONFLICT,
        WriteError::InvalidResource => StatusCode::UNPROCESSABLE_ENTITY,
        WriteError::Io => StatusCode::BAD_GATEWAY,
    }
}
impl RecordPublicationContext {
    fn validate_record_inputs(
        &self,
    ) -> Result<(), vw_agent_apps::providers::live::write_guard::WriteError> {
        use vw_agent_apps::providers::live::write_guard::WriteError;
        let tx =
            ProviderTransactions::open_existing(&self.root, ExecutionHostId::new(&self.host)?)?
                .ok_or(WriteError::Conflict)?;
        if tx.state(self.app)? != self.expected {
            return Err(WriteError::Conflict);
        }
        let snapshot = gateway_aggregation::aggregation_read_snapshot::load(
            &self.database,
            &self.project,
            &self.host,
            self.app,
            &self.home,
            &self.expected,
            None,
        )
        .map_err(|_| WriteError::Conflict)?;
        let revision = gateway_aggregation::aggregation_read_snapshot::revision(
            &self.project,
            &self.host,
            self.app,
            &self.principal,
            &self.expected,
            &snapshot,
        )
        .map_err(|_| WriteError::Conflict)?;
        if revision != self.revision {
            return Err(WriteError::Conflict);
        }
        Ok(())
    }
    pub(crate) fn release_legacy_bindings(&self) -> Result<(), StatusCode> {
        if self.expected.stack.members.is_empty() {
            return Ok(());
        }
        let tx = ProviderTransactions::open_existing(
            &self.root, ExecutionHostId::new(&self.host).map_err(failed)?,
        ).map_err(failed)?.ok_or(StatusCode::CONFLICT)?;
        let guard = vw_agent_apps::providers::live::write_guard::lock_app(self.app, &self.home)
            .map_err(failed)?;
        let store = AppProviderStore::new(self.database.clone()).map_err(failed)?;
        for id in &self.expected.stack.members {
            store.release_legacy_publication(&tx, &guard, &self.project, id).map_err(commit_status)?;
        }
        Ok(())
    }

    pub fn replace(&self, key: &str, before: &str, after: &str) -> Result<bool, StatusCode> {
        let prefix = format!("{}:", serde_json::to_string(&self.app).map_err(failed)?);
        let id = key.strip_prefix(&prefix).ok_or(StatusCode::BAD_REQUEST)?;
        let store = AppProviderStore::new(self.database.clone()).map_err(failed)?;
        let tx = ProviderTransactions::open_existing(
            &self.root,
            ExecutionHostId::new(&self.host).map_err(failed)?,
        )
        .map_err(failed)?
        .ok_or(StatusCode::CONFLICT)?;
        let state = tx.state(self.app).map_err(failed)?;
        state.require_project(&self.project).map_err(|_| StatusCode::FORBIDDEN)?;
        if state != self.expected || tx.pending(self.app).map_err(failed)?.is_some() || state.stack.enabled {
            return Err(StatusCode::CONFLICT);
        }
        if AppProviderStore::preview_record(&self.database, self.project.tenant_scope(), self.app, id)
            .map_err(failed)? != serde_json::from_str::<Value>(before).map_err(failed)?
        {
            return Ok(false);
        }
        let mut row: Value = serde_json::from_str(after).map_err(failed)?;
        if let Some(binding) = row.get("shijuPublicationBinding") {
            let owner = serde_json::json!({"host":self.host,"project":self.project,"app":self.app});
            if binding != &owner {
                return Err(StatusCode::FORBIDDEN);
            }
            row.as_object_mut().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
                .remove("shijuPublicationBinding");
        }
        let change = ProviderRecordChange {
            operation_id: uuid::Uuid::new_v4().to_string(),
            host: ExecutionHostId::new(&self.host).map_err(failed)?,
            project: self.project.clone(),
            app: self.app,
            provider_id: id.into(),
            before: before.into(),
            after: row.to_string(),
            dependencies: vec![],
        };
        let target = PendingTarget::default();
        let live = state.mode.as_ref().is_some_and(|mode| {
            mode.attached && mode.mode == Mode::Proxy && mode.proxy_route.as_deref() == Some(id)
        });
        if live {
            return match self.app {
                AppId::Claude | AppId::Codex => self.publish_route(&tx, &store, &state, id, &row, &change, target),
                AppId::ClaudeDesktop => super::aggregation_desktop_record::replace(
                    self, &ResyncHost, &tx, &store, &state, &row, &change,
                ),
                _ => Err(StatusCode::UNPROCESSABLE_ENTITY),
            };
        }
        let mut guard = vw_agent_apps::providers::live::write_guard::lock_app(self.app, &self.home)
            .map_err(failed)?;
        tx.commit_project_record(
            &self.project, &mut guard, &state, &[], target, &change, &store,
            || self.validate_record_inputs(),
        ).map_err(commit_status)?;
        Ok(true)
    }

    pub fn republish_saved(&self, key: &str, current: &str) -> Result<bool, StatusCode> {
        self.replace(key, current, current)
    }

    fn route_target(&self, id: &str, row: &Value) -> Result<(JsonTarget, bool), StatusCode> {
        let profile = app_provider_services::generation_target::SavedProfile::read_for_host(
            self.app,
            &self.host,
            &id.into(),
            row,
        )
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        if !matches!(
            (self.app, profile.protocol),
            (AppId::Claude, GenerationProtocol::Messages)
                | (AppId::Codex, GenerationProtocol::Chat)
                | (AppId::Codex, GenerationProtocol::Responses)
        ) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let url = profile.protocol.url(
            &profile.endpoint,
            "route",
            row.pointer("/meta/isFullUrl") == Some(&Value::Bool(true)),
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
        Ok((JsonTarget {
            #[cfg(target_os = "macos")]
            managed: None,
            url,
            authorization,
            transport: SharedTransport::new().map_err(failed)?,
            request_options,
        }, self.app == AppId::Claude && !profile.messages_bearer))
    }
}

mod route;
mod exit;

impl app_provider_services::RecordPublication for RecordPublicationContext {
    fn tenant_scope(&self) -> &vw_api_transport::id::TenantScope {
        self.project.tenant_scope()
    }
    fn expected(&self) -> &AppState {
        &self.expected
    }
    fn replace(&self, key: &str, before: &str, after: &str) -> Result<bool, StatusCode> {
        RecordPublicationContext::replace(self, key, before, after)
    }
    fn failure(&self) -> Option<StatusCode> {
        self.failure.lock().ok().and_then(|failure| *failure)
    }
    fn record_failure(&self, status: StatusCode) {
        if let Ok(mut failure) = self.failure.lock() {
            *failure = Some(status);
        }
    }
}

/// Binds desktop publication to the same snapshot and saved-record executor.
struct ResyncHost;
impl crate::publication_context::PublicationHost for ResyncHost {
    fn revision(&self, context: &RecordPublicationContext, provider: Option<&str>) -> Result<String, StatusCode> {
        use gateway_aggregation::aggregation_read_snapshot::{load, revision};
        let snapshot = load(&context.database, &context.project, &context.host, context.app,
            &context.home, &context.expected, provider)?;
        revision(&context.project, &context.host, context.app, &context.principal, &context.expected, &snapshot)
    }
}
