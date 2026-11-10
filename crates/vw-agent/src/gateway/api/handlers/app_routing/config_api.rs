//! Authorized runtime settings for the gateway-owned listener.
use super::lifecycle_api::{ControlError, RoutingControl, apps_attached};
use crate::app_routing::{
    ListenerError,
    config_apply::{AppTimeouts, ListenerConfig},
};
use axum::{
    Extension, Json,
    extract::{
        Path,
        rejection::{JsonRejection, PathRejection},
    },
    http::HeaderMap,
};
use serde::de::DeserializeOwned;
use serde_json::value::RawValue;
use std::sync::Arc;
use vw_api_types::{
    agent_apps::AppId,
    app_routing::{AppRoutingConfig, RoutingConfig},
    id::{ProjectId, ProjectScope, TenantScope},
};

type ConfigBody = Result<Json<Box<RawValue>>, JsonRejection>;
type AppPath = Result<Path<(String, ProjectId, AppId)>, PathRejection>;

fn decode<T: DeserializeOwned>(body: ConfigBody) -> Result<T, ControlError> {
    let Json(body) = body.map_err(|_| ControlError::InvalidConfig)?;
    if !body.get().starts_with('{') {
        return Err(ControlError::InvalidConfig);
    }
    serde_json::from_str(body.get()).map_err(|_| ControlError::InvalidConfig)
}

pub(crate) async fn get_config(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
) -> Result<Json<RoutingConfig>, ControlError> {
    let (control, _) =
        RoutingControl::authorize(control, headers, tenant, host, None, false).await?;
    Ok(Json(control.config().await?.into()))
}

pub(crate) async fn put_config(
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
            Ok(error) => error.into(),
            Err(_) => ControlError::Unavailable,
        })?;
    Ok(Json(config.into()))
}

pub(crate) async fn get_app_config(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    path: AppPath,
    headers: HeaderMap,
) -> Result<Json<AppRoutingConfig>, ControlError> {
    let (control, project, app) = authorize_app(control, tenant, path, headers, false).await?;
    let timeouts = control
        .with_config_store(move |store| Ok(store.app_timeouts(&project, app)?.unwrap_or_default()))
        .await?;
    Ok(Json(timeouts.into()))
}

pub(crate) async fn put_app_config(
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

impl From<ListenerConfig> for RoutingConfig {
    fn from(config: ListenerConfig) -> Self {
        Self {
            enabled: config.enabled,
            listen_address: config.bind.ip(),
            listen_port: config.bind.port(),
            logging_enabled: config.logging_enabled,
        }
    }
}

impl From<AppTimeouts> for AppRoutingConfig {
    fn from(timeouts: AppTimeouts) -> Self {
        Self {
            first_byte_timeout_secs: timeouts.first_byte_timeout_secs,
            idle_timeout_secs: timeouts.idle_timeout_secs,
            non_streaming_timeout_secs: timeouts.non_streaming_timeout_secs,
        }
    }
}
