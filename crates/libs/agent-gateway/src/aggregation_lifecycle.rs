//! Resume saved provider routes and retire legacy multi-provider configurations.
use crate::publication_context::{PublicationContext as RecordPublicationContext, PublicationHost};
use {
    app_provider_services::AppProviderStore, app_routing_services::RoutingListener, agent_database::GlobalDatabase,
};
use axum::http::StatusCode;
use std::{path::PathBuf, sync::Arc};
use vw_agent_apps::providers::live::commit::*;
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
        context.exit_with_routes(routes, authorized)?;
        context.release_legacy_bindings()
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
                    if !state.stack.members.is_empty() {
                        self.context(host, app, state.clone())?.release_legacy_bindings()?;
                    }
                    if routing.bound_address().is_some() {
                        return self.resume_provider(host, &routing, &tx, app, state);
                    }
                    return Ok(());
                }
                let context = self.context(host, app, state)?;
                let routes = routing.begin_route_restore().map_err(|_| StatusCode::CONFLICT)?;
                if app == AppId::ClaudeDesktop {
                    self.require_desktop_installation()?;
                }
                let authorization = || self.unchanged(host, &context);
                context.exit_with_routes(&routes, authorization)?;
                context.release_legacy_bindings()?;
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
                result.failure = Some(status);
                return result;
            }
        };
        for app in [AppId::Claude, AppId::Codex, AppId::ClaudeDesktop] {
            let attempt: Result<Option<FailoverQueueScope>, StatusCode> = (|| {
                let state = tx.state(app).map_err(unavailable)?;
                if !state.stack.enabled
                    || state.project_scope.as_ref().is_some_and(|scope| scope.tenant_scope() != &self.tenant)
                {
                    return Ok(None);
                }
                let context = self.context(host, app, state)?;
                if app == AppId::ClaudeDesktop {
                    self.require_desktop_installation()?;
                }
                context.restore_exit(&tx, &|| self.unchanged(host, &context))?;
                context.release_legacy_bindings()?;
                Ok(Some(FailoverQueueScope {
                    project: context.project,
                    target_host_id: self.host.clone(),
                    app_id: app,
                }))
            })();
            match attempt {
                Ok(Some(scope)) => result.retired.push(scope),
                Ok(None) => {}
                Err(status) => {
                    tracing::warn!(?app, status = status.as_u16(), "Legacy provider restoration failed");
                    result.failure.get_or_insert(status);
                }
            }
        }
        result
    }
}
