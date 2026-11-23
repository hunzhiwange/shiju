use super::{ControlError, RoutingControl, apps_attached, listener_error};
use crate::{RoutingRestoreFailure, RoutingRestoreResult};
use app_routing_services::{RouteRestore, lifecycle_host::RestoreApp};
use axum::{
    Extension, Json,
    extract::{Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
};
use std::path::PathBuf;
use vw_agent_apps::{
    apps::catalog::catalog,
    providers::live::commit::{ExecutionHostId, Mode, ProviderTransactions},
};
use vw_api_app_failover::FailoverQueueScope;
use vw_api_app_identity::AppId;
use vw_api_app_routing::RoutingLifecycleRequest;
use vw_api_transport::id::TenantScope;

pub(in crate::server) async fn stop_with_restore(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
    body: Result<Json<RoutingLifecycleRequest>, JsonRejection>,
) -> Result<Json<RoutingRestoreResult>, ControlError> {
    let (control, root) =
        RoutingControl::authorize(control, headers.clone(), tenant, host, None, true).await?;
    let Json(_) = body.map_err(|_| ControlError::InvalidInput)?;
    let routes = control
        .routing
        .begin_route_restore()
        .map_err(listener_error)?;
    // The same control permit protects preflight, restoration, and the final drain.
    tokio::spawn(async move {
        let worker_control = control.clone();
        let (routes, failures) = tokio::task::spawn_blocking(move || {
            restore_apps(&worker_control, &headers, root, routes)
        })
        .await
        .map_err(|_| ControlError::Unavailable)??;
        if failures.is_empty() {
            routes.stop().await.map_err(listener_error)?;
        }
        Ok(Json(RoutingRestoreResult {
            status: control.status(),
            failures,
        }))
    })
    .await
    .map_err(|_| ControlError::Unavailable)?
}

fn restore_apps(
    control: &RoutingControl,
    headers: &HeaderMap,
    root: Option<PathBuf>,
    routes: RouteRestore,
) -> Result<(RouteRestore, Vec<RoutingRestoreFailure>), ControlError> {
    let host = &control.scope.host;
    let tenant = &control.scope.tenant;
    let host_id = ExecutionHostId::new(host).map_err(|_| ControlError::Unavailable)?;
    let store = root
        .as_ref()
        .map(|root| ProviderTransactions::open_existing(root, host_id))
        .transpose()
        .map_err(|_| ControlError::Unavailable)?
        .flatten();
    let mut apps = Vec::new();
    if let (Some(store), Some(root)) = (&store, &root) {
        for app in catalog() {
            let app = app.metadata.id;
            let state = store.state(app).map_err(|_| ControlError::Unavailable)?;
            let pending = store.pending(app).map_err(|_| ControlError::Unavailable)?;
            let routed = state.stack.enabled
                || state
                    .mode
                    .as_ref()
                    .is_some_and(|mode| mode.attached || mode.mode == Mode::Proxy);
            let pending_route = pending.as_ref().is_some_and(|pending| {
                pending
                    .target
                    .mode
                    .as_ref()
                    .is_some_and(|mode| mode.attached || mode.mode == Mode::Proxy)
                    || pending
                        .target
                        .stack
                        .as_ref()
                        .is_some_and(|stack| stack.enabled)
            });
            if !routed && !pending_route {
                continue;
            }
            let project = state.project_scope.clone().ok_or(ControlError::Forbidden)?;
            if project.tenant_scope() != tenant
                || state.require_project(&project).is_err()
                || pending.as_ref().is_some_and(|pending| {
                    &pending.tenant != tenant
                        || pending.target.project_scope.as_ref() != Some(&project)
                })
            {
                return Err(ControlError::Forbidden);
            }
            let scope = FailoverQueueScope {
                project,
                target_host_id: host.clone(),
                app_id: app,
            };
            let (home, principal) = authorize_app(control, headers, &scope, root)?;
            apps.push(RestoreApp {
                scope,
                home,
                root: root.clone(),
                principal,
                expected: state,
            });
        }
    }
    let scopes = apps.iter().map(|app| app.scope.clone()).collect::<Vec<_>>();
    if routes.has_routes_outside(&scopes) {
        return Err(ControlError::Forbidden);
    }
    let mut failures = Vec::new();
    if !apps.is_empty() {
        let lifecycle = control
            .local_lifecycle
            .as_ref()
            .map(|lifecycle| {
                lifecycle(
                    control.database.clone(),
                    control.policy.clone(),
                    tenant.clone(),
                    host.clone(),
                )
            })
            .or_else(|| control.routing.aggregation_lifecycle())
            .ok_or(ControlError::Unavailable)?;
        for app in apps {
            let scope = app.scope.clone();
            let root = app.root.clone();
            let owner = (app.home.clone(), app.principal.clone());
            let authorized = || {
                authorize_app(control, headers, &scope, &root).is_ok_and(|current| current == owner)
            };
            let result = (|| {
                if scope.app_id == AppId::ClaudeDesktop
                    && app.expected.mode.as_ref().is_some_and(|mode| mode.attached)
                {
                    require_desktop(control, headers, &scope, &owner)?;
                }
                lifecycle.restore(&routes, app, &authorized)
            })();
            if let Err(status) = result {
                failures.push(RoutingRestoreFailure {
                    app_id: scope.app_id,
                    status: status.as_u16(),
                });
            }
        }
    }
    if failures.is_empty() && (routes.has_routes_outside(&[]) || apps_attached(root, host)?) {
        return Err(ControlError::AppsAttached);
    }
    Ok((routes, failures))
}

fn authorize_app(
    control: &RoutingControl,
    headers: &HeaderMap,
    scope: &FailoverQueueScope,
    root: &std::path::Path,
) -> Result<(PathBuf, String), StatusCode> {
    let host = &scope.target_host_id;
    let (route_root, route_principal) =
        control.authorize_mode(headers, &scope.project, host, scope.app_id)?;
    let (home, app_root, principal) = app_access_service::access::authorize_aggregation_write(
        Some(&control.policy),
        headers,
        &scope.project,
        host,
        scope.app_id,
        true,
    )?;
    if route_root != root || app_root.as_deref() != Some(root) || route_principal != principal {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok((home, principal))
}

fn require_desktop(
    control: &RoutingControl,
    headers: &HeaderMap,
    scope: &FailoverQueueScope,
    owner: &(PathBuf, String),
) -> Result<(), StatusCode> {
    use vw_agent_apps::{
        apps::discovery::read::{LocalReadEnvironment, read},
        providers::claude_desktop::publication::DesktopInstallation,
    };
    let (host, principal) = app_access_service::access::authorize_provider_discovery(
        Some(&control.policy),
        headers,
        scope.project.tenant_scope(),
        &scope.target_host_id,
    )?;
    if host.home != owner.0 || principal != owner.1 {
        return Err(StatusCode::FORBIDDEN);
    }
    let env = LocalReadEnvironment {
        host_id: host.id,
        home: host.home,
        search_paths: host.search_paths,
        applications: host.applications,
    };
    let desktop = read(AppId::ClaudeDesktop, &env)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .desktop
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if DesktopInstallation::from(&desktop) != DesktopInstallation::Installed {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(())
}
