//! Prepared execution-host egress, shared by control and the authorized data plane.

use super::{SharedTransport, outbound_custom, outbound_system::SystemProxy, transport_security};
use crate::security::SecretStore;
use axum::http::StatusCode;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use vw_api_types::{
    app_routing::{RoutingEgressMode, RoutingEgressStatus, RoutingEgressUpdate},
    id::TenantScope,
};

pub(crate) struct RoutingEgress {
    pub(crate) status: RoutingEgressStatus,
    pub(super) transport: SharedTransport,
}

#[derive(Debug)]
pub(crate) enum EgressError {
    Invalid,
    NotApplied,
    Listener(super::ListenerError),
}

impl RoutingEgress {
    pub(crate) fn prepare(
        update: &RoutingEgressUpdate,
        listener: Option<SocketAddr>,
    ) -> Result<Self, EgressError> {
        use RoutingEgressMode::{Custom, Direct, System};
        let (transport, effective_mode, scheme, has_credentials) =
            match (update.mode, update.proxy_url.as_deref()) {
                (Custom, Some(url)) => {
                    let (proxy, intercept) =
                        outbound_custom::parse(url).map_err(|_| EgressError::Invalid)?;
                    if let Some(listener) = listener
                        && !matches!(
                            transport_security::proxy_is_listener(intercept.uri(), listener),
                            Ok(false)
                        )
                    {
                        return Err(EgressError::NotApplied);
                    }
                    let scheme = intercept.uri().scheme_str().map(str::to_owned);
                    let has_credentials =
                        intercept.basic_auth().is_some() || intercept.raw_auth().is_some();
                    let transport = SharedTransport::build_custom_proxy(proxy, intercept)
                        .map_err(|_| EgressError::NotApplied)?;
                    (transport, Custom, scheme, has_credentials)
                }
                (mode @ (System | Direct), None) => {
                    let system = match mode {
                        System => SystemProxy::read(),
                        _ => SystemProxy::from_settings(None, |_| None),
                    };
                    let (scheme, has_credentials) = system.proxy_metadata();
                    let effective = if scheme.is_some() { System } else { Direct };
                    let transport = SharedTransport::with_system_proxy(system)
                        .map_err(|_| EgressError::NotApplied)?;
                    (transport, effective, scheme, has_credentials)
                }
                _ => return Err(EgressError::Invalid),
            };
        Ok(Self {
            status: RoutingEgressStatus {
                configured_mode: update.mode,
                effective_mode,
                scheme,
                has_credentials,
            },
            transport,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredEgress {
    mode: RoutingEgressMode,
    encrypted_url: Option<String>,
}

impl StoredEgress {
    pub(super) fn encode(
        update: &RoutingEgressUpdate,
        secrets: &SecretStore,
    ) -> anyhow::Result<String> {
        let encrypted_url =
            update.proxy_url.as_deref().map(|url| secrets.encrypt(url)).transpose()?;
        Ok(serde_json::to_string(&Self { mode: update.mode, encrypted_url })?)
    }

    pub(super) fn restore(
        payload: Option<&str>,
        secrets: &SecretStore,
    ) -> anyhow::Result<RoutingEgress> {
        let update = if let Some(payload) = payload {
            let Self { mode, encrypted_url } = serde_json::from_str(payload)?;
            let proxy_url = encrypted_url
                .map(|encrypted| {
                    anyhow::ensure!(encrypted.starts_with("enc2:"), "invalid egress secret format");
                    secrets.decrypt(&encrypted)
                })
                .transpose()?;
            RoutingEgressUpdate { mode, proxy_url }
        } else {
            RoutingEgressUpdate { mode: RoutingEgressMode::System, proxy_url: None }
        };
        RoutingEgress::prepare(&update, None)
            .map_err(|_| anyhow::anyhow!("stored egress unavailable"))
    }
}

#[derive(Default)]
pub(super) struct EgressState(RwLock<Option<ScopedEgress>>);

struct ScopedEgress {
    tenant: TenantScope,
    host: String,
    current: Option<Arc<RoutingEgress>>,
}

impl EgressState {
    pub(super) fn set(&self, tenant: TenantScope, host: String, current: Option<RoutingEgress>) {
        *self.0.write() = Some(ScopedEgress { tenant, host, current: current.map(Arc::new) });
    }

    pub(super) fn for_scope(
        &self,
        tenant: &TenantScope,
        host: &str,
    ) -> Result<Option<Arc<RoutingEgress>>, StatusCode> {
        match self.0.read().as_ref() {
            Some(scope) if &scope.tenant == tenant && scope.host == host => scope
                .current
                .as_ref()
                .map(Arc::clone)
                .map(Some)
                .ok_or(StatusCode::SERVICE_UNAVAILABLE),
            _ => Ok(None),
        }
    }
}
