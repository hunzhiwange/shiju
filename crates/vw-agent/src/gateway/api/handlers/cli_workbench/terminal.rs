//! Native terminal I/O is available only through the direct local gateway.
use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{HeaderMap, header, request::Parts},
};
use vw_api_types::cli_workbench::CliWorkbenchTerminalCapability;

use super::*;

#[derive(Clone, Default)]
pub(crate) struct LocalWorkbenchGateway(CancellationToken);

impl LocalWorkbenchGateway {
    pub(crate) async fn shutdown(&self) {
        self.0.cancel();
        #[cfg(target_os = "macos")]
        native::shutdown().await;
    }
}

pub(super) struct Authority(Option<LocalWorkbenchGateway>);

impl<S: Send + Sync> FromRequestParts<S> for Authority {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let peer = parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|info| info.0);
        let local = parts.extensions.get::<LocalWorkbenchGateway>();
        Ok(Self(local.filter(|_| direct_local(peer, &parts.headers)).cloned()))
    }
}

fn direct_local(peer: Option<SocketAddr>, headers: &HeaderMap) -> bool {
    peer.is_some_and(|peer| peer.ip().is_loopback())
        && !headers.keys().any(|name| {
            name == header::ORIGIN
                || name == header::FORWARDED
                || name.as_str().starts_with("x-forwarded-")
        })
}

impl Authority {
    pub(super) fn available(&self) -> bool {
        cfg!(target_os = "macos") && self.0.as_ref().is_some_and(|local| !local.0.is_cancelled())
    }

    pub(super) fn require(&self) -> Result<(), WorkbenchError> {
        if self.available() { Ok(()) } else { Err(unavailable()) }
    }
}

pub(super) fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    let router = Router::new().route("/cli-workbench/terminal-capability", get(capability));
    #[cfg(target_os = "macos")]
    let router = router.route(
        &format!("{SESSIONS}/{{session}}/terminal"),
        get(native::output).post(native::input).layer(axum::extract::DefaultBodyLimit::max(
            4 * vw_api_types::cli_workbench::CLI_WORKBENCH_MAX_TERMINAL_INPUT + 512,
        )),
    );
    router
}

#[derive(Deserialize)]
struct CapabilityQuery {
    #[serde(default)]
    include_apps: bool,
}

async fn capability(
    authority: Authority,
    Extension(tenant): Extension<TenantScope>,
    Query(query): Query<CapabilityQuery>,
) -> Result<Json<CliWorkbenchTerminalCapability>, WorkbenchError> {
    let available = authority.available();
    #[cfg(target_os = "macos")]
    let native_apps = if available && query.include_apps {
        tokio::task::spawn_blocking(move || gateway_agent_apps::launch::workbench_native_apps(&tenant))
            .await
            .map_err(|_| unavailable())?
    } else {
        Vec::new()
    };
    #[cfg(not(target_os = "macos"))]
    let native_apps = {
        let _ = (tenant, query.include_apps);
        Vec::new()
    };
    Ok(Json(CliWorkbenchTerminalCapability { available, native_apps }))
}

#[cfg(target_os = "macos")]
mod native;
#[cfg(target_os = "macos")]
pub(super) use native::{Pty, start, stop};

#[cfg(not(target_os = "macos"))]
pub(super) async fn start(
    _: ProjectScope,
    _: String,
    _: String,
    _: String,
    _: CliWorkbenchTarget,
    _: Authority,
) -> Result<(StatusCode, Json<CliWorkbenchSession>), WorkbenchError> {
    Err(unavailable())
}
