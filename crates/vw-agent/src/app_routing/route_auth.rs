//! Application capabilities owned by the gateway's routing listener.

use super::{
    browser_guard,
    cancellation::{ClientDisconnect, ClientReader},
    capacity::RouteCapacity,
    dispatch::JsonTarget,
    header_wire,
    protocol_errors::{Protocol, RouteError},
};
use axum::{
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use hyper::{Request, body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, convert::Infallible, future::Future, io::Cursor, sync::Arc};
use tokio::{
    io::{AsyncReadExt, join},
    net::TcpStream,
    sync::oneshot,
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use vw_agent_apps::aggregation::model_ids::{Decoded, ModelIdError, decode};
use vw_api_types::{agent_apps::AppId, id::ProjectScope};
#[cfg(target_os = "macos")]
use {
    super::{
        aggregation_routing::AggregationSnapshot, config_apply::RoutingConfigStore,
        timeouts::AppTimeouts,
    },
    crate::global_database::GlobalDatabase,
};

#[cfg(target_os = "macos")]
type ProviderPublication = super::provider_publication::PublishedProvider;
#[cfg(not(target_os = "macos"))]
type ProviderPublication = ();

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

    #[cfg(target_os = "macos")]
    pub(crate) fn has_scope(&self, project: &ProjectScope, host: &str, app: AppId) -> bool {
        &self.project == project && self.host == host && self.app == app
    }

    #[cfg(target_os = "macos")]
    pub(super) fn app(&self) -> AppId {
        self.app
    }

    #[cfg(target_os = "macos")]
    pub(super) fn project(&self) -> &ProjectScope {
        &self.project
    }

    #[cfg(target_os = "macos")]
    pub(super) fn host(&self) -> &str {
        &self.host
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
    #[cfg(target_os = "macos")]
    pub(super) provider: Option<super::provider_publication::PublishedProvider>,
    #[cfg(target_os = "macos")]
    pub(super) failover: Arc<crate::gateway::agent_apps_failover_runtime::FailoverRuntime>,
    #[cfg(target_os = "macos")]
    pub(super) actual_route: Mutex<crate::gateway::agent_apps_failover_runtime::ActualRoute>,
    #[cfg(target_os = "macos")]
    pub(super) publication: Arc<Mutex<()>>,
    binding: VerifiedRouteBinding,
    pub(super) references: Arc<super::reference_scope::References>,
    pub(super) responses_history: Arc<crate::model_relay::responses_history::Store>,
    pub(super) gemini_shadow: Arc<crate::model_relay::gemini_shadow::Store>,
    generation: u64,
    pub(super) cancelled: CancellationToken,
    json_target: Option<JsonRoute>,
    pub(super) capacity: Arc<RouteCapacity>,
    #[cfg(target_os = "macos")]
    timeout_store: Option<Arc<RoutingConfigStore>>,
}

impl AuthorizedRoute {
    #[cfg(target_os = "macos")]
    pub(super) fn generation(&self) -> u64 {
        self.generation
    }

    pub(super) fn response_scope(
        &self,
        target: &JsonTarget,
    ) -> Result<super::reference_scope::ReferenceScope, StatusCode> {
        if self.cancelled.is_cancelled() {
            return Err(StatusCode::UNAUTHORIZED);
        }
        Ok(super::reference_scope::ReferenceScope::Application(
            super::reference_scope::ApplicationReferenceScope {
                project: self.binding.project.clone(),
                host: self.binding.host.clone(),
                app: self.binding.app,
                account: self.binding.account.clone(),
                provider: target.url.to_string(),
                credential_generation: self.binding.account_generation,
            },
        ))
    }

    #[cfg(target_os = "macos")]
    pub(super) fn validate_managed(
        &self,
        credential: &super::adapter_selection::ManagedCredential,
    ) -> Result<(), StatusCode> {
        if &credential.tenant != self.binding.project.tenant_scope()
            || credential.host != self.binding.host
            || credential.account != self.binding.account
        {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub(super) async fn check_provider(
        self: &Arc<Self>,
        address: std::net::SocketAddr,
    ) -> Result<(), StatusCode> {
        let Some(provider) = &self.provider else {
            return Ok(());
        };
        if provider.address != address {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        let route = Arc::clone(self);
        tokio::task::spawn_blocking(move || match &route.provider {
            Some(provider) => provider.check_current(),
            None => Ok(()),
        })
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    }

    #[cfg(target_os = "macos")]
    pub(super) fn provider_headers(
        &self,
        headers: &mut HeaderMap,
        model: Option<&str>,
    ) -> Result<(), StatusCode> {
        if let Some(provider) = &self.provider {
            provider.apply_headers(headers);
        }
        if let Some(JsonRoute::Desktop(snapshot)) = &self.json_target
            && snapshot.messages_api_key(&self.binding, model)?
            && let Some(key) = headers.remove(header::AUTHORIZATION)
        {
            headers.insert("x-api-key", key);
        }
        if self.binding.app == AppId::Claude
            && let Some(JsonRoute::Aggregation(snapshot)) = &self.json_target
        {
            if snapshot.messages_api_key(&self.binding, model)?
                && let Some(key) = headers.remove(header::AUTHORIZATION)
            {
                headers.insert("x-api-key", key);
            }
            headers.insert("anthropic-version", axum::http::HeaderValue::from_static("2023-06-01"));
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub(super) fn desktop_native_messages(&self, model: Option<&str>) -> Result<bool, StatusCode> {
        match &self.json_target {
            Some(JsonRoute::Desktop(snapshot)) => snapshot.native_messages(model),
            _ => Err(StatusCode::SERVICE_UNAVAILABLE),
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn model_discovery(&self) -> Result<serde_json::Value, StatusCode> {
        if self.cancelled.is_cancelled() {
            return Err(StatusCode::UNAUTHORIZED);
        }
        match &self.json_target {
            Some(JsonRoute::Desktop(snapshot)) => Ok(snapshot.discovery()),
            Some(JsonRoute::Plain(_)) => self.provider.as_ref()
                .ok_or(StatusCode::SERVICE_UNAVAILABLE)?.discovery(),
            // Aggregation IDs and their catalog publication belong to the aggregation owner.
            _ => Err(StatusCode::SERVICE_UNAVAILABLE),
        }
    }

    pub(super) fn json_app(&self) -> Option<AppId> {
        self.json_target.as_ref().map(|_| self.binding.app)
    }

    pub(super) fn select_json(
        &self,
        model: Option<&str>,
    ) -> Result<(&JsonTarget, Option<&str>, CancellationToken), StatusCode> {
        let route = self.json_target.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let (target, model) = match route {
            JsonRoute::Plain(target) if self.binding.app == AppId::Gemini => {
                (target.as_ref(), None)
            }
            JsonRoute::Plain(target) => match decode(self.binding.app, model.unwrap_or_default()) {
                Ok(Decoded::Plain) => (target.as_ref(), None),
                Ok(_) | Err(ModelIdError::UnsupportedApplication) => {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                Err(ModelIdError::InvalidModelId) => return Err(StatusCode::BAD_REQUEST),
            },
            #[cfg(target_os = "macos")]
            JsonRoute::Aggregation(snapshot) => snapshot.select(&self.binding, model)?,
            #[cfg(target_os = "macos")]
            JsonRoute::Desktop(snapshot) => snapshot.select(&self.binding, model)?,
        };
        Ok((target, model, self.cancelled.clone()))
    }

    #[cfg(target_os = "macos")]
    pub(super) fn egress(
        &self,
        state: &super::outbound_config::EgressState,
    ) -> Result<Option<Arc<super::outbound_config::RoutingEgress>>, StatusCode> {
        state.for_scope(self.binding.project.tenant_scope(), &self.binding.host)
    }

    #[cfg(target_os = "macos")]
    pub(super) async fn timeouts(&self) -> Result<AppTimeouts, StatusCode> {
        let store = Arc::clone(self.timeout_store.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?);
        let project = self.binding.project.clone();
        let app = self.binding.app;
        tokio::task::spawn_blocking(move || store.app_timeouts(&project, app))
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .map(Option::unwrap_or_default)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
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

enum JsonRoute {
    Plain(Box<JsonTarget>),
    #[cfg(target_os = "macos")]
    Aggregation(Box<AggregationSnapshot>),
    #[cfg(target_os = "macos")]
    Desktop(Box<super::desktop_routing::DesktopSnapshot>),
}

type TokenDigest = [u8; 32];

/// The raw secret is returned once for private client configuration. Revocation
/// keeps only the digest, so a stale handle cannot cancel its replacement.
struct IssuedRoute {
    #[cfg(test)]
    token: String,
    revocation: RouteRevocation,
}

pub(crate) struct RouteRevocation {
    digest: TokenDigest,
    generation: u64,
}

pub(crate) struct RouteCommitError {
    pub(crate) status: StatusCode,
    /// A durable intent or a partial write prevents using the old revision.
    pub(crate) recovery_pending: bool,
}

#[derive(Default)]
struct ActiveRoutes {
    grants: HashMap<TokenDigest, Arc<AuthorizedRoute>>,
    generation: u64,
}

impl ActiveRoutes {
    fn retire(&mut self, binding: &VerifiedRouteBinding) -> Option<Arc<RouteCapacity>> {
        let mut capacity = None;
        self.grants.retain(|_, route| {
            if route.binding.same_route(binding) {
                // Keep sharing the budget with requests that are unwinding.
                capacity = Some(Arc::clone(&route.capacity));
                route.cancelled.cancel();
                route.gemini_shadow.clear();
                false
            } else {
                true
            }
        });
        capacity
    }
}

#[derive(Default)]
pub(super) struct RouteAuth {
    pub(crate) requests: Arc<super::request_status::Requests>,
    #[cfg(target_os = "macos")]
    failover: Arc<crate::gateway::agent_apps_failover_runtime::FailoverRuntime>,
                route.responses_history.clear();
    active: Mutex<ActiveRoutes>,
    publication: Arc<Mutex<()>>,
}

impl RouteAuth {
    #[cfg(target_os = "macos")]
    pub(super) fn set_failover_events(
        &self,
        events: tokio::sync::broadcast::Sender<serde_json::Value>,
    ) {
        *self.failover.events.write() = Some(events);
    }

    #[cfg(target_os = "macos")]
    pub(super) fn prepare(&self, binding: &VerifiedRouteBinding) -> anyhow::Result<()> {
        binding.validate()?;
        anyhow::ensure!(
            self.active.lock().generation < u64::MAX,
            "application route generation exhausted"
        );
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub(super) fn has_active_routes(&self) -> bool {
        !self.active.lock().grants.is_empty()
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "unready grants are exercised by authorization tests")
    )]
    fn issue(&self, binding: VerifiedRouteBinding) -> anyhow::Result<IssuedRoute> {
        self.issue_with_target(binding, None)
    }

    #[cfg(all(test, target_os = "macos"))]
    pub(super) fn issue_test_token(
        &self,
        binding: VerifiedRouteBinding,
        target: JsonTarget,
    ) -> anyhow::Result<String> {
        Ok(self.issue_with_target(binding, Some(target))?.token)
    }

    #[cfg(all(test, target_os = "macos"))]
    pub(super) fn issue_revocable_test_token(
        &self,
        binding: VerifiedRouteBinding,
        target: JsonTarget,
    ) -> anyhow::Result<(String, impl Fn() + '_)> {
        let issued = self.issue_with_target(binding, Some(target))?;
        Ok((issued.token, move || self.revoke(&issued.revocation)))
    }

    fn issue_with_target(
        &self,
        binding: VerifiedRouteBinding,
        json_target: Option<JsonTarget>,
    ) -> anyhow::Result<IssuedRoute> {
        if let Some(target) = &json_target {
            vw_agent_apps::providers::validation::validate(binding.app, &target.request_options)
                .map_err(anyhow::Error::msg)?;
        }
        self.issue_route(
            binding,
            json_target.map(|target| JsonRoute::Plain(Box::new(target))),
            None,
            |_| Ok(None),
        )
        .map_err(|status| anyhow::anyhow!("application route issuance failed: {status}"))
    }

    #[cfg(target_os = "macos")]
    pub(super) fn publish_aggregation(
        &self,
        binding: VerifiedRouteBinding,
        snapshot: AggregationSnapshot,
        expected_revision: Option<&str>,
        commit: impl FnOnce(&str) -> Result<(), RouteCommitError>,
    ) -> Result<RouteRevocation, StatusCode> {
        let issued = self.issue_route(
            binding,
            Some(JsonRoute::Aggregation(Box::new(snapshot))),
            expected_revision,
            |token| commit(token).map(|()| None),
        )?;
        Ok(issued.revocation)
    }

    #[cfg(target_os = "macos")]
    pub(super) fn desktop_revision(&self, binding: &VerifiedRouteBinding) -> Option<String> {
        self.active.lock().grants.values().find_map(|route| {
            if !route.binding.same_route(binding) || route.cancelled.is_cancelled() {
                return None;
            }
            match &route.json_target {
                Some(JsonRoute::Desktop(snapshot)) => Some(snapshot.revision().to_owned()),
                _ => None,
            }
        })
    }

    #[cfg(target_os = "macos")]
    pub(super) fn publish_desktop(
        &self,
        binding: VerifiedRouteBinding,
        snapshot: super::desktop_routing::DesktopSnapshot,
        expected_revision: Option<&str>,
        commit: impl FnOnce(&str) -> Result<(), RouteCommitError>,
    ) -> Result<RouteRevocation, StatusCode> {
        self.issue_route(
            binding,
            Some(JsonRoute::Desktop(Box::new(snapshot))),
            expected_revision,
            |token| commit(token).map(|()| None),
        )
        .map(|issued| issued.revocation)
    }

    #[cfg(target_os = "macos")]
    pub(super) fn publish_plain(
        &self,
        binding: VerifiedRouteBinding,
        target: JsonTarget,
        expected_revision: Option<&str>,
        commit: impl FnOnce(&str) -> Result<(), RouteCommitError>,
    ) -> Result<RouteRevocation, StatusCode> {
        // Serialize the transition against aggregation publication and revocation.
        // Revision validation below applies to both directions of the transition.
        self.issue_route(
            binding,
            Some(JsonRoute::Plain(Box::new(target))),
            expected_revision,
            |token| commit(token).map(|()| None),
        )
        .map(|issued| issued.revocation)
    }

    #[cfg(target_os = "macos")]
    pub(super) fn publish_provider(
        &self,
        binding: VerifiedRouteBinding,
        target: JsonTarget,
        commit: impl FnOnce(
            &str,
        )
            -> Result<super::provider_publication::PublishedProvider, RouteCommitError>,
    ) -> Result<(), StatusCode> {
        self.issue_route(binding, Some(JsonRoute::Plain(Box::new(target))), None, |token| {
            commit(token).map(Some)
        })
        .map(|_| ())
    }

    #[cfg(target_os = "macos")]
    pub(super) fn with_provider<T>(
        &self,
        scope: &vw_api_types::agent_apps_high_availability::FailoverQueueScope,
        operation: impl FnOnce(
            &super::provider_publication::PublishedProvider,
            u64,
        ) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let _publication = self.publication.lock();
        let active = self.active.lock();
        let route = active
            .grants
            .values()
            .find(|route| {
                route.binding.has_scope(&scope.project, &scope.target_host_id, scope.app_id)
                    && !route.cancelled.is_cancelled()
            })
            .ok_or_else(|| anyhow::anyhow!("application route is not published"))?;
        let provider = route
            .provider
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("application route has no provider snapshot"))?;
        operation(provider, route.generation)
    }

    #[cfg(target_os = "macos")]
    pub(super) fn restore_provider(
        &self,
        scope: &vw_api_types::agent_apps_high_availability::FailoverQueueScope,
        commit: impl FnOnce() -> Result<(), RouteCommitError>,
    ) -> Result<(), StatusCode> {
        let _publication = self.publication.lock();
        let retire = || {
            self.active.lock().grants.retain(|_, route| {
                if route.binding.has_scope(&scope.project, &scope.target_host_id, scope.app_id) {
                    route.cancelled.cancel();
                route.gemini_shadow.clear();
                    false
                } else {
                    true
                }
            });
        };
        if let Err(error) = commit() {
            if error.recovery_pending {
                retire();
            }
            return Err(error.status);
        }
        retire();
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub(super) fn aggregation_revision(
        &self,
        project: &ProjectScope,
        host: &str,
        app: AppId,
    ) -> Option<String> {
        self.active
            .lock()
            .grants
            .values()
            .find(|route| route.binding.has_scope(project, host, app))
            .and_then(|route| match &route.json_target {
                Some(JsonRoute::Aggregation(snapshot)) => Some(snapshot.revision.clone()),
                _ => None,
            })
    }

    fn issue_route(
        &self,
        binding: VerifiedRouteBinding,
        json_target: Option<JsonRoute>,
        expected_revision: Option<&str>,
        commit: impl FnOnce(&str) -> Result<Option<ProviderPublication>, RouteCommitError>,
    ) -> Result<IssuedRoute, StatusCode> {
        binding.validate().map_err(|_| StatusCode::FORBIDDEN)?;
        #[cfg(not(target_os = "macos"))]
        let _ = expected_revision;
        #[cfg(target_os = "macos")]
        let timeout_store = if json_target.is_some() {
            let database = GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let store = RoutingConfigStore::new(
                database,
                binding.project.tenant_scope().clone(),
                binding.host.clone(),
            )
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            Some(Arc::new(store))
                route.responses_history.clear();
        } else {
            None
        };
        let mut bytes = [0; TOKEN_BYTES];
        getrandom::fill(&mut bytes).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let token = format!("{TOKEN_PREFIX}{}", hex::encode(bytes));
        let digest = Sha256::digest(token.as_bytes()).into();

        // Serialize grant mutations while the caller commits private client
        // configuration. Authentication can still read the previous grant.
        let _publication = self.publication.lock();
        let generation = {
            let active = self.active.lock();
            #[cfg(target_os = "macos")]
            if json_target.is_some() {
                let previous =
                    active.grants.values().find(|route| route.binding.same_route(&binding));
                let previous_revision = previous.and_then(|route| match &route.json_target {
                    Some(JsonRoute::Aggregation(snapshot)) => Some(snapshot.revision.as_str()),
                    Some(JsonRoute::Desktop(snapshot)) => Some(snapshot.revision()),
                    _ => None,
                });
                if previous_revision != expected_revision
                    || matches!(&json_target, Some(JsonRoute::Aggregation(next)) if previous_revision == Some(next.revision.as_str()))
                    || matches!(&json_target, Some(JsonRoute::Desktop(next)) if previous_revision == Some(next.revision()))
                    || previous.is_some_and(|route| {
                        route.binding.account == binding.account
                            && route.binding.account_generation > binding.account_generation
                    })
                {
                    return Err(StatusCode::CONFLICT);
                }
            }
            if active.grants.contains_key(&digest) {
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
            active.generation.checked_add(1).ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        };
        let provider = match commit(&token) {
            Ok(provider) => provider,
            Err(error) => {
                if error.recovery_pending {
                    drop(self.active.lock().retire(&binding));
                }
                return Err(error.status);
            }
        };

        #[cfg(not(target_os = "macos"))]
        let _ = provider;
        let mut active = self.active.lock();
        let capacity = active.retire(&binding);
        active.generation = generation;
        active.grants.insert(
            digest,
            Arc::new(AuthorizedRoute {
                #[cfg(target_os = "macos")]
                actual_route: Mutex::default(),
                #[cfg(target_os = "macos")]
                publication: Arc::clone(&self.publication),
                #[cfg(target_os = "macos")]
                failover: Arc::clone(&self.failover),
                #[cfg(target_os = "macos")]
                provider,
                binding,
                references: Arc::default(),
                gemini_shadow: Arc::default(),
                generation,
                cancelled: CancellationToken::new(),
                json_target,
                capacity: capacity.unwrap_or_default(),
                #[cfg(target_os = "macos")]
                timeout_store,
            }),
        );
        Ok(IssuedRoute {
            #[cfg(test)]
            token,
            revocation: RouteRevocation { digest, generation },
        })
    }

    #[cfg(target_os = "macos")]
    pub(super) fn revoke_providers(&self) {
        let _publication = self.publication.lock();
        self.active.lock().grants.retain(|_, route| {
            if route.provider.is_some() {
                route.cancelled.cancel();
                route.gemini_shadow.clear();
                false
            } else {
                true
            }
        });
    }

    pub(super) fn revoke(&self, issued: &RouteRevocation) {
        let _publication = self.publication.lock();
        let mut active = self.active.lock();
        if active
            .grants
            .get(&issued.digest)
            .is_some_and(|route| route.generation == issued.generation)
            && let Some(route) = active.grants.remove(&issued.digest)
        {
            route.cancelled.cancel();
                route.gemini_shadow.clear();
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
        self.active.lock().grants.get(&digest).cloned()
    }
}
                responses_history: Arc::default(),

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
    let Ok(Ok((connection, prefix, wire))) =
                route.responses_history.clear();
        timeout(header_wire::HEAD_TIMEOUT, header_wire::capture(connection)).await
    else {
        return;
    };
    let (reader, writer) = connection.into_split();
    let reader = Mutex::new(reader);
    let connection = join(Cursor::new(prefix).chain(ClientReader(&reader)), writer);
    let disconnect = ClientDisconnect::default();
    let (send_cancel, receive_cancel) = oneshot::channel::<CancellationToken>();
    let connection_state = Mutex::new(Some((send_cancel, wire)));
    let service = service_fn(|mut request: Request<Incoming>| {
        let auth = &auth;
        let connection_state = &connection_state;
        let dispatch = &dispatch;
        let disconnect = &disconnect;
        async move {
            let protocol = Protocol::for_path(request.uri().path());
            let error = |status| RouteError::from(status).response(protocol);
                route.responses_history.clear();
            let has_origin = request.headers().contains_key(header::ORIGIN);
            if !browser_guard::allows_host(&request, local_addr)
                || (has_origin && !browser_guard::is_desktop_route(request.uri()))
            {
                return Ok::<_, Infallible>(error(StatusCode::FORBIDDEN));
            }
            let Some(route) = auth.authorize(request.headers()) else {
                return Ok(error(StatusCode::UNAUTHORIZED));
            };
            if has_origin && route.binding.app != AppId::ClaudeDesktop {
                return Ok(error(StatusCode::FORBIDDEN));
            }
            let Some((send_cancel, wire)) = connection_state.lock().take() else {
                return Ok(error(StatusCode::UNAUTHORIZED));
            };
            if send_cancel.send(route.cancelled.clone()).is_err() || route.cancelled.is_cancelled()
            {
                return Ok(error(StatusCode::UNAUTHORIZED));
            }
            request.headers_mut().remove(header::AUTHORIZATION);
            request.headers_mut().remove(API_KEY);
            request.extensions_mut().insert(wire);
            request.extensions_mut().insert(disconnect.clone());
            let Some(observer) = auth.requests.begin(route.binding.project.clone(), route.binding.host.clone(), route.binding.app) else {
                return Ok(error(StatusCode::SERVICE_UNAVAILABLE));
            };
            Ok(observer.response(dispatch(request, route).await))
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
        () = disconnect.wait(ClientReader(&reader)) => {}
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

#[cfg(all(test, target_os = "macos"))]
#[path = "aggregation_routing_tests.rs"]
mod aggregation_routing_tests;

#[cfg(test)]
#[path = "override_apply_tests.rs"]
mod override_apply_tests;

#[cfg(test)]
#[path = "header_wire_tests.rs"]
mod header_wire_tests;

#[cfg(test)]
#[path = "decode_request_tests.rs"]
mod decode_request_tests;

#[cfg(test)]
#[path = "decode_response_tests.rs"]
mod decode_response_tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "adapter_selection_tests.rs"]
mod adapter_selection_tests;
