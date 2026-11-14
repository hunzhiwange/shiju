//! Request-owned ledger observation. Bodies are inspected in bounded memory only.
use std::time::Instant;
use axum::{body::Body, response::Response};
use futures_util::StreamExt;
use serde_json::Value;
use vw_agent_apps::usage::{store::{self, StoredUsageRecord, UsageFailure, UsageSourceIdentity}, tokens::{parse_response, UsageProtocol}};
use vw_api_types::{id::ProjectScope, usage::*};
use crate::{global_database::GlobalDatabase, model_relay::{generation::GenerationProtocol, stream_observer::StreamObserver}};

pub(crate) struct RequestUsage {
    database: GlobalDatabase,
    scope: ProjectScope,
    identity: UsageSourceIdentity,
    value: Option<StoredUsageRecord>,
    started: Instant,
    protocol: GenerationProtocol,
    observer: StreamObserver,
    pending: Vec<u8>,
    sse: bool,
    invalid: bool,
}

impl RequestUsage {
    pub(crate) fn new(database: GlobalDatabase, scope: ProjectScope, host: String,
        app: String, provider: Option<String>, model: Option<String>, session: Option<String>,
        protocol: GenerationProtocol) -> anyhow::Result<Self> {
        anyhow::ensure!(!host.trim().is_empty() && !app.trim().is_empty(), "missing usage identity");
        // Verify persistence support before dispatch, never silently create another ledger.
        crate::global_database::run_sync_database(|| {
            let mut connection = database.connect_sqlite()?;
            store::initialize_usage_store(&mut connection)?;
            Ok(())
        })?;
        let id = uuid::Uuid::new_v4().to_string();
        let identity = UsageSourceIdentity { target_host: host, app: app.clone(),
            instance_id: "routing".into(), kind: UsageSourceKind::Relay,
            file_identity: "routing".into(), native_session_id: session.clone(), native_request_id: id.clone() };
        let record = UsageRecord {
            id: id.clone(), started_at_unix_seconds: UsageUnixSeconds(chrono::Utc::now().timestamp()),
            provider: provider.map_or(UsageProvider::Unknown, |provider_id| UsageProvider::Known { provider_id }),
            sources: vec![UsageSourceRef { source_id: id.clone(), kind: UsageSourceKind::Relay, app,
                native_request_id: Some(id.clone()), native_session_id: session }],
            models: UsageModels { request_model: model, response_model: None, pricing_model: None,
                pricing_multiplier: None, multiplier_basis: None },
            status: UsageRequestStatus::Unknown, billable: false, tokens: None,
            cost: UsageCost { source: UsageCostSource::Unknown, amount_usd: None },
            performance: UsagePerformance { first_token_ms: None, latency_ms: None, output_tokens_per_second: None },
            suspected_duplicate_of: None,
        };
        Ok(Self { database, scope, identity, value: Some(StoredUsageRecord { record,
            status_code: None, failure: None }), started: Instant::now(), protocol,
            observer: StreamObserver::new(protocol), pending: Vec::new(), sse: false, invalid: false })
    }

    pub(crate) fn unknown_provider(&mut self) {
        if let Some(value) = &mut self.value { value.record.provider = UsageProvider::Unknown; }
    }

    pub(crate) fn fail(&mut self, status: axum::http::StatusCode) {
        if let Some(value) = &mut self.value { value.status_code = Some(status.as_u16()); }
        let failure = match status {
            axum::http::StatusCode::GATEWAY_TIMEOUT | axum::http::StatusCode::REQUEST_TIMEOUT => UsageFailure::Timeout,
            axum::http::StatusCode::FORBIDDEN | axum::http::StatusCode::UNAUTHORIZED => UsageFailure::PermissionDenied,
            _ => UsageFailure::Upstream,
        };
        self.finish(UsageRequestStatus::Failed, Some(failure));
    }

    fn snapshot(&mut self, value: &Value) {
        let protocol = match self.protocol {
            GenerationProtocol::Messages => UsageProtocol::Claude,
            GenerationProtocol::Responses => UsageProtocol::Responses,
            GenerationProtocol::Gemini => UsageProtocol::Gemini,
            _ => UsageProtocol::OpenAi,
        };
        let value = value.get("response").or_else(|| value.get("message")).unwrap_or(value);
        let Some(parsed) = parse_response(protocol, value) else { return };
        let record = &mut self.value.as_mut().expect("active observation").record;
        record.billable |= parsed.has_billable_tokens();
        if let Some(model) = parsed.response_model { record.models.response_model = Some(model); }
        if let Some(id) = parsed.response_id {
            self.identity.native_request_id = id.clone();
            record.sources[0].native_request_id = Some(id);
        }
        let mut tokens = parsed.tokens;
        if let Some(previous) = record.tokens.take() {
            tokens.input = tokens.input.or(previous.input);
            tokens.output = tokens.output.or(previous.output);
            tokens.reasoning = tokens.reasoning.or(previous.reasoning);
            tokens.cache_read = tokens.cache_read.or(previous.cache_read);
            tokens.cache_creation = tokens.cache_creation.or(previous.cache_creation);
            tokens.cache_creation_1h = tokens.cache_creation_1h.or(previous.cache_creation_1h);
        }
        record.tokens = Some(tokens);
    }

