// 覆盖记忆列表、检索、写入和删除接口的认证与错误映射。

use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::{Memory, MemoryCategory, MemoryEntry};
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use async_trait::async_trait;
use axum::body::to_bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

// 记录型内存后端同时保存调用参数与结果，用于区分列表、召回和删除分支。
#[test]
fn exported_handlers_are_available() {
    let _ = handle_api_memory_list;
    let _ = handle_api_memory_store;
    let _ = handle_api_memory_delete;
}

#[derive(Default)]
struct RecordingMemory {
    fail_recall: bool,
    fail_list: bool,
    fail_store: bool,
    fail_forget: bool,
    recalled: Mutex<Vec<(String, usize)>>,
    listed: Mutex<Vec<Option<MemoryCategory>>>,
    stored: Mutex<Vec<(String, String, MemoryCategory)>>,
    forgotten: Mutex<Vec<String>>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Memory for RecordingMemory {
    fn name(&self) -> &str {
        "recording"
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        if self.fail_store {
            anyhow::bail!("store boom");
        }
        self.stored.lock().push((key.to_string(), content.to_string(), category));
        Ok(())
    }

    async fn recall(
        &self,
        query: &str,
        limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        if self.fail_recall {
            anyhow::bail!("recall boom");
        }
        self.recalled.lock().push((query.to_string(), limit));
        Ok(vec![entry("recall", query, MemoryCategory::Core)])
    }

    async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(None)
    }

    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        if self.fail_list {
            anyhow::bail!("list boom");
        }
        self.listed.lock().push(category.cloned());
        Ok(vec![entry("list", "listed content", category.cloned().unwrap_or(MemoryCategory::Core))])
    }

    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        if self.fail_forget {
            anyhow::bail!("forget boom");
        }
        self.forgotten.lock().push(key.to_string());
        Ok(key == "known")
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(0)
    }

    async fn health_check(&self) -> bool {
        true
    }
}

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(message.to_string())
    }
}

fn entry(key: &str, content: &str, category: MemoryCategory) -> MemoryEntry {
    MemoryEntry {
        id: format!("id-{key}"),
        key: key.to_string(),
        content: content.to_string(),
        category,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        session_id: None,
        score: Some(0.5),
    }
}

