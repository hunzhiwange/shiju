//! Authorized runtime settings for the gateway-owned listener.
use super::lifecycle_api::{ControlError, RoutingControl, apps_attached};
use app_routing_services::{
    ListenerError,
    config_apply::{AppTimeouts, ListenerConfig},
};
use axum::{
    Extension, Json,
    extract::{
        Path,
        rejection::PathRejection,
    },
    http::HeaderMap,
};
use std::sync::Arc;
use {
    vw_api_app_identity::AppId,
    vw_api_app_routing::AppRoutingConfig,
    vw_api_app_routing::RoutingConfig,
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
};

pub use super::{ConfigBody, decode};
type AppPath = Result<Path<(String, ProjectId, AppId)>, PathRejection>;

pub async fn get_config(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
) -> Result<Json<RoutingConfig>, ControlError> {
    let (control, _) =
        RoutingControl::authorize(control, headers, tenant, host, None, false).await?;
    Ok(Json(routing_config(control.config().await?)))
}

pub async fn put_config(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
    body: ConfigBody,
) -> Result<Json<RoutingConfig>, ControlError> {
    let (control, root) =
        RoutingControl::authorize(control, headers, tenant, host.clone(), None, true).await?;
    let input: RoutingConfig = decode(body)?;
    let config = ListenerConfig {
        enabled: input.enabled,
        bind: (input.listen_address.to_canonical(), input.listen_port).into(),
        logging_enabled: input.logging_enabled,
    };
    config.validate().map_err(|_| ControlError::InvalidConfig)?;

    let store = Arc::new(control.with_config_store(Ok).await?);
    store
        .apply_listener(&control.routing, control.policy_default, config, move || {
            let attached = apps_attached(root, &host)
                .map_err(|_| anyhow::anyhow!("routing attachment status is unavailable"))?;
            if attached {
                return Err(ListenerError::AppsAttached.into());
            }
            Ok(())
        })
        .await
        .map_err(|error| match error.downcast::<ListenerError>() {
            Ok(
                ListenerError::AddressInUse | ListenerError::Bind(_) | ListenerError::AppsAttached,
            ) => ControlError::ConfigNotApplied,
            Ok(error) => super::lifecycle_api::listener_error(error),
            Err(_) => ControlError::Unavailable,
        })?;
    Ok(Json(routing_config(config)))
}

pub async fn get_app_config(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    path: AppPath,
    headers: HeaderMap,
) -> Result<Json<AppRoutingConfig>, ControlError> {
    let (control, project, app) = authorize_app(control, tenant, path, headers, false).await?;
    let timeouts = control
        .with_config_store(move |store| Ok(store.app_timeouts(&project, app)?.unwrap_or_default()))
        .await?;
    Ok(Json(app_config(timeouts)))
}

pub async fn put_app_config(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    path: AppPath,
    headers: HeaderMap,
    body: ConfigBody,
) -> Result<Json<AppRoutingConfig>, ControlError> {
    let (control, project, app) = authorize_app(control, tenant, path, headers, true).await?;
    let input: AppRoutingConfig = decode(body)?;
    let timeouts = AppTimeouts {
        first_byte_timeout_secs: input.first_byte_timeout_secs,
        idle_timeout_secs: input.idle_timeout_secs,
        non_streaming_timeout_secs: input.non_streaming_timeout_secs,
    };
    timeouts.validate().map_err(|_| ControlError::InvalidConfig)?;
    control
        .with_config_store(move |store| store.save_app_timeouts(&project, app, timeouts))
        .await?;
    Ok(Json(input))
}

async fn authorize_app(
    control: Option<Extension<RoutingControl>>,
    tenant: TenantScope,
    path: AppPath,
    headers: HeaderMap,
    manage: bool,
) -> Result<(RoutingControl, ProjectScope, AppId), ControlError> {
    let Path((host, project, app)) = path.map_err(|_| ControlError::InvalidConfig)?;
    let (control, _) = RoutingControl::authorize(
        control,
        headers,
        tenant.clone(),
        host,
        Some((project.clone(), app)),
        manage,
    )
    .await?;
    if !vw_agent_apps::routing::supports_app(app) {
        return Err(ControlError::UnsupportedApp);
    }
    let project = ProjectScope::new(tenant, project).map_err(|_| ControlError::InvalidConfig)?;
    Ok((control, project, app))
}

fn routing_config(config: ListenerConfig) -> RoutingConfig {
        RoutingConfig {
            enabled: config.enabled,
            listen_address: config.bind.ip(),
            listen_port: config.bind.port(),
            logging_enabled: config.logging_enabled,
        }
    }

fn app_config(timeouts: AppTimeouts) -> AppRoutingConfig {
        AppRoutingConfig {
            first_byte_timeout_secs: timeouts.first_byte_timeout_secs,
            idle_timeout_secs: timeouts.idle_timeout_secs,
            non_streaming_timeout_secs: timeouts.non_streaming_timeout_secs,
        }
    }
