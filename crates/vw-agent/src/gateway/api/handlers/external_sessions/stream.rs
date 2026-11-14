//! Bounded NDJSON reads. Cancellation waits for the owned blocking parser to exit.
use super::{
    catalog::Policy,
    messages::{Failure, failure},
};
use axum::{
    Extension, Json,
    body::Body,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;
use vw_agent_apps::apps::lifecycle::operations::Operation;
use vw_api_types::{TenantScope, external_session::*};

#[derive(Clone, Default)]
pub(super) struct Reads(Arc<Mutex<HashMap<String, Entry>>>);
struct Entry {
    identity: ExternalSessionIdentity,
    principal: String,
    operation: Operation,
}
struct Subscription {
    receiver: mpsc::Receiver<SessionStreamFrame>,
    operation: Operation,
}
impl Drop for Subscription {
    fn drop(&mut self) {
        self.operation.cancel();
    }
}
fn authorize(
    policy: &Policy,
    tenant: &TenantScope,
    headers: &HeaderMap,
    request: &SessionStreamRequest,
) -> Result<String, Failure> {
    if request.identity.scope.tenant_scope() != tenant {
        return Err(failure(StatusCode::FORBIDDEN, ExternalSessionErrorCode::Forbidden));
    }
    if uuid::Uuid::parse_str(&request.read_id).is_err()
        || vw_agent_apps::sessions::identity::validate_identity(&request.identity).is_err()
    {
        return Err(failure(StatusCode::BAD_REQUEST, ExternalSessionErrorCode::InvalidInput));
    }
    let (principal, _, instances) = super::super::agent_apps::authorize_external_history(
        policy.0.as_deref(),
        headers,
        &request.identity.scope,
        &request.identity.target_host,
    )
    .map_err(|s| failure(s, ExternalSessionErrorCode::Forbidden))?;
    if !instances.iter().any(|i| {
        i.provider == request.identity.provider && i.instance_id == request.identity.instance_id
    }) {
        return Err(failure(StatusCode::FORBIDDEN, ExternalSessionErrorCode::Forbidden));
    }
    Ok(principal)
}
pub(super) async fn cancel(
    Extension(policy): Extension<Policy>,
    Extension(reads): Extension<Reads>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(request): Json<SessionStreamRequest>,
) -> Result<StatusCode, Failure> {
    let principal = authorize(&policy, &tenant, &headers, &request)?;
    let operation = {
        let store = reads.0.lock().map_err(|_| {
            failure(StatusCode::SERVICE_UNAVAILABLE, ExternalSessionErrorCode::StorageUnavailable)
        })?;
        match store.get(&request.read_id) {
            Some(entry) if entry.identity == request.identity && entry.principal == principal => {
                Some(entry.operation.clone())
            }
            Some(_) => {
                return Err(failure(StatusCode::FORBIDDEN, ExternalSessionErrorCode::Forbidden));
            }
            None => None,
        }
    };
    if let Some(operation) = operation {
        operation.cancel_and_wait().await;
    }
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn read(
    Extension(policy): Extension<Policy>,
    Extension(reads): Extension<Reads>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(request): Json<SessionStreamRequest>,
) -> Result<Response, Failure> {
    let principal = authorize(&policy, &tenant, &headers, &request)?;
    let operation = Operation::new();
    {
        let mut store = reads.0.lock().map_err(|_| {
            failure(StatusCode::SERVICE_UNAVAILABLE, ExternalSessionErrorCode::StorageUnavailable)
        })?;
        if store.contains_key(&request.read_id) {
            return Err(failure(StatusCode::CONFLICT, ExternalSessionErrorCode::Conflict));
        }
        if store.len() >= 32 {
            return Err(failure(StatusCode::TOO_MANY_REQUESTS, ExternalSessionErrorCode::Conflict));
        }
        store.insert(
            request.read_id.clone(),
            Entry { identity: request.identity.clone(), principal, operation: operation.clone() },
        );
    }
    let (sender, receiver) = mpsc::channel(2);
    let worker = operation.clone();
    let key = request.read_id.clone();
    operation
        .start_action(async move {
            let read_id = request.read_id;
            let started = std::time::Instant::now();
            let parser_operation = worker.clone();
            let parse = tokio::task::spawn_blocking(move || {
                super::messages::resolve_with_cancel(
                    policy,
                    tenant,
                    headers,
                    SessionMessagesRequest { identity: request.identity },
                    Some(&parser_operation),
                )
            });
            let result = parse.await;
            let outcome = async {
                if worker.is_cancelled() || sender.is_closed() {
                    return Err("operation cancelled".into());
                }
                let mut sequence = 0;
                match result {
                    Ok(Ok((session, transcript, cached))) => {
                        let turns = vw_agent_apps::sessions::turns::build_turn_index(
                            &vw_agent_apps::sessions::turns::project(
                                transcript.messages(),
                                session.identity.provider,
                            ),
                        );
                        send(
                            &sender,
                            &worker,
                            &read_id,
                            &mut sequence,
                            TranscriptChunk::Header {
                                total: transcript.messages().len(),
                                turns,
                                cached,
                                parse_ms: started.elapsed().as_millis() as u64,
                            },
                        )
                        .await?;
                        let mut start = 0;
                        while start < transcript.messages().len() {
                            let mut end = start;
                            let mut bytes = 0usize;
                            while end < transcript.messages().len() && end - start < 128 {
                                let next = transcript.message_bytes()[end];
                                if end > start && bytes.saturating_add(next) > 256 * 1024 {
                                    break;
                                }
                                bytes = bytes.saturating_add(next);
                                end += 1;
                            }
                            send(
                                &sender,
                                &worker,
                                &read_id,
                                &mut sequence,
                                TranscriptChunk::Messages {
                                    start,
                                    messages: transcript.messages()[start..end].to_vec(),
                                },
                            )
                            .await?;
                            start = end;
                        }
                        send(
                            &sender,
                            &worker,
                            &read_id,
                            &mut sequence,
                            TranscriptChunk::Done {
                                payload_bytes: transcript.approx_bytes() as u64,
                            },
                        )
                        .await
                    }
                    _ => {
                        send(
                            &sender,
                            &worker,
                            &read_id,
                            &mut sequence,
                            TranscriptChunk::Error {
                                message: "Session source is unavailable or changed; retry the read"
                                    .into(),
                            },
                        )
                        .await?;
                        Err("session read failed".into())
                    }
                }
            }
            .await;
            if let Ok(mut store) = reads.0.lock() {
                store.remove(&key);
            }
            outcome
        })
        .map_err(|_| {
            failure(StatusCode::SERVICE_UNAVAILABLE, ExternalSessionErrorCode::StorageUnavailable)
        })?;
    let stream = futures_util::stream::unfold(
        Subscription { receiver, operation },
        |mut subscription| async move {
            let frame = subscription.receiver.recv().await?;
            let bytes = serde_json::to_vec(&frame).map(|mut b| {
                b.push(b'\n');
                b
            });
            Some((bytes, subscription))
        },
    );
    Ok((
        [("content-type", "application/x-ndjson"), ("cache-control", "no-store")],
        Body::from_stream(stream),
    )
        .into_response())
}
async fn send(
    sender: &mpsc::Sender<SessionStreamFrame>,
    operation: &Operation,
    read_id: &str,
    sequence: &mut usize,
    chunk: TranscriptChunk,
) -> Result<(), String> {
    tokio::select! {
        biased;
        _ = operation.cancelled() => Err("operation cancelled".into()),
        result = sender.send(SessionStreamFrame { read_id: read_id.into(), sequence: *sequence, chunk }) => {
            result.map_err(|_| "operation cancelled".to_string())?;
            *sequence += 1;
            Ok(())
        }
    }
}
