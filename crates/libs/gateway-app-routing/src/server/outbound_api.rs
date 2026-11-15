//! Authorized reads and replacements of execution-host egress.
use super::{
    config_api::{ConfigBody, decode},
    lifecycle_api::{ControlError, RoutingControl},
};
use {
    app_routing_services::{ListenerError, outbound_config::EgressError},
    secret_store::secrets::SecretStore,
};
use axum::{Extension, Json, extract::Path, http::HeaderMap};
use {
    vw_api_app_routing::RoutingEgressStatus,
    vw_api_app_routing::RoutingEgressUpdate,
    vw_api_transport::id::TenantScope,
};

pub fn secrets(config: &std::path::Path) -> SecretStore {
    SecretStore::new(config, true)
}

pub async fn get_egress(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
) -> Result<Json<RoutingEgressStatus>, ControlError> {
    let (control, _) =
        RoutingControl::authorize(control, headers, tenant.clone(), host.clone(), None, false)
            .await?;
    let status = control.routing.egress_status(&tenant, &host).ok_or(ControlError::Unavailable)?;
    Ok(Json(status))
}

pub async fn put_egress(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
    body: ConfigBody,
) -> Result<Json<RoutingEgressStatus>, ControlError> {
    let (control, _) =
        RoutingControl::authorize(control, headers, tenant, host, None, true).await?;
    let update: RoutingEgressUpdate = decode(body)?;
    let routing = control.routing.clone();
    let secrets = secrets(&control.paths.config);
    // Once scheduled, the worker owns validation, the commit and publication.
    let status = control
        .with_config_store(move |store| Ok(store.apply_egress(&routing, &update, &secrets)))
        .await
        .map_err(|_| ControlError::EgressNotApplied)?
        .map_err(|error| match error {
            EgressError::Invalid => ControlError::InvalidConfig,
            EgressError::NotApplied
            | EgressError::Listener(ListenerError::TransitionInProgress) => {
                ControlError::EgressNotApplied
            }
            EgressError::Listener(error) => super::lifecycle_api::listener_error(error),
        })?;
    Ok(Json(status))
}
