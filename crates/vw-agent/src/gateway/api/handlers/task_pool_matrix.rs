//! 任务池 Matrix 协同事件路由模块。

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use parking_lot::Mutex;
use serde::Deserialize;
use std::collections::{HashSet, VecDeque};
use std::sync::LazyLock;
use vw_api_types::task::{
    TaskPoolMatrixEventDto, TaskPoolMatrixEventsResponse, TaskPoolMatrixPublishRequest,
    TaskPoolMatrixPublishResponse,
};

use crate::app::agent::gateway::{ApiError, AppState};

#[cfg(feature = "channel-matrix")]
use crate::app::agent::channels::MatrixChannel;
#[cfg(feature = "channel-matrix")]
use vw_config_types::channels::chat::DEFAULT_CHANNEL_INSTANCE_ID;
#[cfg(feature = "channel-matrix")]
use vw_config_types::channels::{ChannelsConfig, MatrixConfig};

const TASK_POOL_MATRIX_EVENT_QUEUE_CAPACITY: usize = 2048;

static TASK_POOL_MATRIX_EVENTS: LazyLock<Mutex<TaskPoolMatrixEventQueue>> = LazyLock::new(|| {
    Mutex::new(TaskPoolMatrixEventQueue::new(TASK_POOL_MATRIX_EVENT_QUEUE_CAPACITY))
});

pub(crate) fn router() -> Router<AppState> {
    Router::new().route(
        "/task-pool/matrix/events",
        get(handle_api_task_pool_matrix_events).post(handle_api_task_pool_matrix_publish),
    )
}

#[derive(Debug, Deserialize)]
pub struct TaskPoolMatrixEventsQuery {
    #[serde(default)]
    pub after_cursor: Option<u64>,
}

#[derive(Debug, Clone)]
struct QueuedTaskPoolMatrixEvent {
    cursor: u64,
    event: TaskPoolMatrixEventDto,
}

#[derive(Debug)]
struct TaskPoolMatrixEventQueue {
    capacity: usize,
    next_cursor: u64,
    events: VecDeque<QueuedTaskPoolMatrixEvent>,
    event_ids: HashSet<String>,
}

impl TaskPoolMatrixEventQueue {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            next_cursor: 0,
            events: VecDeque::with_capacity(capacity),
            event_ids: HashSet::new(),
        }
    }

    fn push(&mut self, event: TaskPoolMatrixEventDto) -> u64 {
        if self.event_ids.contains(&event.event_id) {
            return self.next_cursor;
        }

        self.next_cursor = self.next_cursor.saturating_add(1);
        self.event_ids.insert(event.event_id.clone());
        self.events.push_back(QueuedTaskPoolMatrixEvent { cursor: self.next_cursor, event });

        while self.events.len() > self.capacity {
            if let Some(evicted) = self.events.pop_front() {
                self.event_ids.remove(&evicted.event.event_id);
            }
        }

        self.next_cursor
    }

    fn response_after(&self, after_cursor: Option<u64>) -> TaskPoolMatrixEventsResponse {
        let after_cursor = after_cursor.unwrap_or_default();
        let events = self
            .events
            .iter()
            .filter(|queued| queued.cursor > after_cursor)
            .map(|queued| queued.event.clone())
            .collect();

        TaskPoolMatrixEventsResponse { cursor: self.next_cursor, events }
    }

    #[cfg(test)]
    fn clear(&mut self) {
        self.next_cursor = 0;
        self.events.clear();
        self.event_ids.clear();
    }
}

pub async fn handle_api_task_pool_matrix_publish(
    State(state): State<AppState>,
    Json(body): Json<TaskPoolMatrixPublishRequest>,
) -> Result<Json<TaskPoolMatrixPublishResponse>, ApiError> {
    validate_task_pool_matrix_publish_request(&body)?;
    let matrix_event_id =
        publish_task_pool_matrix_event(&state, body.matrix_instance_id.as_deref(), &body.event)
            .await?;

    Ok(Json(TaskPoolMatrixPublishResponse {
        accepted: true,
        matrix_event_id: Some(matrix_event_id),
    }))
}

pub async fn handle_api_task_pool_matrix_events(
    Query(query): Query<TaskPoolMatrixEventsQuery>,
) -> Result<Json<TaskPoolMatrixEventsResponse>, ApiError> {
    Ok(Json(task_pool_matrix_events_response(query.after_cursor)))
}

