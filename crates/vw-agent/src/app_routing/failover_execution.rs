//! Execute the published ordinary route with one request-wide send budget.
use super::{
    SharedTransport,
    dispatch::{self, JsonTarget},
    header_wire::HeaderWire,
    provider_publication::PublishedProvider,
    route_auth::AuthorizedRoute,
};
use crate::model_relay::generation::GenerationTransport;
use crate::{
    app_providers::{AppProviderStore, generation_target::SavedProfile},
    gateway::{
        agent_apps_failover_runtime::{self, FailoverRuntime},
        agent_apps_high_availability_store::{FailoverQueueState, FailoverQueueStore},
    },
};
use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use futures_util::StreamExt;
use serde_json::Value;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use vw_agent_apps::high_availability::{
    circuit_breaker::{AttemptOutcome, OwnedCircuitAttempt},
    health::{HealthAccount, HealthKey},
    retry::{FailureCode, FailurePhase, ReplayEvidence, RetryBudget, RetryFailure, RetryStop},
    selection::{SelectionSnapshot, select},
};
use vw_api_types::{agent_apps::AppId, agent_apps_high_availability::CircuitBreakerPolicy};

struct Candidate {
    key: HealthKey,
    row: Value,
    target: JsonTarget,
    messages_api_key: bool,
}

pub(super) struct Plan {
    queue: FailoverQueueState,
    candidates: Vec<Candidate>,
    max_retries: u8,
    policy: CircuitBreakerPolicy,
    actual_version: u64,
    correlation_id: String,
}

fn unavailable<T>(_: T) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}

pub(super) async fn prepare(route: Arc<AuthorizedRoute>) -> Result<Option<Plan>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let Some(provider) = &route.provider else {
            return Ok(None);
        };
        provider.check_current()?;
        let queue = FailoverQueueStore::new(provider.database.clone())
            .map_err(unavailable)?
            .get(&provider.scope)
            .map_err(unavailable)?;
        if !queue.enabled {
            return Ok(None);
        }
        let store = AppProviderStore::new(provider.database.clone()).map_err(unavailable)?;
        let providers = store
            .failover_providers(&provider.scope.project, provider.scope.app_id)
            .map_err(unavailable)?;
        let current = providers
            .iter()
            .find(|entry| entry.provider.id == provider.provider_id)
            .ok_or(StatusCode::CONFLICT)?;
        // Credential ownership, rather than a UI category or stale queue flag,
        // decides admission. Official requests retain the original target and
        // its account-bound state without constructing a replay budget.
        match current.auth {
            vw_agent_apps::providers::failover::ProviderAuth::OfficialAccount => return Ok(None),
            vw_agent_apps::providers::failover::ProviderAuth::Unknown => {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            vw_agent_apps::providers::failover::ProviderAuth::ThirdParty => {}
        }
        if provider.scope.app_id != AppId::Claude {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let actual = route.actual_route.lock().clone();
        Plan::prepare(provider, &route.failover, queue, actual).map(Some)
    })
    .await
    .map_err(unavailable)?
}

