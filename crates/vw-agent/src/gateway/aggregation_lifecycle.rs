//! Gateway lifecycle for saved aggregation intent. The original journal owns
//! files and recovery; this executor supplies only policy-bound, frozen inputs.
use super::{aggregation_resync::RecordPublicationContext, api::handlers};
use crate::{
    app_providers::AppProviderStore, app_routing::RoutingListener, global_database::GlobalDatabase,
};
use axum::http::StatusCode;
use std::{path::PathBuf, sync::Arc};
use vw_agent_apps::{
    aggregation::{
        exit_restore::{Restore, RestoreError},
        lifecycle_restore::{self, LifecycleError, StartupAction},
    },
    providers::{
        claude::ClaudeFile,
        codex_catalog::CodexFiles,
        codex_login::{AuthTarget, LoginInput},
        live::commit::*,
    },
};
use vw_api_types::{
    agent_apps::AppId, agent_apps_high_availability::FailoverQueueScope, id::TenantScope,
};

#[derive(Clone)]
pub(crate) struct AggregationLifecycle {
    pub(crate) database: GlobalDatabase,
    pub(crate) policy: PathBuf,
    pub(crate) tenant: TenantScope,
    pub(crate) host: String,
}

#[derive(Default)]
pub(crate) struct DetachResult {
    pub(crate) retired: Vec<FailoverQueueScope>,
    pub(crate) failure: Option<StatusCode>,
}