    fn observe(&mut self, bytes: &[u8]) {
        if self.invalid || self.value.is_none() { return; }
        if self.sse {
            match self.observer.push(bytes) {
                Ok(update) if update.text_events > 0 => {
                    let elapsed = measured(self.started.elapsed().as_secs_f64() * 1000.0);
                    self.value.as_mut().unwrap().record.performance.first_token_ms.get_or_insert(elapsed);
                }
                Ok(_) => {},
                Err(_) => { self.invalid = true; return; }
            }
        }
        if self.pending.len().saturating_add(bytes.len()) > 4 * 1024 * 1024 {
            self.invalid = true;
            self.pending.clear();
            return;
        }
        self.pending.extend_from_slice(bytes);
        if self.sse {
            while let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
                let line: Vec<_> = self.pending.drain(..=end).collect();
                if let Some(data) = line.strip_prefix(b"data:") {
                    if let Ok(value) = serde_json::from_slice::<Value>(data) { self.snapshot(&value); }
                }
            }
        }
    }

    pub(crate) fn finish(&mut self, status: UsageRequestStatus, failure: Option<UsageFailure>) {
        let Some(mut value) = self.value.take() else { return };
        value.record.status = status;
        value.failure = failure;
        value.record.performance.latency_ms = Some(measured(self.started.elapsed().as_secs_f64() * 1000.0));
        let database = self.database.clone();
        let result = crate::global_database::run_sync_database(|| {
            let mut connection = database.connect_sqlite()?;
            store::save_usage_record(&mut connection, &self.scope, &self.identity, &value)?;
            Ok(())
        });
        if result.is_err() { tracing::error!("routing usage persistence failed"); }
    }

    pub(crate) fn response(mut self, response: Response) -> Response {
        let (parts, body) = response.into_parts();
        self.value.as_mut().unwrap().status_code = Some(parts.status.as_u16());
        if !parts.status.is_success() {
            self.finish(UsageRequestStatus::Failed, Some(UsageFailure::Upstream));
            return Response::from_parts(parts, body);
        }
        self.sse = parts.headers.get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()).is_some_and(|v| v.starts_with("text/event-stream"));
        let stream = futures_util::stream::unfold((body.into_data_stream(), self), |(mut stream, mut usage)| async move {
            if usage.value.is_none() { return None; }
            match stream.next().await {
                Some(Ok(bytes)) => { usage.observe(&bytes); Some((Ok::<_, axum::Error>(bytes), (stream, usage))) }
                Some(Err(error)) => {
                    let mut cause: &(dyn std::error::Error + 'static) = &error;
                    let mut failure = UsageFailure::Upstream;
                    loop {
                        if let Some(io) = cause.downcast_ref::<std::io::Error>() {
                            failure = match io.kind() {
                                std::io::ErrorKind::TimedOut => UsageFailure::Timeout,
                                std::io::ErrorKind::Interrupted => UsageFailure::Cancelled,
                                _ => UsageFailure::Upstream,
                            };
                            break;
                        }
                        match cause.source() { Some(next) => cause = next, None => break }
                    }
                    usage.finish(if matches!(failure, UsageFailure::Cancelled) {
                        UsageRequestStatus::Cancelled
                    } else { UsageRequestStatus::Failed }, Some(failure));
                    Some((Err(error), (stream, usage)))
                }
                None => {
                    if !usage.sse && !usage.invalid {
                        match serde_json::from_slice::<Value>(&usage.pending) {
                            Ok(value) => usage.snapshot(&value),
                            Err(_) => usage.invalid = true,
                        }
                    }
                    let success = !usage.invalid && (!usage.sse || usage.observer.finish().is_ok());
                    usage.finish(if success { UsageRequestStatus::Succeeded } else { UsageRequestStatus::Failed },
                        (!success).then_some(UsageFailure::Upstream));
                    None
                }
            }
        });
        Response::from_parts(parts, Body::from_stream(stream))
    }
}

impl Drop for RequestUsage {
    fn drop(&mut self) { self.finish(UsageRequestStatus::Cancelled, Some(UsageFailure::Cancelled)); }
}
fn measured(value: f64) -> UsageMeasurement { UsageMeasurement { value, kind: UsageMeasurementKind::Measured } }