impl Plan {
    fn prepare(
        provider: &PublishedProvider,
        runtime: &FailoverRuntime,
        queue: FailoverQueueState,
        actual: agent_apps_failover_runtime::ActualRoute,
    ) -> Result<Self, StatusCode> {
        let scope = &provider.scope;
        let policy = agent_apps_failover_runtime::policy(provider.database.clone(), scope)
            .map_err(unavailable)?;
        let store = AppProviderStore::new(provider.database.clone()).map_err(unavailable)?;
        let configuration =
            store.failover_configuration(&scope.project, scope.app_id).map_err(unavailable)?;
        let rows = configuration
            .iter()
            .map(|entry| (entry.provider.provider.id.clone(), entry.preimage.payload_json.clone()))
            .collect::<std::collections::HashMap<_, _>>();
        let providers = configuration.into_iter().map(|entry| entry.provider).collect::<Vec<_>>();
        let key = |id: &str| HealthKey {
            project: scope.project.clone(),
            target_host_id: scope.target_host_id.clone(),
            app_id: scope.app_id,
            provider_id: id.into(),
            account: HealthAccount::Unbound,
        };
        let now = std::time::Instant::now();
        let mut selected = select(
            &SelectionSnapshot {
                app: scope.app_id,
                tenant: scope.project.tenant_scope(),
                state: &provider.state,
                enabled: queue.enabled,
                members: &queue.members,
                providers: &providers,
            },
            |id| runtime.health.is_available(&key(id), &policy.circuit_breaker, now),
        )
        .map_err(unavailable)?;
        // Keep the successful resident target first without changing queue
        // priority, the configured provider, or its native preimage proof.
        if let Some(id) = &actual.provider_id
            && let Some(index) = selected.iter().position(|entry| &entry.provider.id == id)
        {
            let current = selected.remove(index);
            selected.insert(0, current);
        }
        let mut candidates = Vec::new();
        for entry in selected {
            let id = &entry.provider.id;
            let row: Value = serde_json::from_str(rows.get(id).ok_or(StatusCode::CONFLICT)?)
                .map_err(unavailable)?;
            let profile = SavedProfile::read_for_host(
                scope.app_id,
                &scope.target_host_id,
                &id.clone().into(),
                &row,
            )
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            if profile.protocol != crate::model_relay::generation::GenerationProtocol::Messages {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            let url = profile.protocol.url(
                &profile.endpoint,
                "route",
                row.pointer("/meta/isFullUrl") == Some(&Value::Bool(true)),
            )?;
            let headers = profile
                .protocol
                .headers(&profile.credential, profile.messages_bearer)
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            let authorization = headers
                .get(header::AUTHORIZATION)
                .or_else(|| headers.get("x-api-key"))
                .cloned()
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            let request_options = row
                .get("requestOptions")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?
                .unwrap_or_default();
            vw_agent_apps::providers::validation::validate(scope.app_id, &request_options)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            let target = JsonTarget {
                managed: None,
                url,
                authorization,
                transport: SharedTransport::new().map_err(unavailable)?,
                request_options,
            };
            crate::model_relay::upstream::validate_target(
                &target.url,
                &reqwest::Request::new(reqwest::Method::POST, target.url.clone()),
            )?;
            candidates.push(Candidate {
                key: key(id),
                row,
                target,
                messages_api_key: !profile.messages_bearer,
            });
        }
        Ok(Self {
            queue,
            candidates,
            max_retries: policy.retry_timeout.max_retries,
            policy: policy.circuit_breaker,
            actual_version: actual.version,
            correlation_id: uuid::Uuid::new_v4().to_string(),
        })
    }

    pub(super) async fn forward(
        &self,
        request: AttemptRequest<'_>,
    ) -> Result<Response, StatusCode> {
        let app = request.route.json_app().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let runtime = &request.route.failover;
        let cancelled = &request.route.cancelled;
        let mut budget =
            RetryBudget::new(self.max_retries, Duration::from_millis(500), request.deadline)
                .map_err(unavailable)?;
        let mut failure = None;
        let mut last_status = StatusCode::SERVICE_UNAVAILABLE;
        let mut next = 0;
        loop {
            let now = std::time::Instant::now();
            // Preflight skips do not consume a send. Visit each candidate at most
            // once per admission, then stop if every circuit is unavailable.
            let chosen = (0..self.candidates.len()).find_map(|_| {
                let candidate = &self.candidates[next % self.candidates.len()];
                next += 1;
                let circuit = runtime.health.circuit(&candidate.key, now)?;
                let permit = circuit.allow_owned_request(&self.policy, now)?;
                Some((candidate, permit))
            });
            let Some((candidate, permit)) = chosen else {
                return Err(last_status);
            };
            if let Err(stop) = budget.begin_attempt(failure, cancelled.cancelled()).await {
                return Err(match stop {
                    RetryStop::Cancelled => StatusCode::REQUEST_TIMEOUT,
                    RetryStop::DeadlineExceeded => StatusCode::GATEWAY_TIMEOUT,
                    RetryStop::Exhausted | RetryStop::Rejected(_) => last_status,
                    RetryStop::InvalidMaxRetries | RetryStop::InvalidAttemptSequence => {
                        StatusCode::SERVICE_UNAVAILABLE
                    }
                });
            }
            // Recheck native ownership and both saved preimages after backoff.
            let route = Arc::clone(request.route);
            let expected_queue = self.queue.clone();
            let expected_row = candidate.row.clone();
            let id = candidate.key.provider_id.clone();
            tokio::task::spawn_blocking(move || {
                let provider = route.provider.as_ref().ok_or(StatusCode::CONFLICT)?;
                provider.check_current()?;
                let queue = FailoverQueueStore::new(provider.database.clone())
                    .map_err(unavailable)?
                    .get(&provider.scope)
                    .map_err(unavailable)?;
                let row = AppProviderStore::preview_record(
                    &provider.database,
                    provider.scope.project.tenant_scope(),
                    provider.scope.app_id,
                    id.as_ref(),
                )
                .map_err(unavailable)?;
                if queue != expected_queue || row != expected_row {
                    return Err(StatusCode::CONFLICT);
                }
                Ok(())
            })
            .await
            .map_err(unavailable)??;
            let model = request.body.get("model").and_then(Value::as_str).map(|model| {
                vw_agent_apps::routing::target_model::resolve(
                    app,
                    candidate.row.get("settingsConfig").unwrap_or(&Value::Null),
                    model,
                )
            });
            let model = model.as_deref().filter(|mapped| {
                Some(*mapped) != request.body.get("model").and_then(Value::as_str)
            });
            let mut outgoing = dispatch::prepare_request(request.body, &candidate.target, model)?;
            let mut wire = request.wire.clone();
            wire.retain_allowed(request.headers, outgoing.headers_mut());
            if candidate.messages_api_key
                && let Some(key) = outgoing.headers_mut().remove(header::AUTHORIZATION)
            {
                outgoing.headers_mut().insert("x-api-key", key);
            }
            outgoing
                .headers_mut()
                .entry("anthropic-version")
                .or_insert(axum::http::HeaderValue::from_static("2023-06-01"));
            if request.body.get("stream") == Some(&Value::Bool(true)) {
                outgoing.headers_mut().insert(
                    header::ACCEPT_ENCODING,
                    axum::http::HeaderValue::from_static("identity"),
                );
            }
            let transport = request.transport.unwrap_or(&candidate.target.transport);
            let result = super::timeouts::before(
                request.deadline,
                transport.forward_attempt(
                    &candidate.target.url,
                    outgoing,
                    wire,
                    cancelled.clone(),
                    request.listener,
                ),
            )
            .await;
            match result {
                Ok(Ok(response)) => {
                    if response.status().is_success() {
                        let response = observe(
                            response,
                            permit,
                            cancelled.clone(),
                            (request.body.get("stream") == Some(&Value::Bool(true)))
                                .then_some(request.deadline),
                        )
                        .await?;
                        self.publish_actual(request.route, candidate, failure.is_some()).await?;
                        return Ok(response);
                    }
                    let status = response.status();
                    permit.record(
                        if cancelled.is_cancelled() {
                            AttemptOutcome::Neutral
                        } else if matches!(status.as_u16(), 429 | 500..=599) {
                            AttemptOutcome::ProviderFailure
                        } else {
                            AttemptOutcome::Neutral
                        },
                        std::time::Instant::now(),
                    );
                    // A status never proves that the provider did not execute.
                    return Ok(response);
                }
                Ok(Err(error)) => {
                    permit.record(
                        if cancelled.is_cancelled() {
                            AttemptOutcome::Neutral
                        } else {
                            AttemptOutcome::ProviderFailure
                        },
                        std::time::Instant::now(),
                    );
                    last_status = error.status;
                    failure = Some(RetryFailure {
                        phase: if error.before_send {
                            FailurePhase::Connect
                        } else {
                            FailurePhase::RequestSend
                        },
                        code: FailureCode::Connection,
                        replay: if error.before_send {
                            ReplayEvidence::ProvenSafe
                        } else {
                            ReplayEvidence::Unknown
                        },
                    });
                }
                Err(status) => {
                    permit.record(
                        if cancelled.is_cancelled() {
                            AttemptOutcome::Neutral
                        } else {
                            AttemptOutcome::ProviderFailure
                        },
                        std::time::Instant::now(),
                    );
                    return Err(status);
                }
            }
        }
    }

    async fn publish_actual(
        &self,
        route: &Arc<AuthorizedRoute>,
        candidate: &Candidate,
        connection_failed: bool,
    ) -> Result<(), StatusCode> {
        let route = Arc::clone(route);
        let queue = self.queue.clone();
        let row = candidate.row.clone();
        let id = candidate.key.provider_id.clone();
        let expected = self.actual_version;
        let correlation_id = self.correlation_id.clone();
        tokio::task::spawn_blocking(move || {
            let provider = route.provider.as_ref().ok_or(StatusCode::CONFLICT)?;
            // Do not acquire the native lock under the publication lock: mode
            // changes take those locks in the opposite order.
            provider.check_current()?;
            let _publication = route.publication.lock();
            if route.cancelled.is_cancelled() {
                return Err(StatusCode::CONFLICT);
            }
            if FailoverQueueStore::new(provider.database.clone())
                .map_err(unavailable)?
                .get(&provider.scope)
                .map_err(unavailable)?
                != queue
                || AppProviderStore::preview_record(
                    &provider.database,
                    provider.scope.project.tenant_scope(),
                    provider.scope.app_id,
                    id.as_ref(),
                )
                .map_err(unavailable)?
                    != row
            {
                return Err(StatusCode::CONFLICT);
            }
            let mut actual = route.actual_route.lock();
            let previous = actual
                .commit(expected, id.as_ref(), &provider.provider_id)
                .map_err(unavailable)?;
            if let Some(previous) = previous {
                // Serialize state and event publication together. A late
                // request from an old version cannot roll back this target.
                if let Some(events) = route.failover.events.read().as_ref() {
                    let reason = if connection_failed {
                        vw_api_types::agent_apps_high_availability::FailoverFailureReason::Connection
                    } else {
                        vw_api_types::agent_apps_high_availability::FailoverFailureReason::Other
                    };
                    let _ = events.send(serde_json::json!({
                        "type": "provider-switched",
                        "source": "failover",
                        "scope": provider.scope,
                        "configuredProviderId": provider.provider_id,
                        "previousProviderId": previous,
                        "actualProviderId": id,
                        "reason": reason,
                        "correlationId": correlation_id,
                        "generation": route.generation(),
                        "version": actual.version,
                        "queueVersion": queue.version,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }));
                }
            }
            Ok(())
        })
        .await
        .map_err(unavailable)?
    }
}

pub(super) struct AttemptRequest<'a> {
    pub(super) route: &'a Arc<AuthorizedRoute>,
    pub(super) body: &'a Value,
    pub(super) headers: &'a HeaderMap,
    pub(super) wire: &'a HeaderWire,
    pub(super) transport: Option<&'a SharedTransport>,
    pub(super) listener: SocketAddr,
    pub(super) deadline: Instant,
}

