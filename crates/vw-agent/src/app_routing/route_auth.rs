//! Application capabilities owned by the gateway's routing listener.

use super::{browser_guard, dispatch::JsonTarget, header_wire};
use axum::{
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use hyper::{Request, body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, convert::Infallible, future::Future, sync::Arc};
use tokio::{net::TcpStream, sync::oneshot, time::timeout};
use tokio_util::sync::CancellationToken;
use vw_api_types::{agent_apps::AppId, id::ProjectScope};

const TOKEN_PREFIX: &str = "sra_";
const TOKEN_BYTES: usize = 32;
const API_KEY: &str = "x-api-key";

/// The mode transaction verifies project permission and current account ownership
/// before constructing this proof. Request headers never supply these fields.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct VerifiedRouteBinding {
    project: ProjectScope,
    host: String,
    app: AppId,
    account: String,
    account_generation: u64,
}

impl VerifiedRouteBinding {
    #[cfg(target_os = "macos")]
    pub(crate) fn new(
        project: ProjectScope,
        host: String,
        app: AppId,
        account: String,
        account_generation: u64,
    ) -> Self {
        Self { project, host, app, account, account_generation }
    }

    fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.host.trim().is_empty()
                && !self.account.trim().is_empty()
                && self.account_generation != 0,
            "incomplete application route binding"
        );
        Ok(())
    }

    fn same_route(&self, other: &Self) -> bool {
        self.project == other.project && self.host == other.host && self.app == other.app
    }
}

/// Neither the accepted binding nor its epoch can be changed by a handler.
pub(super) struct AuthorizedRoute {
    binding: VerifiedRouteBinding,
    generation: u64,
    cancelled: CancellationToken,
    json_target: Option<JsonTarget>,
}

impl AuthorizedRoute {
    pub(super) fn json_target(&self) -> Option<(AppId, &JsonTarget, CancellationToken)> {
        self.json_target.as_ref().map(|target| (self.binding.app, target, self.cancelled.clone()))
    }

    /// The dispatcher compares the selected target's trusted scope and epoch
    /// before accessing provider state or reading the request body.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "consumed when protocol dispatch is attached")
    )]
    pub(super) fn matches(&self, binding: &VerifiedRouteBinding, generation: u64) -> bool {
        !self.cancelled.is_cancelled() && &self.binding == binding && self.generation == generation
    }
}

type TokenDigest = [u8; 32];

/// The raw secret is returned once for private client configuration. Revocation
/// keeps only the digest, so a stale handle cannot cancel its replacement.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "production issuance awaits the authorized attach transaction")
)]
struct IssuedRoute {
    token: String,
    revocation: RouteRevocation,
}

struct RouteRevocation {
    digest: TokenDigest,
    generation: u64,
}

#[derive(Default)]
struct ActiveRoutes {
    grants: HashMap<TokenDigest, Arc<AuthorizedRoute>>,
    generation: u64,
}

#[derive(Default)]
pub(super) struct RouteAuth(Mutex<ActiveRoutes>);

impl RouteAuth {
    #[cfg(target_os = "macos")]
    pub(super) fn prepare(&self, binding: &VerifiedRouteBinding) -> anyhow::Result<()> {
        binding.validate()?;
        anyhow::ensure!(
            self.0.lock().generation < u64::MAX,
            "application route generation exhausted"
        );
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub(super) fn has_active_routes(&self) -> bool {
        !self.0.lock().grants.is_empty()
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "production issuance awaits the authorized attach transaction")
    )]
    fn issue(&self, binding: VerifiedRouteBinding) -> anyhow::Result<IssuedRoute> {
        self.issue_with_target(binding, None)
    }

    // Attach the trusted destination when publishing the same immutable grant.
    // Rotating or revoking the grant also retires its target and in-flight work.
    fn issue_with_target(
        &self,
        binding: VerifiedRouteBinding,
        json_target: Option<JsonTarget>,
    ) -> anyhow::Result<IssuedRoute> {
        binding.validate()?;
        if let Some(target) = &json_target {
            // Reject bad persisted options before invalidating the current grant.
            vw_agent_apps::providers::validation::validate(binding.app, &target.request_options)
                .map_err(anyhow::Error::msg)?;
        }
        let mut bytes = [0; TOKEN_BYTES];
        getrandom::fill(&mut bytes)
            .map_err(|_| anyhow::anyhow!("application route entropy unavailable"))?;
        let token = format!("{TOKEN_PREFIX}{}", hex::encode(bytes));
        let digest = Sha256::digest(token.as_bytes()).into();

        let mut active = self.0.lock();
        let generation = active
            .generation
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("application route generation exhausted"))?;
        anyhow::ensure!(!active.grants.contains_key(&digest), "application route token collision");

        // A project/host/app has one active route. A rotation or account
        // switch invalidates the previous credential and its in-flight work.
        active.grants.retain(|_, route| {
            if route.binding.same_route(&binding) {
                route.cancelled.cancel();
                false
            } else {
                true
            }
        });
        active.generation = generation;
        active.grants.insert(
            digest,
            Arc::new(AuthorizedRoute {
                binding,
                generation,
                cancelled: CancellationToken::new(),
                json_target,
            }),
        );
        Ok(IssuedRoute { token, revocation: RouteRevocation { digest, generation } })
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production revocation awaits the authorized attach transaction"
        )
    )]
    fn revoke(&self, issued: &RouteRevocation) {
        let mut active = self.0.lock();
        if active
            .grants
            .get(&issued.digest)
            .is_some_and(|route| route.generation == issued.generation)
            && let Some(route) = active.grants.remove(&issued.digest)
        {
            route.cancelled.cancel();
        }
    }

    fn authorize(&self, headers: &HeaderMap) -> Option<Arc<AuthorizedRoute>> {
        let token = credential(headers)?;
        let payload = token.strip_prefix(TOKEN_PREFIX)?;
        if payload.len() != TOKEN_BYTES * 2 || !payload.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return None;
        }
        let digest: TokenDigest = Sha256::digest(token.as_bytes()).into();
        self.0.lock().grants.get(&digest).cloned()
    }
}