fn unavailable(_: impl Sized) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
fn restore_status(error: RestoreError) -> StatusCode {
    match error {
        RestoreError::Forbidden => StatusCode::FORBIDDEN,
        RestoreError::RevisionConflict | RestoreError::RecoveryPending => StatusCode::CONFLICT,
        RestoreError::UnsupportedTarget | RestoreError::InvalidConfiguration => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
        RestoreError::TargetFailed => StatusCode::BAD_GATEWAY,
    }
}
fn lifecycle_status(error: LifecycleError) -> StatusCode {
    match error {
        LifecycleError::Cancelled => StatusCode::CONFLICT,
        LifecycleError::Restore(error) => restore_status(error),
    }
}
impl AggregationLifecycle {
    fn transactions(&self) -> Result<Option<ProviderTransactions>, StatusCode> {
        let (_, root) = handlers::agent_apps::aggregation_lifecycle_host(
            &self.policy,
            &self.tenant,
            &self.host,
        )?;
        ProviderTransactions::open_existing(
            &root,
            ExecutionHostId::new(&self.host).map_err(unavailable)?,
        )
        .map_err(unavailable)
    }
    fn context(&self, app: AppId, state: AppState) -> Result<RecordPublicationContext, StatusCode> {
        let project = state.project_scope.as_ref().ok_or(StatusCode::FORBIDDEN)?;
        if project.tenant_scope() != &self.tenant {
            return Err(StatusCode::FORBIDDEN);
        }
        let principal = match app {
            AppId::Claude | AppId::Codex => Some(
                state.aggregation_recovery.as_ref().ok_or(StatusCode::CONFLICT)?.principal.as_str(),
            ),
            // Desktop deliberately retains its client projection on service exit.
            AppId::ClaudeDesktop => None,
            _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        };
        let (home, root, principal) = handlers::agent_apps::authorize_aggregation_lifecycle(
            &self.policy,
            project,
            &self.host,
            app,
            principal,
        )?;
        let snapshot = handlers::aggregation_read_snapshot::load(
            &self.database,
            project,
            &self.host,
            app,
            &home,
            &state,
            None,
        )?;
        let revision = handlers::aggregation_read_snapshot::revision(
            project, &self.host, app, &principal, &state, &snapshot,
        )?;
        Ok(RecordPublicationContext {
            database: self.database.clone(),
            project: project.clone(),
            host: self.host.clone(),
            app,
            home,
            root,
            routing: None,
            expected: state,
            principal,
            revision,
            failure: Default::default(),
        })
    }
    fn unchanged(&self, context: &RecordPublicationContext) -> bool {
        let authorization = handlers::agent_apps::authorize_aggregation_lifecycle(
            &self.policy,
            &context.project,
            &self.host,
            context.app,
            Some(&context.principal),
        );
        authorization.is_ok_and(|(home, root, principal)| {
            home == context.home && root == context.root && principal == context.principal
        }) && context.validate_inputs().is_ok()
    }
    fn catalog_policy(
        context: &RecordPublicationContext,
    ) -> Result<vw_agent_apps::aggregation::codex_publish::CatalogPolicy, StatusCode> {
        use vw_agent_apps::aggregation::codex_publish::CatalogPolicy;
        if context.app != AppId::Codex {
            return Ok(CatalogPolicy::Preserve);
        }
        let Some(accepted) = context
            .expected
            .aggregation_recovery
            .as_ref()
            .and_then(|proof| proof.accepted_external_catalog.as_deref())
        else {
            return Ok(CatalogPolicy::Preserve);
        };
        let files = CodexFiles::open(&context.home).map_err(unavailable)?;
        let before = files.guard.read(std::ffi::OsStr::new("config.toml")).map_err(unavailable)?;
        let config: toml::Value = toml::from_str(
            std::str::from_utf8(before.as_deref().unwrap_or_default()).map_err(unavailable)?,
        )
        .map_err(unavailable)?;
        // Restart reuses confirmation only for the original external pointer.
        // An off-service user edit to a different pointer must be previewed again.
        Ok(if config.get("model_catalog_json").and_then(toml::Value::as_str) == Some(accepted) {
            CatalogPolicy::UseManaged
        } else {
            CatalogPolicy::Preserve
        })
    }
    pub(crate) fn resume(&self, routing: Arc<RoutingListener>) -> Result<(), StatusCode> {
        let Some(tx) = self.transactions()? else {
            return Ok(());
        };
        let mut failure = None;
        for app in [AppId::Claude, AppId::Codex, AppId::ClaudeDesktop] {
            let result: Result<(), StatusCode> = (|| {
                let state = tx.state(app).map_err(unavailable)?;
                if !state.stack.enabled {
                    return Ok(());
                }
                let mut context = self.context(app, state)?;
                if lifecycle_restore::startup_action(&tx, &context.project, app)
                    .map_err(restore_status)?
                    == StartupAction::StayDirect
                {
                    return Ok(());
                }
                let live = if app == AppId::ClaudeDesktop {
                    let binding = crate::app_routing::VerifiedRouteBinding::new(
                        context.project.clone(),
                        self.host.clone(),
                        app,
                        "saved-aggregation".into(),
                        1,
                    );
                    routing.desktop_revision(&binding).is_some()
                } else {
                    routing.aggregation_revision(&context.project, &self.host, app).is_some()
                };
                if live && context.expected.mode.as_ref().is_some_and(|m| m.attached) {
                    return Ok(());
                }
                if app == AppId::ClaudeDesktop {
                    let (environment, _) = handlers::agent_apps::aggregation_lifecycle_host(
                        &self.policy,
                        &self.tenant,
                        &self.host,
                    )?;
                    let observation = vw_agent_apps::apps::discovery::read::read(app, &environment)
                        .map_err(unavailable)?
                        .desktop
                        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
                    if vw_agent_apps::providers::claude_desktop::publication::DesktopInstallation::from(&observation)
                        != vw_agent_apps::providers::claude_desktop::publication::DesktopInstallation::Installed
                    { return Err(StatusCode::UNPROCESSABLE_ENTITY); }
                }
                let default = context
                    .expected
                    .mode
                    .as_ref()
                    .and_then(|m| m.proxy_route.clone())
                    .ok_or(StatusCode::CONFLICT)?;
                let row =
                    AppProviderStore::preview_record(&self.database, &self.tenant, app, &default)
                        .map_err(unavailable)?;
                context.routing = Some(routing.clone());
                let policy = Self::catalog_policy(&context)?;
                context.publish_aggregation(&default, policy, &row)?;
                Ok(())
            })();
            if let Err(status) = result {
                tracing::warn!(
                    ?app,
                    status = status.as_u16(),
                    "aggregation startup recovery failed"
                );
                failure.get_or_insert(status);
            }
        }
        failure.map_or(Ok(()), Err)
    }
    pub(crate) fn detach(&self) -> DetachResult {
        let mut result = DetachResult::default();
        let tx = match self.transactions() {
            Ok(Some(tx)) => tx,
            Ok(None) => return result,
            Err(status) => {
                tracing::warn!(status = status.as_u16(), "aggregation shutdown policy unavailable");
                result.failure.get_or_insert(status);
                return result;
            }
        };
        for app in [AppId::Claude, AppId::Codex, AppId::ClaudeDesktop] {
            let mut authorized_scope = None;
            let attempt: Result<Option<FailoverQueueScope>, StatusCode> = (|| {
                let state = tx.state(app).map_err(unavailable)?;
                if !state.stack.enabled {
                    return Ok(None);
                }
                let context = self.context(app, state)?;
                let scope = FailoverQueueScope {
                    project: context.project.clone(),
                    target_host_id: self.host.clone(),
                    app_id: app,
                };
                authorized_scope = Some(scope.clone());
                lifecycle_restore::startup_action(&tx, &context.project, app)
                    .map_err(restore_status)?;
                if app == AppId::ClaudeDesktop
                    || !context.expected.mode.as_ref().is_some_and(|m| m.attached)
                {
                    return Ok(Some(scope));
                }
                let direct_id =
                    context.expected.pointer.as_ref().ok_or(StatusCode::CONFLICT)?.id.as_str();
                let direct =
                    AppProviderStore::preview_record(&self.database, &self.tenant, app, direct_id)
                        .map_err(unavailable)?;
                let direct_settings =
                    direct.get("settingsConfig").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
                let published = &context
                    .expected
                    .aggregation_recovery
                    .as_ref()
                    .ok_or(StatusCode::CONFLICT)?
                    .published;
                let restore = Restore {
                    project: &context.project,
                    expected: &context.expected,
                    published,
                    direct: direct_settings,
                };
                match app {
                    AppId::Claude => {
                        let _directory = tx.lock_claude_directory().map_err(unavailable)?;
                        let selected = tx.claude_directory(&self.tenant).map_err(unavailable)?;
                        let mut file =
                            ClaudeFile::open_selected(&context.home, selected.as_ref(), false)
                                .map_err(unavailable)?;
                        lifecycle_restore::detach_claude(
                            &mut file,
                            &tx,
                            restore,
                            || self.unchanged(&context),
                            || false,
                        )
                        .map_err(lifecycle_status)?;
                    }
                    AppId::Codex => {
                        let profile =
                            vw_agent_apps::providers::codex_catalog::form::editable_profile(
                                direct_id, &direct,
                            )
                            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
                        let deployment = vw_agent_apps::providers::codex_catalog::identity(
                            &self.host, direct_id, &profile,
                        )
                        .map_err(unavailable)?;
                        let catalog = crate::model_catalog::ModelCatalogStore::read_existing(
                            &self.database,
                            &self.tenant,
                            &deployment,
                            direct["modelCatalogRevision"].as_str(),
                        )
                        .map_err(unavailable)?;
                        let mut files = CodexFiles::open(&context.home).map_err(unavailable)?;
                        lifecycle_restore::detach_codex(
                            &mut files,
                            &tx,
                            restore,
                            LoginInput {
                                target: AuthTarget::ThirdParty { preserve: profile.preserve_login },
                                leaving_official: None,
                                official_logins: &[],
                                third_party_keys: &[],
                                live_is_managed: false,
                                now: chrono::Utc::now().timestamp(),
                            },
                            catalog.as_ref(),
                            || self.unchanged(&context),
                            || false,
                        )
                        .map_err(lifecycle_status)?;
                    }
                    _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
                }
                Ok(Some(scope))
            })();
            match attempt {
                Ok(Some(scope)) => result.retired.push(scope),
                Ok(None) => {}
                Err(status) => {
                    if let Some(scope) = authorized_scope
                        && tx.pending(app).is_ok_and(|pending| {
                            pending.is_some_and(|pending| {
                                pending.published
                                    && pending.tenant == self.tenant
                                    && pending.target.project_scope.as_ref() == Some(&scope.project)
                            })
                        })
                    {
                        result.retired.push(scope);
                    }
                    tracing::warn!(
                        ?app,
                        status = status.as_u16(),
                        "aggregation shutdown restoration failed"
                    );
                    result.failure.get_or_insert(status);
                }
            }
        }
        result
    }
}
