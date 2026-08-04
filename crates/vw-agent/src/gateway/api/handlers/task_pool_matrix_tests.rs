use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use vw_api_types::task::{
    TaskPoolMatrixActorDto, TaskPoolMatrixEventDto, TaskPoolMatrixEventKindDto,
    TaskPoolMatrixPublishRequest, TaskPoolStatus,
};

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("{}".to_string())
    }
}

fn app_state(config: Config) -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    AppState {
        config: Arc::new(parking_lot::Mutex::new(config)),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        whatsapp: None,
        whatsapp_app_secret: None,
        linq: None,
        linq_signing_secret: None,
        nextcloud_talk: None,
        nextcloud_talk_webhook_secret: None,
        wati: None,
        qq: None,
        qq_webhook_enabled: false,
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_tool_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
    }
}

fn matrix_task_pool_event(event_id: &str, task_id: &str) -> TaskPoolMatrixEventDto {
    TaskPoolMatrixEventDto {
        version: 1,
        event_id: event_id.to_string(),
        task_id: task_id.to_string(),
        actor: TaskPoolMatrixActorDto {
            matrix_user_id: "@alice:matrix.example.com".to_string(),
            display_name: Some("Alice".to_string()),
        },
        occurred_at_ms: 1_000,
        kind: TaskPoolMatrixEventKindDto::TaskStatusChanged {
            from_status: Some(TaskPoolStatus::Pool),
            to_status: TaskPoolStatus::Pending,
        },
    }
}

#[tokio::test]
async fn task_pool_matrix_router_and_events_handler_return_queued_events() {
    clear_task_pool_matrix_events_for_tests();
    let _router: axum::Router<AppState> = router();

    let event = matrix_task_pool_event("event-handler", "task-handler");
    let cursor = record_inbound_task_pool_matrix_event(event.clone());
    let Json(response) =
        handle_api_task_pool_matrix_events(Query(TaskPoolMatrixEventsQuery { after_cursor: None }))
            .await
            .expect("events handler");

    assert_eq!(response.cursor, cursor);
    assert_eq!(response.events, vec![event]);
}

#[tokio::test]
async fn task_pool_matrix_publish_handler_rejects_before_network_without_matrix_config() {
    let state = app_state(Config::default());
    let error = handle_api_task_pool_matrix_publish(
        State(state),
        Json(TaskPoolMatrixPublishRequest {
            event: matrix_task_pool_event("event-no-config", "task-no-config"),
            matrix_instance_id: None,
        }),
    )
    .await
    .expect_err("missing matrix config should fail before send");

    assert_eq!(error.status, StatusCode::NOT_IMPLEMENTED);
    assert!(error.to_string().contains("Matrix task-pool events require"));
}

#[tokio::test]
async fn matrix_task_pool_publish_rejects_invalid_event_and_events_respects_cursor() {
    clear_task_pool_matrix_events_for_tests();

    let mut invalid = matrix_task_pool_event("event-invalid", "task-1");
    invalid.task_id.clear();
    let error = validate_task_pool_matrix_publish_request(&TaskPoolMatrixPublishRequest {
        event: invalid,
        matrix_instance_id: None,
    })
    .expect_err("invalid payload should be rejected before Matrix publish");

    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert!(error.to_string().contains("task_id"));

    let empty = task_pool_matrix_events_response(Some(42));
    assert_eq!(empty.cursor, 0);
    assert!(empty.events.is_empty());

    let first_cursor =
        record_inbound_task_pool_matrix_event(matrix_task_pool_event("event-1", "task-1"));
    let second_cursor =
        record_inbound_task_pool_matrix_event(matrix_task_pool_event("event-2", "task-2"));

    let response = task_pool_matrix_events_response(Some(first_cursor));

    assert_eq!(response.cursor, second_cursor);
    assert_eq!(response.events, vec![matrix_task_pool_event("event-2", "task-2")]);
}

#[test]
fn task_pool_matrix_queue_deduplicates_evicts_and_allows_reinsertion() {
    let mut queue = TaskPoolMatrixEventQueue::new(2);
    let event_1 = matrix_task_pool_event("event-1", "task-1");
    let event_2 = matrix_task_pool_event("event-2", "task-2");
    let event_3 = matrix_task_pool_event("event-3", "task-3");

    assert_eq!(queue.push(event_1.clone()), 1);
    assert_eq!(queue.push(event_1.clone()), 1);
    assert_eq!(queue.response_after(None).events, vec![event_1.clone()]);

    assert_eq!(queue.push(event_2.clone()), 2);
    assert_eq!(queue.push(event_3.clone()), 3);
    assert_eq!(queue.response_after(None).events, vec![event_2, event_3]);

    assert_eq!(queue.push(event_1.clone()), 4);
    assert_eq!(queue.response_after(Some(3)).events, vec![event_1]);
}