fn credential(headers: &HeaderMap) -> Option<&str> {
    let mut bearer = headers.get_all(header::AUTHORIZATION).iter();
    let mut keys = headers.get_all(API_KEY).iter();
    match (bearer.next(), keys.next()) {
        (Some(value), None) if bearer.next().is_none() => {
            let (scheme, token) = value.to_str().ok()?.split_once(' ')?;
            scheme.eq_ignore_ascii_case("bearer").then_some(token)
        }
        (None, Some(value)) if keys.next().is_none() => value.to_str().ok(),
        _ => None,
    }
}

pub(super) async fn serve<F, Fut>(connection: TcpStream, auth: Arc<RouteAuth>, dispatch: F)
where
    F: Fn(Request<Incoming>, Arc<AuthorizedRoute>) -> Fut,
    Fut: Future<Output = Response>,
{
    let Ok(local_addr) = connection.local_addr() else {
        return;
    };
    let Ok(Ok((connection, wire))) =
        timeout(header_wire::HEAD_TIMEOUT, header_wire::capture(connection)).await
    else {
        return;
    };
    let (send_cancel, receive_cancel) = oneshot::channel::<CancellationToken>();
    let connection_state = Mutex::new(Some((send_cancel, wire)));
    let service = service_fn(|mut request: Request<Incoming>| {
        let auth = &auth;
        let connection_state = &connection_state;
        let dispatch = &dispatch;
        async move {
            let has_origin = request.headers().contains_key(header::ORIGIN);
            if !browser_guard::allows_host(&request, local_addr)
                || (has_origin && !browser_guard::is_desktop_route(request.uri()))
            {
                return Ok::<_, Infallible>(StatusCode::FORBIDDEN.into_response());
            }
            let Some(route) = auth.authorize(request.headers()) else {
                return Ok(StatusCode::UNAUTHORIZED.into_response());
            };
            if has_origin && route.binding.app != AppId::ClaudeDesktop {
                return Ok(StatusCode::FORBIDDEN.into_response());
            }
            let Some((send_cancel, wire)) = connection_state.lock().take() else {
                return Ok(StatusCode::UNAUTHORIZED.into_response());
            };
            if send_cancel.send(route.cancelled.clone()).is_err() || route.cancelled.is_cancelled()
            {
                return Ok(StatusCode::UNAUTHORIZED.into_response());
            }
            request.headers_mut().remove(header::AUTHORIZATION);
            request.headers_mut().remove(API_KEY);
            request.extensions_mut().insert(wire);
            Ok(dispatch(request, route).await)
        }
    });
    // One request per connection lets revocation drop the whole HTTP future,
    // even while it cannot write the response because the client stopped reading.
    let mut http = http1::Builder::new();
    http.keep_alive(false)
        .max_headers(header_wire::MAX_HEADERS)
        .max_buf_size(header_wire::MAX_HEAD_BYTES)
        .timer(TokioTimer::new())
        .header_read_timeout(header_wire::HEAD_TIMEOUT);
    let revoked = async {
        match receive_cancel.await {
            Ok(cancel) => cancel.cancelled().await,
            Err(_) => std::future::pending().await,
        }
    };
    tokio::select! {
        biased;
        () = revoked => {}
        _ = http.serve_connection(TokioIo::new(connection), service) => {}
    }
}

#[cfg(test)]
#[path = "route_auth_tests.rs"]
mod route_auth_tests;

#[cfg(test)]
#[path = "browser_guard_tests.rs"]
mod browser_guard_tests;

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod dispatch_tests;

#[cfg(test)]
#[path = "override_apply_tests.rs"]
mod override_apply_tests;

#[cfg(test)]
#[path = "header_wire_tests.rs"]
mod header_wire_tests;
