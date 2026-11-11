//! Authorized reads and replacements of execution-host egress.
use super::{
    config_api::{ConfigBody, decode},
    lifecycle_api::{ControlError, RoutingControl},
};
use crate::{
    app_routing::{ListenerError, outbound_config::EgressError},
    security::SecretStore,
};
use axum::{Extension, Json, extract::Path, http::HeaderMap};
use vw_api_types::{
    app_routing::{RoutingEgressStatus, RoutingEgressUpdate},
    id::TenantScope,
};

pub(super) fn secrets() -> SecretStore {
    SecretStore::new(&crate::global::paths().config, true)
}

pub(crate) async fn get_egress(
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

pub(crate) async fn put_egress(
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
    // Once scheduled, the worker owns validation, the commit and publication.
    let status = control
        .with_config_store(move |store| Ok(store.apply_egress(&routing, &update, &secrets())))
        .await
        .map_err(|_| ControlError::EgressNotApplied)?
        .map_err(|error| match error {
            EgressError::Invalid => ControlError::InvalidConfig,
            EgressError::NotApplied
            | EgressError::Listener(ListenerError::TransitionInProgress) => {
                ControlError::EgressNotApplied
            }
            EgressError::Listener(error) => error.into(),
        })?;
    Ok(Json(status))
}
