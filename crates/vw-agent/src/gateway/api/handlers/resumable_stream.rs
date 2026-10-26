//! 网关可恢复 SSE 运行存储。
//!
//! 运行按租户、流类型和客户端请求标识隔离。生产者写入的 JSON 事件会获得单调
//! SSE 序号，并保存在内存中供后续订阅从 `Last-Event-ID` 之后继续读取。

use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Response, Sse};
use futures_util::stream;
use sha2::{Digest, Sha256};
use tokio::sync::watch;
use vw_api_types::id::TenantScope;

use crate::app::agent::gateway::ApiError;

pub(super) const STREAM_REQUEST_ID_HEADER: &str = "x-vw-stream-request-id";
const LAST_EVENT_ID_HEADER: &str = "last-event-id";
const DEFAULT_TERMINAL_CAPACITY: usize = 128;
const DEFAULT_TERMINAL_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_REQUEST_ID_LEN: usize = 128;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) enum StreamKind {
    Chat,
    TaskPlanRun,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct StreamKey {
    tenant: TenantScope,
    kind: StreamKind,
    request_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum StreamOpenError {
    RequestConflict,
}

#[derive(Clone, Debug)]
pub(super) struct StoredEvent {
    pub(super) id: u64,
    pub(super) data: String,
}

#[derive(Debug)]
struct RunState {
    events: Vec<StoredEvent>,
    terminal: bool,
    completed_at: Option<Instant>,
    completion_order: Option<u64>,
    cancellation: Option<StreamCancellation>,
    cancellation_requested: bool,
}

#[derive(Clone)]
struct StreamCancellation(Arc<dyn Fn() + Send + Sync>);

impl fmt::Debug for StreamCancellation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("StreamCancellation(..)")
    }
}

impl StreamCancellation {
    fn invoke(&self) {
        (self.0)();
    }
}

#[derive(Debug)]
struct StreamRun {
    key: StreamKey,
    request_signature: String,
    state: Mutex<RunState>,
    changes: watch::Sender<u64>,
}

#[derive(Debug, Default)]
struct StoreState {
    runs: HashMap<StreamKey, Arc<StreamRun>>,
    next_completion_order: u64,
}

#[derive(Clone, Debug)]
pub(super) struct ResumableStreamStore {
    state: Arc<Mutex<StoreState>>,
    terminal_capacity: usize,
    terminal_ttl: Duration,
}

#[derive(Debug)]
pub(super) struct OpenedStream {
    pub(super) subscription: ResumableSubscription,
    pub(super) producer: Option<ResumableProducer>,
}

#[derive(Debug)]
pub(super) struct ResumableSubscription {
    run: Arc<StreamRun>,
    cursor: u64,
    changes: watch::Receiver<u64>,
}

#[derive(Debug)]
pub(super) struct ResumableProducer {
    store: ResumableStreamStore,
    run: Arc<StreamRun>,
}

impl ResumableStreamStore {
    pub(super) fn new(terminal_capacity: usize, terminal_ttl: Duration) -> Self {
        Self { state: Arc::new(Mutex::new(StoreState::default())), terminal_capacity, terminal_ttl }
    }

    pub(super) fn open(
        &self,
        tenant: TenantScope,
        kind: StreamKind,
        request_id: impl Into<String>,
        request_signature: String,
        after_event_id: u64,
    ) -> Result<OpenedStream, StreamOpenError> {
        let key = StreamKey { tenant, kind, request_id: request_id.into() };
        let mut state = lock_or_recover(&self.state);
        self.prune_locked(&mut state, Instant::now());

        if let Some(run) = state.runs.get(&key) {
            if run.request_signature != request_signature {
                return Err(StreamOpenError::RequestConflict);
            }
            return Ok(OpenedStream {
                subscription: ResumableSubscription {
                    run: Arc::clone(run),
                    cursor: after_event_id,
                    changes: run.changes.subscribe(),
                },
                producer: None,
            });
        }

        let (changes, _) = watch::channel(0);
        let run = Arc::new(StreamRun {
            key: key.clone(),
            request_signature,
            state: Mutex::new(RunState {
                events: Vec::new(),
                terminal: false,
                completed_at: None,
                completion_order: None,
                cancellation: None,
                cancellation_requested: false,
            }),
            changes,
        });
        state.runs.insert(key, Arc::clone(&run));

        Ok(OpenedStream {
            subscription: ResumableSubscription {
                run: Arc::clone(&run),
                cursor: after_event_id,
                changes: run.changes.subscribe(),
            },
            producer: Some(ResumableProducer { store: self.clone(), run }),
        })
    }

    fn complete(&self, run: &Arc<StreamRun>) {
        let mut store_state = lock_or_recover(&self.state);
        let Some(registered) = store_state.runs.get(&run.key) else {
            return;
        };
        if !Arc::ptr_eq(registered, run) {
            return;
        }

        store_state.next_completion_order = store_state.next_completion_order.saturating_add(1);
        let completion_order = store_state.next_completion_order;
        let mut run_state = lock_or_recover(&run.state);
        run_state.completion_order = Some(completion_order);
        drop(run_state);
        self.prune_locked(&mut store_state, Instant::now());
    }

