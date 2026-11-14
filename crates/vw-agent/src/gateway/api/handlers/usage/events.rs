//! Authenticated SSE invalidations. Clients requery the named usage APIs.
use std::{collections::BTreeMap, convert::Infallible, path::PathBuf, time::Duration};
use axum::{Extension, Router, extract::Query, http::{HeaderMap, StatusCode},
    response::{IntoResponse, sse::{Event, KeepAlive, Sse}}, routing::get};
use vw_api_types::{ProjectScope, TenantScope, id::ProjectId};
use crate::usage_bridge::events as bridge;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Selector { project_id: ProjectId, target_host: String }

pub(super) fn router() -> Router {
    Router::new().route("/usage/events", get(subscribe))
}

async fn subscribe(
    Extension(tenant): Extension<TenantScope>, headers: HeaderMap,
    Query(selector): Query<Selector>,
) -> Result<axum::response::Response, StatusCode> {
    let scope = ProjectScope::new(tenant, selector.project_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from);
    let host = selector.target_host;
    let authorization_scope = scope.clone();
    let authorization_host = host.clone();
    let authorize = move || -> Result<String, StatusCode> {
        let principal = super::super::agent_apps::authorize_usage_events(
            policy.as_deref(), &headers, &authorization_scope, &authorization_host)?;
        if crate::usage_bridge::context::local_host().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? != authorization_host {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(principal)
    };
    let principal = authorize()?;
    let receiver = bridge::cache_channel().subscribe();
    let state = (receiver, None::<BTreeMap<String, u64>>, true, authorize, scope, host, principal);
    let stream = futures_util::stream::unfold(state,
        |(mut receiver, previous, initial, authorize, scope, host, principal)| async move {
            loop {
                tokio::time::sleep(Duration::from_millis(200)).await;
                // Revocation ends the stream before reading or publishing data.
                if authorize().ok().as_deref() != Some(principal.as_str()) { return None; }
                let read_scope = scope.clone();
                let read_host = host.clone();
                let current = match tokio::task::spawn_blocking(move ||
                    bridge::revisions(&read_scope, &read_host)).await {
                    Ok(Ok(value)) => value,
                    _ => return None,
                };
                let mut caches = BTreeMap::new();
                let mut recover = initial;
                loop {
                    match receiver.try_recv() {
                        Ok(change) if change.scope == scope && change.host == host
                            && change.principal == principal => {
                            caches.insert((change.app, change.provider), change.revision);
                        }
                        Ok(_) => {},
                        Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => recover = true,
                        Err(_) => break,
                    }
                }
                if !recover && previous.as_ref() == Some(&current) && caches.is_empty() { continue; }
                let payload = serde_json::json!({
                    "type": "usage-revision", "scope": scope, "target_host": host,
                    "statistics_revisions": current,
                    "cache_updates": caches.into_iter().map(|((app, provider), revision)|
                        serde_json::json!({"app": app, "provider_id": provider, "revision": revision}))
                        .collect::<Vec<_>>(),
                    "recover": recover,
                });
                let event = Event::default().data(payload.to_string());
                return Some((Ok::<_, Infallible>(event),
                    (receiver, Some(current), false, authorize, scope, host, principal)));
            }
        });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()).into_response())
}