fn state_with_memory(mem: Arc<RecordingMemory>, require_pairing: bool) -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    AppState {
        config: Arc::new(parking_lot::Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem,
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(require_pairing, &[])),
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
        max_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

async fn response_json(response: Response) -> (StatusCode, serde_json::Value) {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    let value = serde_json::from_slice(&bytes).expect("response should be json");
    (status, value)
}

#[tokio::test]
async fn memory_list_requires_auth_when_pairing_enabled() {
    let mem = Arc::new(RecordingMemory::default());
    let state = state_with_memory(mem.clone(), true);

    let (status, value) = response_json(
        handle_api_memory_list(
            State(state),
            HeaderMap::new(),
            Query(MemoryQuery { query: None, category: None }),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(value["error"].as_str().unwrap_or_default().contains("Unauthorized"));
    assert!(mem.recalled.lock().is_empty());
    assert!(mem.listed.lock().is_empty());
}

#[tokio::test]
async fn memory_list_uses_recall_when_query_is_present() {
    let mem = Arc::new(RecordingMemory::default());
    let state = state_with_memory(mem.clone(), false);

    let (status, value) = response_json(
        handle_api_memory_list(
            State(state),
            HeaderMap::new(),
            Query(MemoryQuery { query: Some("rust".to_string()), category: None }),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["entries"][0]["content"], "rust");
    assert_eq!(mem.recalled.lock().as_slice(), &[("rust".to_string(), 50)]);
}

#[tokio::test]
async fn memory_list_treats_empty_query_as_recall_and_ignores_category() {
    let mem = Arc::new(RecordingMemory::default());
    let state = state_with_memory(mem.clone(), false);

    let (status, value) = response_json(
        handle_api_memory_list(
            State(state),
            HeaderMap::new(),
            Query(MemoryQuery { query: Some(String::new()), category: Some("daily".to_string()) }),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["entries"][0]["content"], "");
    assert_eq!(mem.recalled.lock().as_slice(), &[(String::new(), 50)]);
    assert!(mem.listed.lock().is_empty());
}

#[tokio::test]
async fn memory_list_without_filters_lists_all_entries() {
    let mem = Arc::new(RecordingMemory::default());
    let state = state_with_memory(mem.clone(), false);

    let (status, value) = response_json(
        handle_api_memory_list(
            State(state),
            HeaderMap::new(),
            Query(MemoryQuery { query: None, category: None }),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["entries"][0]["key"], "list");
    assert_eq!(mem.listed.lock().as_slice(), &[None]);
    assert!(mem.recalled.lock().is_empty());
}

#[tokio::test]
async fn memory_list_maps_builtin_and_custom_categories() {
    for (raw, expected) in [
        ("core", MemoryCategory::Core),
        ("daily", MemoryCategory::Daily),
        ("conversation", MemoryCategory::Conversation),
        ("project", MemoryCategory::Custom("project".to_string())),
    ] {
        let mem = Arc::new(RecordingMemory::default());
        let state = state_with_memory(mem.clone(), false);

        let (status, _value) = response_json(
            handle_api_memory_list(
                State(state),
                HeaderMap::new(),
                Query(MemoryQuery { query: None, category: Some(raw.to_string()) }),
            )
            .await
            .into_response(),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(mem.listed.lock().as_slice(), &[Some(expected)]);
    }
}

#[tokio::test]
async fn memory_store_requires_auth_without_writing() {
    let mem = Arc::new(RecordingMemory::default());
    let state = state_with_memory(mem.clone(), true);

    let (status, value) = response_json(
        handle_api_memory_store(
            State(state),
            HeaderMap::new(),
            JsonResponse(MemoryStoreBody {
                key: "secret".to_string(),
                content: "must not be stored".to_string(),
                category: Some("core".to_string()),
            }),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(value["error"].as_str().unwrap_or_default().contains("Unauthorized"));
    assert!(mem.stored.lock().is_empty());
}

#[tokio::test]
async fn memory_store_defaults_to_core_and_accepts_custom_category() {
    let mem = Arc::new(RecordingMemory::default());
    let state = state_with_memory(mem.clone(), false);

    let (status, value) = response_json(
        handle_api_memory_store(
            State(state.clone()),
            HeaderMap::new(),
            JsonResponse(MemoryStoreBody {
                key: "pref".to_string(),
                content: "likes rust".to_string(),
                category: None,
            }),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["status"], "ok");

    let (status, _value) = response_json(
        handle_api_memory_store(
            State(state),
            HeaderMap::new(),
            JsonResponse(MemoryStoreBody {
                key: "note".to_string(),
                content: "custom".to_string(),
                category: Some("project".to_string()),
            }),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        mem.stored.lock().as_slice(),
        &[
            ("pref".to_string(), "likes rust".to_string(), MemoryCategory::Core),
            (
                "note".to_string(),
                "custom".to_string(),
                MemoryCategory::Custom("project".to_string())
            )
        ]
    );
}

#[tokio::test]
async fn memory_store_maps_all_explicit_builtin_categories() {
    let mem = Arc::new(RecordingMemory::default());
    let state = state_with_memory(mem.clone(), false);

    for category in ["core", "daily", "conversation"] {
        let (status, value) = response_json(
            handle_api_memory_store(
                State(state.clone()),
                HeaderMap::new(),
                JsonResponse(MemoryStoreBody {
                    key: category.to_string(),
                    content: format!("{category} content"),
                    category: Some(category.to_string()),
                }),
            )
            .await
            .into_response(),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(value["status"], "ok");
    }

    assert_eq!(
        mem.stored.lock().as_slice(),
        &[
            ("core".to_string(), "core content".to_string(), MemoryCategory::Core,),
            ("daily".to_string(), "daily content".to_string(), MemoryCategory::Daily,),
            (
                "conversation".to_string(),
                "conversation content".to_string(),
                MemoryCategory::Conversation,
            ),
        ]
    );
}

#[tokio::test]
async fn memory_delete_requires_auth_without_forgetting() {
    let mem = Arc::new(RecordingMemory::default());
    let state = state_with_memory(mem.clone(), true);

    let (status, value) = response_json(
        handle_api_memory_delete(State(state), HeaderMap::new(), Path("known".to_string()))
            .await
            .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(value["error"].as_str().unwrap_or_default().contains("Unauthorized"));
    assert!(mem.forgotten.lock().is_empty());
}

#[tokio::test]
async fn memory_delete_returns_deleted_flag() {
    let mem = Arc::new(RecordingMemory::default());
    let state = state_with_memory(mem.clone(), false);

    let (status, value) = response_json(
        handle_api_memory_delete(State(state), HeaderMap::new(), Path("known".to_string()))
            .await
            .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["deleted"], true);
    assert_eq!(mem.forgotten.lock().as_slice(), &["known".to_string()]);
}

// 拒绝路径除状态码外还检查状态保持不变，防止错误响应掩盖已发生的副作用。
#[tokio::test]
async fn memory_delete_preserves_false_for_missing_key() {
    let mem = Arc::new(RecordingMemory::default());
    let state = state_with_memory(mem.clone(), false);

    let (status, value) = response_json(
        handle_api_memory_delete(State(state), HeaderMap::new(), Path("missing".to_string()))
            .await
            .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["status"], "ok");
    assert_eq!(value["deleted"], false);
    assert_eq!(mem.forgotten.lock().as_slice(), &["missing".to_string()]);
}

#[tokio::test]
async fn memory_handlers_map_backend_errors_to_500() {
    let failing = Arc::new(RecordingMemory {
        fail_recall: true,
        fail_list: true,
        fail_store: true,
        fail_forget: true,
        ..RecordingMemory::default()
    });

    let (status, value) = response_json(
        handle_api_memory_list(
            State(state_with_memory(failing.clone(), false)),
            HeaderMap::new(),
            Query(MemoryQuery { query: Some("boom".to_string()), category: None }),
        )
        .await
        .into_response(),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(value["error"].as_str().unwrap_or_default().contains("Memory recall failed"));

    let (status, value) = response_json(
        handle_api_memory_list(
            State(state_with_memory(failing.clone(), false)),
            HeaderMap::new(),
            Query(MemoryQuery { query: None, category: None }),
        )
        .await
        .into_response(),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(value["error"].as_str().unwrap_or_default().contains("Memory list failed"));

    let (status, value) = response_json(
        handle_api_memory_store(
            State(state_with_memory(failing.clone(), false)),
            HeaderMap::new(),
            JsonResponse(MemoryStoreBody {
                key: "k".to_string(),
                content: "c".to_string(),
                category: Some("daily".to_string()),
            }),
        )
        .await
        .into_response(),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(value["error"].as_str().unwrap_or_default().contains("Memory store failed"));

    let (status, value) = response_json(
        handle_api_memory_delete(
            State(state_with_memory(failing, false)),
            HeaderMap::new(),
            Path("k".to_string()),
        )
        .await
        .into_response(),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(value["error"].as_str().unwrap_or_default().contains("Memory forget failed"));
}