#[test]
fn task_pool_matrix_publish_validation_rejects_blank_instance_and_accepts_valid_event() {
    let blank = validate_task_pool_matrix_publish_request(&TaskPoolMatrixPublishRequest {
        event: matrix_task_pool_event("event-blank", "task-1"),
        matrix_instance_id: Some(" \t ".to_string()),
    })
    .expect_err("blank Matrix instance should be rejected");
    assert_eq!(blank.status, StatusCode::BAD_REQUEST);
    assert!(blank.to_string().contains("matrix_instance_id"));

    validate_task_pool_matrix_publish_request(&TaskPoolMatrixPublishRequest {
        event: matrix_task_pool_event("event-valid", "task-2"),
        matrix_instance_id: Some("planner".to_string()),
    })
    .expect("valid event and Matrix instance should be accepted");
}

#[cfg(feature = "channel-matrix")]
fn matrix_config(user_id: Option<&str>, access_token: &str, room_id: &str) -> MatrixConfig {
    MatrixConfig {
        homeserver: "https://matrix.example.test".to_string(),
        access_token: access_token.to_string(),
        user_id: user_id.map(ToString::to_string),
        device_id: Some("DEVICE".to_string()),
        room_id: room_id.to_string(),
        allowed_users: vec!["@alice:matrix.example.test".to_string()],
        mention_only: false,
    }
}

#[cfg(feature = "channel-matrix")]
#[test]
fn task_pool_matrix_instance_publish_uses_named_matrix_instance() {
    use vw_config_types::channels::{ChannelsConfig, MatrixInstanceConfig};

    let mut channels_config = ChannelsConfig {
        matrix_enabled: true,
        matrix: Some(matrix_config(
            Some("@legacy:matrix.example.test"),
            "legacy-token",
            "!legacy:matrix.example.test",
        )),
        ..ChannelsConfig::default()
    };
    channels_config.matrix_instances.push(MatrixInstanceConfig {
        id: "planner".to_string(),
        config: matrix_config(
            Some("@planner:matrix.example.test"),
            "planner-token",
            "!planner:matrix.example.test",
        ),
    });
    channels_config.matrix_instances.push(MatrixInstanceConfig {
        id: "missing-user".to_string(),
        config: matrix_config(None, "missing-user-token", "!missing-user:matrix.example.test"),
    });

    let legacy =
        matrix_task_pool_publish_config(&channels_config, None).expect("legacy config selected");
    assert_eq!(legacy.access_token, "legacy-token");
    assert_eq!(legacy.room_id, "!legacy:matrix.example.test");

    let planner = matrix_task_pool_publish_config(&channels_config, Some("planner"))
        .expect("named config selected");
    assert_eq!(planner.access_token, "planner-token");
    assert_eq!(planner.user_id.as_deref(), Some("@planner:matrix.example.test"));
    assert_eq!(planner.room_id, "!planner:matrix.example.test");

    let missing =
        matrix_task_pool_publish_config(&channels_config, Some("missing")).expect_err("missing");
    assert_eq!(missing.status, StatusCode::BAD_REQUEST);
    assert!(missing.to_string().contains("missing"));

    let default_missing_config =
        ChannelsConfig { matrix_enabled: true, ..ChannelsConfig::default() };
    let default_missing = matrix_task_pool_publish_config(
        &default_missing_config,
        Some("default"),
    )
    .expect_err("missing default");
    assert_eq!(default_missing.status, StatusCode::BAD_REQUEST);
    assert!(default_missing.to_string().contains("'default'"));

    let missing_user = matrix_task_pool_publish_config(&channels_config, Some("missing-user"))
        .expect_err("missing user");
    assert_eq!(missing_user.status, StatusCode::BAD_REQUEST);
    assert!(missing_user.to_string().contains("user_id"));
}

#[cfg(feature = "channel-matrix")]
#[test]
fn task_pool_matrix_publish_channel_scopes_store_by_named_instance_and_user() {
    let channel = task_pool_matrix_channel(
        matrix_config(Some("@planner:matrix.example.test"), "token", "!room:matrix.example.test"),
        crate::app::agent::config::TranscriptionConfig::default(),
        Some(std::path::PathBuf::from("/tmp/vibewindow")),
        Some(" planner "),
    );

    assert_eq!(
        channel.matrix_store_dir(),
        Some(std::path::PathBuf::from(
            "/tmp/vibewindow/state/matrix/planner/%40planner%3Amatrix.example.test"
        ))
    );
}