pub(crate) fn validate_task_pool_matrix_publish_request(
    request: &TaskPoolMatrixPublishRequest,
) -> Result<(), ApiError> {
    if let Some(matrix_instance_id) = &request.matrix_instance_id
        && matrix_instance_id.trim().is_empty()
    {
        return Err(ApiError::bad_request("matrix_instance_id must not be empty"));
    }

    request
        .event
        .validate_for_matrix_room_event()
        .map_err(|error| ApiError::bad_request(format!("invalid task-pool Matrix event: {error}")))
}

pub(crate) fn task_pool_matrix_events_response(
    after_cursor: Option<u64>,
) -> TaskPoolMatrixEventsResponse {
    TASK_POOL_MATRIX_EVENTS.lock().response_after(after_cursor)
}

pub(crate) fn record_inbound_task_pool_matrix_event(event: TaskPoolMatrixEventDto) -> u64 {
    TASK_POOL_MATRIX_EVENTS.lock().push(event)
}

#[cfg(test)]
fn clear_task_pool_matrix_events_for_tests() {
    TASK_POOL_MATRIX_EVENTS.lock().clear();
}

#[cfg(feature = "channel-matrix")]
async fn publish_task_pool_matrix_event(
    state: &AppState,
    matrix_instance_id: Option<&str>,
    event: &TaskPoolMatrixEventDto,
) -> Result<String, ApiError> {
    let matrix_instance_id = matrix_instance_id.map(str::trim).filter(|value| !value.is_empty());
    let (matrix, transcription, config_dir) = {
        let config = state.config.lock();
        let matrix = matrix_task_pool_publish_config(&config.channels_config, matrix_instance_id)?;
        let transcription = config.transcription.clone();
        let config_dir = config.config_path.parent().map(std::path::PathBuf::from);
        (matrix, transcription, config_dir)
    };

    let channel = task_pool_matrix_channel(matrix, transcription, config_dir, matrix_instance_id);

    channel.send_task_pool_matrix_event(event).await.map_err(|error| {
        ApiError::internal(format!("Matrix task-pool event publish failed: {error}"))
    })
}

#[cfg(feature = "channel-matrix")]
fn task_pool_matrix_channel(
    matrix: MatrixConfig,
    transcription: crate::app::agent::config::TranscriptionConfig,
    config_dir: Option<std::path::PathBuf>,
    matrix_instance_id: Option<&str>,
) -> MatrixChannel {
    MatrixChannel::new_with_session_hint_instance_id_and_vibewindow_dir(
        matrix.homeserver,
        matrix.access_token,
        matrix.room_id,
        matrix.allowed_users,
        matrix.user_id,
        matrix.device_id,
        matrix_instance_id.map(ToString::to_string),
        config_dir,
    )
    .with_mention_only(matrix.mention_only)
    .with_transcription(transcription)
}

#[cfg(not(feature = "channel-matrix"))]
async fn publish_task_pool_matrix_event(
    _state: &AppState,
    _matrix_instance_id: Option<&str>,
    _event: &TaskPoolMatrixEventDto,
) -> Result<String, ApiError> {
    Err(ApiError::not_implemented("Matrix task-pool events require the channel-matrix feature"))
}

#[cfg(feature = "channel-matrix")]
fn matrix_task_pool_publish_config(
    channels_config: &ChannelsConfig,
    matrix_instance_id: Option<&str>,
) -> Result<MatrixConfig, ApiError> {
    let matrix_instance_id = matrix_instance_id.map(str::trim).filter(|value| !value.is_empty());

    let matrix = match matrix_instance_id {
        None => channels_config.matrix.clone().ok_or_else(|| {
            ApiError::not_implemented(
                "Matrix task-pool events require channels.matrix configuration",
            )
        })?,
        Some(DEFAULT_CHANNEL_INSTANCE_ID) => channels_config.matrix.clone().ok_or_else(|| {
            ApiError::bad_request("Matrix task-pool instance 'default' is not configured")
        })?,
        Some(instance_id) => channels_config
            .matrix_instances
            .iter()
            .find(|instance| instance.id == instance_id)
            .map(|instance| instance.config.clone())
            .ok_or_else(|| {
                ApiError::bad_request(format!(
                    "Matrix task-pool instance '{instance_id}' is not configured"
                ))
            })?,
    };

    if matrix_instance_id.is_some()
        && matrix.user_id.as_deref().map(str::trim).filter(|value| !value.is_empty()).is_none()
    {
        let instance_id = matrix_instance_id.unwrap_or(DEFAULT_CHANNEL_INSTANCE_ID);
        return Err(ApiError::bad_request(format!(
            "Matrix task-pool instance '{instance_id}' must configure user_id"
        )));
    }

    Ok(matrix)
}

#[cfg(test)]
#[path = "task_pool_matrix_tests.rs"]
mod task_pool_matrix_tests;