    /// 请求停止一个仍在运行的可恢复流。
    ///
    /// 返回 `false` 表示目标流不存在；重复请求保持幂等并返回 `true`。
    pub(super) fn cancel(&self, tenant: TenantScope, kind: StreamKind, request_id: &str) -> bool {
        let key = StreamKey { tenant, kind, request_id: request_id.to_string() };
        let run = {
            let state = lock_or_recover(&self.state);
            state.runs.get(&key).cloned()
        };
        let Some(run) = run else {
            return false;
        };

        let cancellation = {
            let mut state = lock_or_recover(&run.state);
            if state.cancellation_requested {
                return true;
            }
            state.cancellation_requested = true;
            state.cancellation.clone()
        };
        if let Some(cancellation) = cancellation {
            cancellation.invoke();
        }
        true
    }

    fn prune_locked(&self, state: &mut StoreState, now: Instant) {
        state.runs.retain(|_, run| {
            let run_state = lock_or_recover(&run.state);
            run_state
                .completed_at
                .is_none_or(|completed_at| now.duration_since(completed_at) < self.terminal_ttl)
        });

        let mut terminal_runs = state
            .runs
            .iter()
            .filter_map(|(key, run)| {
                lock_or_recover(&run.state).completion_order.map(|order| (order, key.clone()))
            })
            .collect::<Vec<_>>();
        if terminal_runs.len() <= self.terminal_capacity {
            return;
        }

        terminal_runs.sort_unstable_by_key(|(order, _)| *order);
        let evict_count = terminal_runs.len() - self.terminal_capacity;
        for (_, key) in terminal_runs.into_iter().take(evict_count) {
            state.runs.remove(&key);
        }
    }
}

impl ResumableProducer {
    /// 注册显式停止回调。若停止请求先于注册到达，会在注册后立即触发。
    pub(super) fn install_cancellation(&self, cancel: impl Fn() + Send + Sync + 'static) {
        let cancellation = StreamCancellation(Arc::new(cancel));
        let invoke_now = {
            let mut state = lock_or_recover(&self.run.state);
            if state.terminal {
                return;
            }
            state.cancellation = Some(cancellation.clone());
            state.cancellation_requested
        };
        if invoke_now {
            cancellation.invoke();
        }
    }

    /// 发布已编码的 JSON 事件。
    ///
    /// 返回 `false` 只表示该运行已经到达终态；订阅断开不会停止生产者。
    pub(super) fn publish(&self, data: impl Into<String>, terminal: bool) -> bool {
        let mut state = lock_or_recover(&self.run.state);
        if state.terminal {
            return false;
        }

        let id = state.events.len() as u64 + 1;
        state.events.push(StoredEvent { id, data: data.into() });
        if terminal {
            state.terminal = true;
            state.completed_at = Some(Instant::now());
            // Chat turns can leave owned shell processes running after their
            // stream terminates. Retain their explicit stop hook until eviction.
            if self.run.key.kind != StreamKind::Chat {
                state.cancellation = None;
            }
        }
        drop(state);

        self.run.changes.send_replace(id);
        if terminal {
            self.store.complete(&self.run);
        }
        true
    }

    pub(super) fn is_terminal(&self) -> bool {
        lock_or_recover(&self.run.state).terminal
    }
}

impl ResumableSubscription {
    pub(super) async fn next(&mut self) -> Option<StoredEvent> {
        loop {
            {
                let state = lock_or_recover(&self.run.state);
                if let Some(event) = state.events.iter().find(|event| event.id > self.cursor) {
                    let event = event.clone();
                    self.cursor = event.id;
                    return Some(event);
                }
                if state.terminal {
                    return None;
                }
            }

            if self.changes.changed().await.is_err() {
                return None;
            }
        }
    }
}

pub(super) fn request_signature(serialized_request: &[u8]) -> String {
    hex::encode(Sha256::digest(serialized_request))
}

pub(super) fn stream_request_id(headers: &HeaderMap) -> Result<String, ApiError> {
    let Some(value) = headers.get(STREAM_REQUEST_ID_HEADER) else {
        return Ok(uuid::Uuid::new_v4().to_string());
    };
    let value = value
        .to_str()
        .map_err(|_| ApiError::bad_request("x-vw-stream-request-id 必须是有效文本"))?
        .trim();
    if value.is_empty() || value.len() > MAX_REQUEST_ID_LEN {
        return Err(ApiError::bad_request("x-vw-stream-request-id 必须为 1 到 128 字节的非空文本"));
    }
    Ok(value.to_string())
}

pub(super) fn last_event_id(headers: &HeaderMap) -> Result<u64, ApiError> {
    let Some(value) = headers.get(LAST_EVENT_ID_HEADER) else {
        return Ok(0);
    };
    value
        .to_str()
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| ApiError::bad_request("Last-Event-ID 必须是非负整数"))
}

pub(super) fn global_stream_store() -> &'static ResumableStreamStore {
    static STORE: OnceLock<ResumableStreamStore> = OnceLock::new();
    STORE.get_or_init(|| ResumableStreamStore::new(DEFAULT_TERMINAL_CAPACITY, DEFAULT_TERMINAL_TTL))
}

pub(super) fn sse_response(subscription: ResumableSubscription) -> Response {
    let stream = stream::unfold(subscription, |mut subscription| async move {
        subscription.next().await.map(|stored| {
            let event = Event::default().id(stored.id.to_string()).data(stored.data);
            (Ok::<Event, Infallible>(event), subscription)
        })
    });
    let mut response = Sse::new(stream).keep_alive(KeepAlive::new()).into_response();
    response.headers_mut().insert(header::CONNECTION, HeaderValue::from_static("keep-alive"));
    response
}

fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
#[path = "resumable_stream_tests.rs"]
mod resumable_stream_tests;
