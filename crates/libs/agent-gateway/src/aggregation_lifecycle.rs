//! Gateway lifecycle for saved aggregation intent. The original journal owns
//! files and recovery; this executor supplies only policy-bound, frozen inputs.
use crate::publication_context::{PublicationContext as RecordPublicationContext, PublicationHost};
use {
    app_provider_services::AppProviderStore, app_routing_services::RoutingListener, agent_database::GlobalDatabase,
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
use {
    vw_api_app_identity::AppId,
    vw_api_app_failover::FailoverQueueScope,
    vw_api_transport::id::TenantScope,
};

#[derive(Clone)]
pub struct AggregationLifecycle {
    pub database: GlobalDatabase,
    pub policy: PathBuf,
    pub tenant: TenantScope,
    pub host: String,
}

pub use app_routing_services::lifecycle_host::DetachResult;

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
    pub fn restore(
        &self,
        host: &impl PublicationHost,
        routes: &app_routing_services::RouteRestore,
        app: app_routing_services::lifecycle_host::RestoreApp,
        authorized: &dyn Fn() -> bool,
    ) -> Result<(), StatusCode> {
        if app.scope.target_host_id != self.host
            || app.scope.project.tenant_scope() != &self.tenant
        {
            return Err(StatusCode::FORBIDDEN);
        }
        if !matches!(app.scope.app_id, AppId::Claude | AppId::Codex | AppId::ClaudeDesktop) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let mut context = RecordPublicationContext {
            database: self.database.clone(),
            project: app.scope.project,
            host: self.host.clone(),
            app: app.scope.app_id,
            home: app.home,
            root: app.root,
            routing: None,
            expected: app.expected,
            principal: app.principal,
            revision: String::new(),
            failure: Default::default(),
        };
        context.revision = host.revision(&context, None)?;
        context.exit_with_routes(routes, authorized)
    }

    fn transactions(&self) -> Result<Option<ProviderTransactions>, StatusCode> {
        let (_, root) = app_access_service::access::aggregation_lifecycle_host(
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
    fn context(&self, host: &impl PublicationHost, app: AppId, state: AppState) -> Result<RecordPublicationContext, StatusCode> {
        let project = state.project_scope.as_ref().ok_or(StatusCode::FORBIDDEN)?;
        if project.tenant_scope() != &self.tenant {
            return Err(StatusCode::FORBIDDEN);
        }
        let principal = match app {
            AppId::Claude | AppId::Codex if state.stack.enabled => Some(
                state.aggregation_recovery.as_ref().ok_or(StatusCode::CONFLICT)?.principal.as_str(),
            ),
            // Only CLI aggregation persists a recovery principal.
            AppId::Claude | AppId::Codex | AppId::ClaudeDesktop => None,
            _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        };
        let (home, root, principal) = app_access_service::access::authorize_aggregation_lifecycle(
            &self.policy,
            project,
            &self.host,
            app,
            principal,
        )?;
        let mut context = RecordPublicationContext {
            database: self.database.clone(),
            project: project.clone(),
            host: self.host.clone(),
            app,
            home,
            root,
            routing: None,
            expected: state,
            principal,
            revision: String::new(),
            failure: Default::default(),
        };
        context.revision = host.revision(&context, None)?;
        Ok(context)
    }
    fn unchanged(&self, host: &impl PublicationHost, context: &RecordPublicationContext) -> bool {
        let authorization = app_access_service::access::authorize_aggregation_lifecycle(
            &self.policy,
            &context.project,
            &self.host,
            context.app,
            Some(&context.principal),
        );
        authorization.is_ok_and(|(home, root, principal)| {
            home == context.home && root == context.root && principal == context.principal
        }) && context.validate_inputs(host).is_ok()
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
    fn desktop_route_is_live(
        &self,
        routing: &RoutingListener,
        project: &vw_api_transport::ProjectScope,
    ) -> bool {
        let binding = app_routing_services::VerifiedRouteBinding::new(
            project.clone(),
            self.host.clone(),
            AppId::ClaudeDesktop,
            "saved-aggregation".into(),
            1,
        );
        routing.desktop_revision(&binding).is_some()
    }

    fn require_desktop_installation(&self) -> Result<(), StatusCode> {
        use vw_agent_apps::providers::claude_desktop::publication::DesktopInstallation;
        let (environment, _) = app_access_service::access::aggregation_lifecycle_host(
            &self.policy,
            &self.tenant,
            &self.host,
        )?;
        let observation = vw_agent_apps::apps::discovery::read::read(AppId::ClaudeDesktop, &environment)
            .map_err(unavailable)?
            .desktop
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        if DesktopInstallation::from(&observation) != DesktopInstallation::Installed {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        Ok(())
    }

    fn resume_provider(
        &self,
        host: &impl PublicationHost,
        routing: &Arc<RoutingListener>,
        tx: &ProviderTransactions,
        app: AppId,
        state: AppState,
    ) -> Result<(), StatusCode> {
        let Some(mode) = &state.mode else {
            return Ok(());
        };
        if !mode.attached || mode.mode != Mode::Proxy {
            return Ok(());
        }
        let project = state.project_scope.clone().ok_or(StatusCode::FORBIDDEN)?;
        state.require_project(&project).map_err(|_| StatusCode::FORBIDDEN)?;
        let provider_id = mode.proxy_route.clone().ok_or(StatusCode::CONFLICT)?;
        let scope = FailoverQueueScope {
            project,
            target_host_id: self.host.clone(),
            app_id: app,
        };
        if tx.pending(app).map_err(unavailable)?.is_none() {
            let live = if app == AppId::ClaudeDesktop {
                self.desktop_route_is_live(routing, &scope.project)
            } else {
                routing
                    .with_provider(&scope, |provider, _| Ok(provider.matches_saved(&provider_id, &state)))
                    .unwrap_or(false)
            };
            if live {
                return Ok(());
            }
        }

        match app {
            AppId::Claude => {
                use vw_api_app_provider_environment::mode_apply::ModeApplyAction;
                let (home, root, _) = app_access_service::access::authorize_aggregation_lifecycle(
                    &self.policy,
                    &scope.project,
                    &self.host,
                    app,
                    None,
                )?;
                app_routing_services::provider_publication::apply_checked(
                    self.database.clone(),
                    routing,
                    &home,
                    &root,
                    &scope,
                    &ModeApplyAction::Proxy { provider_id },
                    || tx.state(app).is_ok_and(|current| current == state),
                )?;
            }
            AppId::Codex | AppId::ClaudeDesktop => {
                if app == AppId::ClaudeDesktop {
                    self.require_desktop_installation()?;
                }
                let mut context = self.context(host, app, state)?;
                let store = AppProviderStore::new(self.database.clone()).map_err(unavailable)?;
                let record = store
                    .publication_record(&self.tenant, app, &provider_id)
                    .map_err(unavailable)?;
                context.routing = Some(Arc::clone(routing));
                let kind = serde_json::to_string(&app).map_err(unavailable)?;
                let key = format!("{kind}:{provider_id}");
                if !context.republish_saved(&key, &record)? {
                    return Err(StatusCode::CONFLICT);
                }
            }
            _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        }
        Ok(())
    }

    pub fn resume(&self, host: &impl PublicationHost, routing: Arc<RoutingListener>) -> Result<(), StatusCode> {
        let Some(tx) = self.transactions()? else {
            return Ok(());
        };
        let mut failure = None;
        for app in [AppId::Claude, AppId::Codex, AppId::ClaudeDesktop] {
            let result: Result<(), StatusCode> = (|| {
                let state = tx.state(app).map_err(unavailable)?;
                if state
                    .project_scope
                    .as_ref()
                    .is_some_and(|project| project.tenant_scope() != &self.tenant)
                {
                    return Ok(());
                }
                if !state.stack.enabled {
                    return self.resume_provider(host, &routing, &tx, app, state);
                }
                let mut context = self.context(host, app, state)?;
                if lifecycle_restore::startup_action(&tx, &context.project, app)
                    .map_err(restore_status)?
                    == StartupAction::StayDirect
                {
                    return Ok(());
                }
                let live = if app == AppId::ClaudeDesktop {
                    self.desktop_route_is_live(&routing, &context.project)
                } else {
                    routing.aggregation_revision(&context.project, &self.host, app).is_some()
                };
                if live && context.expected.mode.as_ref().is_some_and(|m| m.attached) {
                    return Ok(());
                }
                if app == AppId::ClaudeDesktop {
                    self.require_desktop_installation()?;
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
                if !host.publish_aggregation(&context, &default, policy, &row)? {
                    return Err(StatusCode::CONFLICT);
                }
                Ok(())
            })();
            if let Err(status) = result {
                tracing::warn!(
                    ?app,
                    status = status.as_u16(),
                    "saved application route recovery failed"
                );
                failure.get_or_insert(status);
            }
        }
        failure.map_or(Ok(()), Err)
    }
    pub fn detach(&self, host: &impl PublicationHost) -> DetachResult {
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
                let context = self.context(host, app, state)?;
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
                            || self.unchanged(host, &context),
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
                        let catalog = model_catalog::ModelCatalogStore::read_existing(
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
                            || self.unchanged(host, &context),
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