async fn observe(
    response: Response,
    permit: OwnedCircuitAttempt,
    cancel: CancellationToken,
    first_byte_deadline: Option<Instant>,
) -> Result<Response, StatusCode> {
    use crate::model_relay::{generation::GenerationProtocol, stream_observer::StreamObserver};
    let response = if let Some(deadline) = first_byte_deadline {
        let (parts, body) = response.into_parts();
        let result = async {
            let codings = super::decode_request::ContentCodings::from_headers(&parts.headers)
                .map_err(|_| super::protocol_errors::RouteError::from(StatusCode::BAD_GATEWAY))?;
            if !codings.is_identity()
                || !dispatch::is_content_type(&parts.headers, "text/event-stream")
            {
                return Err(StatusCode::BAD_GATEWAY.into());
            }
            super::timeouts::first_output_body(
                body,
                deadline,
                super::protocol_errors::Protocol::Messages,
            )
            .await
        }
        .await;
        match result {
            Ok(body) => Response::from_parts(parts, body),
            Err(error) => {
                permit.record(
                    if cancel.is_cancelled() {
                        AttemptOutcome::Neutral
                    } else {
                        AttemptOutcome::ProviderFailure
                    },
                    std::time::Instant::now(),
                );
                // After send, a missing event does not prove that execution was
                // safe to replay. Return directly without visiting a backup.
                return Err(error.status());
            }
        }
    } else {
        response
    };
    let observer =
        first_byte_deadline.is_some().then(|| StreamObserver::new(GenerationProtocol::Messages));
    Ok(response.map(|body| {
        Body::from_stream(futures_util::stream::unfold(
            (body.into_data_stream(), Some(permit), cancel, observer),
            |(mut stream, mut permit, cancel, mut observer)| async move {
                match stream.next().await {
                    Some(chunk) => {
                        let outcome = match &chunk {
                            Err(_) => Some(AttemptOutcome::ProviderFailure),
                            Ok(bytes) => {
                                match observer.as_mut().map(|observer| observer.push(bytes)) {
                                    Some(Ok(update)) if update.completed => {
                                        Some(AttemptOutcome::Success)
                                    }
                                    Some(Err(_)) => Some(AttemptOutcome::ProviderFailure),
                                    _ => None,
                                }
                            }
                        };
                        if let Some(outcome) = outcome
                            && let Some(permit) = permit.take()
                        {
                            permit.record(
                                if cancel.is_cancelled() {
                                    AttemptOutcome::Neutral
                                } else {
                                    outcome
                                },
                                std::time::Instant::now(),
                            );
                        }
                        Some((chunk, (stream, permit, cancel, observer)))
                    }
                    None => {
                        if let Some(permit) = permit.take() {
                            let outcome = if cancel.is_cancelled() {
                                AttemptOutcome::Neutral
                            } else if observer
                                .as_ref()
                                .is_some_and(|observer| observer.finish().is_err())
                            {
                                AttemptOutcome::ProviderFailure
                            } else {
                                AttemptOutcome::Success
                            };
                            permit.record(outcome, std::time::Instant::now());
                        }
                        None
                    }
                }
            },
        ))
    }))
}

#[cfg(test)]
#[path = "failover_execution_tests.rs"]
mod tests;
