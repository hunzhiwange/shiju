//! Authorized saved-record re-publication through the original native journal.
use crate::{
    app_providers::AppProviderStore,
    app_routing::{
        RouteCommitError, RoutingListener, SharedTransport, VerifiedRouteBinding,
        aggregation_routing::{AggregationModel, AggregationPublication, AggregationTarget},
        dispatch::JsonTarget,
    },
    global_database::GlobalDatabase,
    model_relay::generation::{GenerationProtocol, GenerationTransport},
};
use axum::http::{StatusCode, header};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{ffi::OsStr, path::PathBuf, sync::Arc};
use vw_agent_apps::{
    aggregation::{
        claude_catalog::{ClaudeCatalogMember, ClaudeModelCatalog},
        claude_publish,
        codex_catalog::*,
        codex_publish,
        preview::{Catalog, PreviewInput, Provider},
    },
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
use vw_api_types::{agent_apps::AppId, id::ProjectScope};

/// Construct only after project aggregation-write and (when attached) route grants.
/// No user-selected paths or raw provider payloads cross an HTTP boundary here.
#[derive(Clone)]
pub(crate) struct RecordPublicationContext {
    pub(crate) database: GlobalDatabase,
    pub(crate) project: ProjectScope,
    pub(crate) host: String,
    pub(crate) app: AppId,
    pub(crate) home: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) routing: Option<Arc<RoutingListener>>,
    pub(crate) expected: AppState,
    pub(crate) principal: String,
    pub(crate) revision: String,
    pub(crate) failure: Arc<std::sync::Mutex<Option<StatusCode>>>,
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
    pub(super) fn validate_inputs(
        &self,
    ) -> Result<(), vw_agent_apps::providers::live::write_guard::WriteError> {
        use vw_agent_apps::providers::live::write_guard::WriteError;
        let tx =
            ProviderTransactions::open_existing(&self.root, ExecutionHostId::new(&self.host)?)?
                .ok_or(WriteError::Conflict)?;
        if tx.state(self.app)? != self.expected {
            return Err(WriteError::Conflict);
        }
        let snapshot = crate::gateway::api::handlers::aggregation_read_snapshot::load(
            &self.database,
            &self.project,
            &self.host,
            self.app,
            &self.home,
            &self.expected,
            None,
        )
        .map_err(|_| WriteError::Conflict)?;
        let revision = crate::gateway::api::handlers::aggregation_read_snapshot::revision(
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
    pub(crate) fn replace(&self, key: &str, before: &str, after: &str) -> Result<bool, StatusCode> {
        self.replace_candidate(key, before, after, None, codex_publish::CatalogPolicy::Preserve)
    }
    /// The membership/default API uses this same planner rather than first saving
    /// a candidate state. The unchanged record participates in crash recovery.
    pub(crate) fn publish_candidate(&self, candidate: &AppState) -> Result<bool, StatusCode> {
        if candidate == &self.expected {
            self.validate_inputs().map_err(|_| StatusCode::CONFLICT)?;
            return Ok(false);
        }
        if self.expected.stack.enabled
            && self.expected.mode.as_ref().is_some_and(|mode| mode.attached)
            && self.routing.as_ref().and_then(|routing| routing.bound_address()).is_none()
        {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        let mut allowed = self.expected.clone();
        allowed.stack = candidate.stack.clone();
        if allowed.stack.enabled != self.expected.stack.enabled {
            return Err(StatusCode::FORBIDDEN);
        }
        match (&mut allowed.mode, &candidate.mode) {
            (Some(expected), Some(next)) => expected.proxy_route = next.proxy_route.clone(),
            (None, Some(next))
                if next.mode == Mode::Direct && !next.attached && next.contract.is_none() =>
            {
                allowed.mode = Some(next.clone())
            }
            (None, None) => {}
            _ => return Err(StatusCode::FORBIDDEN),
        }
        if &allowed != candidate {
            return Err(StatusCode::FORBIDDEN);
        }
        let id = candidate
            .mode
            .as_ref()
            .and_then(|m| m.proxy_route.as_deref())
            .or_else(|| candidate.stack.members.first().map(String::as_str))
            .ok_or(StatusCode::CONFLICT)?;
        let key = format!("{}:{id}", serde_json::to_string(&self.app).map_err(failed)?);
        let row = AppProviderStore::new(self.database.clone())
            .map_err(failed)?
            .publication_record(self.project.tenant_scope(), self.app, id)
            .map_err(failed)?;
        self.replace_candidate(
            &key,
            &row,
            &row,
            Some(candidate),
            codex_publish::CatalogPolicy::Preserve,
        )
    }
    fn replace_candidate(
        &self,
        key: &str,
        before: &str,
        after: &str,
        candidate_override: Option<&AppState>,
        catalog_policy: codex_publish::CatalogPolicy,
    ) -> Result<bool, StatusCode> {
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
        if state != self.expected || tx.pending(self.app).map_err(failed)?.is_some() {
            return Err(StatusCode::CONFLICT);
        }
        let before_value: Value = serde_json::from_str(before).map_err(failed)?;
        if AppProviderStore::preview_record(
            &self.database,
            self.project.tenant_scope(),
            self.app,
            id,
        )
        .map_err(failed)?
            != before_value
        {
            return Ok(false);
        }
        if before == after
            && candidate_override.is_none()
            && self.routing.as_ref().is_some_and(|routing| {
                routing.aggregation_revision(&self.project, &self.host, self.app).is_some()
            })
        {
            self.validate_inputs().map_err(|_| StatusCode::CONFLICT)?;
            return Ok(true);
        }
        let mut change = ProviderRecordChange {
            operation_id: uuid::Uuid::new_v4().to_string(),
            host: ExecutionHostId::new(&self.host).map_err(failed)?,
            project: self.project.clone(),
            app: self.app,
            provider_id: id.into(),
            before: before.into(),
            after: after.into(),
            dependencies: vec![],
        };
        let mut candidate = candidate_override.unwrap_or(&state).clone();
        let mut rows = Vec::new();
        for member in &candidate.stack.members {
            let raw = if member == id {
                serde_json::from_str(after).map_err(failed)?
            } else {
                AppProviderStore::preview_record(
                    &self.database,
                    self.project.tenant_scope(),
                    self.app,
                    member,
                )
                .map_err(failed)?
            };
            if raw["category"] == "official"
                || raw.get("managedCodexLogin").is_some()
                || raw.get("managedClaudeLogin").is_some()
                || raw.pointer("/meta/authBinding").is_some()
            {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            rows.push((member.clone(), raw));
        }
        // Stable sort only the existing membership. Missing/equal indices retain order.
        if candidate_override.is_none() {
            candidate.stack.members.sort_by_key(|member| {
                rows.iter()
                    .find(|(id, _)| id == member)
                    .and_then(|(_, row)| row.get("sortIndex"))
                    .and_then(Value::as_i64)
                    .map_or((1, 0), |i| (0, i))
            });
        }
        rows.sort_by_key(|(id, _)| {
            candidate.stack.members.iter().position(|m| m == id).unwrap_or(usize::MAX)
        });
        let binding = serde_json::json!({"host":change.host,"project":self.project,"app":self.app});
        let bind = |raw: &str| -> Result<String, StatusCode> {
            let mut row: Value = serde_json::from_str(raw).map_err(failed)?;
            if row.get("shijuPublicationBinding").is_some_and(|owner| owner != &binding) {
                return Err(StatusCode::FORBIDDEN);
            }
            row.as_object_mut()
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
                .insert("shijuPublicationBinding".into(), binding.clone());
            Ok(row.to_string())
        };
        let unbind = |raw: &str| -> Result<String, StatusCode> {
            let mut row: Value = serde_json::from_str(raw).map_err(failed)?;
            if row.get("shijuPublicationBinding").is_some_and(|owner| owner != &binding) {
                return Err(StatusCode::FORBIDDEN);
            }
            row.as_object_mut()
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
                .remove("shijuPublicationBinding");
            Ok(row.to_string())
        };
        change.after = if candidate.stack.members.iter().any(|member| member == id) {
            bind(after)?
        } else {
            unbind(after)?
        };
        let mut source_ids = candidate.stack.members.clone();
        for member in &state.stack.members {
            if !source_ids.contains(member) {
                source_ids.push(member.clone());
            }
        }
        for member in source_ids {
            if member == id {
                continue;
            }
            let raw = store
                .publication_record(self.project.tenant_scope(), self.app, &member)
                .map_err(failed)?;
            let after =
                if candidate.stack.members.contains(&member) { bind(&raw)? } else { unbind(&raw)? };
            change.dependencies.push(
                vw_agent_apps::providers::live::record_publication::ProviderRecordDelta {
                    provider_id: member,
                    before: raw,
                    after,
                },
            );
        }
        let target = PendingTarget {
            stack: Some(candidate.stack.clone()),
            mode: candidate_override.and_then(|_| candidate.mode.clone()),
            ..Default::default()
        };
        let active = candidate.stack.enabled
            && candidate.mode.as_ref().is_some_and(|m| m.attached && m.mode == Mode::Proxy);
        let ordinary = !candidate.stack.enabled
            && candidate
                .mode
                .as_ref()
                .is_some_and(|mode| mode.attached && mode.mode == Mode::Proxy);
        if ordinary
            && matches!(self.app, AppId::Claude | AppId::Codex)
            && candidate.mode.as_ref().and_then(|mode| mode.proxy_route.as_deref()) == Some(id)
        {
            return self.publish_route(&tx, &store, &state, &candidate, &rows, &change, target);
        }
        if matches!(self.app, AppId::Claude | AppId::Codex)
            && (!active || !candidate.stack.members.iter().any(|member| member == id))
        {
            if state.mode.as_ref().is_some_and(|mode| mode.attached)
                && target.mode.as_ref().is_some_and(|next| {
                    state.mode.as_ref().and_then(|old| old.proxy_route.as_ref())
                        != next.proxy_route.as_ref()
                })
            {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            // A planned catalog does not provision or touch native client files.
            let mut guard =
                vw_agent_apps::providers::live::write_guard::lock_app(self.app, &self.home)
                    .map_err(failed)?;
            tx.commit_project_record(
                &self.project,
                &mut guard,
                &state,
                &[],
                target,
                &change,
                &store,
                || self.validate_inputs(),
            )
            .map_err(commit_status)?;
            return Ok(true);
        }
        match self.app {
            AppId::Claude => {
                let _directory = tx.lock_claude_directory().map_err(failed)?;
                let selected = tx.claude_directory(self.project.tenant_scope()).map_err(failed)?;
                let mut file = ClaudeFile::open_selected(&self.home, selected.as_ref(), false)
                    .map_err(failed)?;
                let providers: Vec<_> = rows
                    .iter()
                    .map(|(id, row)| Provider {
                        id,
                        name: row["name"].as_str().unwrap_or(id),
                        official_service: None,
                        catalog: Catalog::Claude(row),
                    })
                    .collect();
                let default = candidate
                    .mode
                    .as_ref()
                    .and_then(|m| m.proxy_route.as_deref())
                    .ok_or(StatusCode::CONFLICT)?;
                let members: Vec<_> = rows
                    .iter()
                    .map(|(id, row)| {
                        Ok(ClaudeCatalogMember {
                            provider_id: id,
                            key: candidate
                                .stack
                                .keys
                                .iter()
                                .find_map(|(key, owner)| (owner == id).then_some(key.as_str()))
                                .ok_or(StatusCode::CONFLICT)?,
                            record: row,
                        })
                    })
                    .collect::<Result<_, StatusCode>>()?;
                let catalog = ClaudeModelCatalog::new(&members, Some(default))
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                let models = catalog
                    .models()
                    .iter()
                    .map(|m| AggregationModel {
                        id: m.id.clone(),
                        provider_id: m.provider_id.clone(),
                        upstream_model: m.upstream_model.clone(),
                    })
                    .collect();
                let routing = self.routing.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
                let revision = hex::encode(Sha256::digest(
                    serde_json::to_vec(&(&candidate, &rows)).map_err(failed)?,
                ));
                let publication =
                    self.publication(&candidate, &rows, models, &revision, default)?;
                let previous = routing.aggregation_revision(&self.project, &self.host, self.app);
                let previous_config = state
                    .mode
                    .as_ref()
                    .and_then(|m| m.contract.as_ref())
                    .map(|c| serde_json::json!({"env":c.exclusive}));
                let result = routing.publish_aggregation(
                    publication,
                    previous.as_deref(),
                    |address, token| {
                        let target_config = claude_publish::plan(
                            self.project.tenant_scope(),
                            &candidate,
                            PreviewInput {
                                candidate_default: Some(default),
                                providers: &providers,
                                pending_tenant: None,
                                failover_enabled: false,
                                direct_restorable: true,
                                catalog_policy:
                                    vw_agent_apps::aggregation::preview::CatalogPolicy::Preserve,
                            },
                            &format!("http://{address}"),
                            token,
                        )
                        .map_err(|_| RouteCommitError {
                            status: StatusCode::UNPROCESSABLE_ENTITY,
                            recovery_pending: false,
                        })?;
                        let contract = ClientConfiguration::saved(self.app, &target_config, false)
                            .map_err(|_| RouteCommitError {
                                status: StatusCode::UNPROCESSABLE_ENTITY,
                                recovery_pending: false,
                            })?
                            .contract();
                        let patch = |pre: Option<&[u8]>| {
                            vw_agent_apps::providers::claude::patch_with_residue(
                                pre,
                                previous_config.as_ref(),
                                &target_config,
                                false,
                            )
                        };
                        let target = PendingTarget {
                            aggregation_recovery: Some(AggregationRecovery {
                                principal: self.principal.clone(),
                                published: target_config.clone(),
                                accepted_external_catalog: None,
                            }),
                            stack: Some(candidate.stack.clone()),
                            mode: Some(ModeState {
                                mode: Mode::Proxy,
                                attached: true,
                                proxy_route: Some(default.into()),
                                contract: Some(contract),
                            }),
                            ..Default::default()
                        };
                        tx.commit_project_record(
                            &self.project,
                            &mut file.guard,
                            &state,
                            &[FileChange {
                                name: OsStr::new(file.name),
                                privacy: FilePrivacy::Private,
                                patch: &patch,
                            }],
                            target,
                            &change,
                            &store,
                            || self.validate_inputs(),
                        )
                        .map_err(|error| RouteCommitError {
                            status: commit_status(error),
                            recovery_pending: tx
                                .pending(self.app)
                                .map_or(true, |p| p.is_some_and(|p| p.published)),
                        })
                    },
                );
                result.map(|_| true)
            }
            AppId::Codex => {
                self.codex(&tx, &store, &state, &candidate, &rows, &change, target, catalog_policy)
            }
            AppId::ClaudeDesktop => super::aggregation_desktop_record::replace(
                self,
                &tx,
                &store,
                &state,
                &candidate,
                &rows,
                &change,
                target,
                active || ordinary,
            ),
            _ => Err(StatusCode::UNPROCESSABLE_ENTITY),
        }
    }
    fn publication(
        &self,
        state: &AppState,
        rows: &[(String, Value)],
        models: Vec<AggregationModel>,
        revision: &str,
        default: &str,
    ) -> Result<AggregationPublication, StatusCode> {
        let binding = VerifiedRouteBinding::new(
            self.project.clone(),
            self.host.clone(),
            self.app,
            "saved-aggregation".into(),
            1,
        );
        let target = |id: &str, row: &Value| {
            let profile = crate::app_providers::generation_target::SavedProfile::read_for_host(
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
            Ok(AggregationTarget {
                messages_api_key: self.app == AppId::Claude && !profile.messages_bearer,
                binding: binding.clone(),
                revision: revision.into(),
                provider_id: id.into(),
                target: JsonTarget {
                    #[cfg(target_os = "macos")]
                    managed: None,
                    url,
                    authorization,
                    transport: SharedTransport::new().map_err(failed)?,
                    request_options,
                },
            })
        };
        let default_row = rows.iter().find(|(id, _)| id == default).ok_or(StatusCode::CONFLICT)?;
        Ok(AggregationPublication {
            binding: binding.clone(),
            revision: revision.into(),
            state: state.clone(),
            models,
            default: target(default, &default_row.1)?,
            members: rows
                .iter()
                .filter(|(id, _)| id != default)
                .map(|(id, row)| target(id, row))
                .collect::<Result<_, _>>()?,
        })
    }
}

mod codex;
mod route;

mod publish;

mod exit;
