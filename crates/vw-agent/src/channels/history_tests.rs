// 本文件验证按会话键维护、压缩、清理渠道对话历史的边界行为。

use super::*;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::AutonomyConfig;
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::Provider;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
        Ok("ok".to_string())
    }
}

fn channel_message(thread_ts: Option<&str>) -> traits::ChannelMessage {
    traits::ChannelMessage {
        instance_id: "default".to_string(),
        id: "msg-1".to_string(),
        sender: "sender-1".to_string(),
        reply_target: "room-1".to_string(),
        content: "hello".to_string(),
        channel: "telegram".to_string(),
        timestamp: 1,
        thread_ts: thread_ts.map(str::to_string),
    }
}

fn test_context() -> ChannelRuntimeContext {
    let provider: Arc<dyn Provider> = Arc::new(StaticProvider);
    ChannelRuntimeContext {
        project_state: Default::default(),
        message_snapshot: None,
        session_full_access: true,
        channels_by_name: Arc::new(HashMap::new()),
        provider: Arc::clone(&provider),
        default_provider: Arc::new("provider".to_string()),
        memory: Arc::new(NoneMemory::new()),
        tools_registry: Arc::new(Vec::new()),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("system".to_string()),
        model: Arc::new("model".to_string()),
        temperature: 0.0,
        auto_save_memory: false,
        max_iterations: 1,
        min_relevance_score: 0.0,
        conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        provider_cache: Arc::new(Mutex::new(HashMap::new())),
        route_overrides: Arc::new(Mutex::new(HashMap::new())),
        api_key: None,
        api_url: None,
        reliability: Arc::new(crate::app::agent::config::ReliabilityConfig::default()),
        provider_runtime_options: crate::app::agent::providers::ProviderRuntimeOptions::default(),
        workspace_dir: Arc::new(PathBuf::from(".")),
        message_timeout_secs: 30,
        interrupt_on_new_message: false,
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        hooks: None,
        non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),
        query_classification: crate::app::agent::config::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&AutonomyConfig::default())),
    }
}

#[test]
fn conversation_keys_include_thread_when_present() {
    let threaded = channel_message(Some("thread-9"));
    let unthreaded = channel_message(None);

    assert_ne!(conversation_memory_key(&threaded), conversation_memory_key(&unthreaded));
    assert_ne!(conversation_history_key(&threaded), conversation_history_key(&unthreaded));
    let identity: serde_json::Value =
        serde_json::from_str(&conversation_history_key(&threaded)).unwrap();
    assert_eq!(identity["thread"], "thread-9");
    assert_eq!(identity["reply_target"], "room-1");
    assert_eq!(identity["instance"], "default");
    assert_eq!(interruption_scope_key(&threaded), conversation_history_key(&threaded));
}

#[test]
fn normalize_cached_turns_merges_unexpected_roles_and_skips_system() {
    let turns = vec![
        ChatMessage::system("ignored"),
        ChatMessage::user("one"),
        ChatMessage::user("two"),
        ChatMessage::assistant("answer"),
        ChatMessage::assistant("more"),
        ChatMessage::user("three"),
    ];

    let normalized = normalize_cached_channel_turns(turns);

    assert_eq!(normalized.len(), 3);
    assert_eq!(normalized[0].role, "user");
    assert_eq!(normalized[0].content, "one\n\ntwo");
    assert_eq!(normalized[1].role, "assistant");
    assert_eq!(normalized[1].content, "answer\n\nmore");
    assert_eq!(normalized[2].content, "three");
}

#[test]
fn normalize_cached_turns_drops_orphan_leading_roles_and_merges_without_separator_for_empty_last() {
    let turns = vec![
        ChatMessage::assistant("orphan"),
        ChatMessage::tool("ignored"),
        ChatMessage::user(""),
        ChatMessage::user("late"),
        ChatMessage::user(""),
        ChatMessage::assistant(""),
        ChatMessage::assistant("tail"),
    ];

    let normalized = normalize_cached_channel_turns(turns);

    assert_eq!(normalized.len(), 2);
    assert_eq!(normalized[0].role, "user");
    assert_eq!(normalized[0].content, "late");
    assert_eq!(normalized[1].role, "assistant");
    assert_eq!(normalized[1].content, "tail");
}

#[test]
fn append_sender_turn_preserves_complete_history() {
    let ctx = test_context();

    for idx in 0..53 {
        append_sender_turn(&ctx, "sender", ChatMessage::user(format!("turn-{idx}")));
    }

    let histories = ctx.conversation_histories.lock().unwrap();
    let turns = histories.get("sender").expect("history should exist");
    assert_eq!(turns.len(), 53);
    assert_eq!(turns.first().expect("first turn").content, "turn-0");
}

#[test]
fn clear_sender_history_removes_existing_key_and_ignores_missing_key() {
    let ctx = test_context();
    append_sender_turn(&ctx, "sender", ChatMessage::user("hello"));

    clear_sender_history(&ctx, "missing");
    assert!(ctx.conversation_histories.lock().unwrap().contains_key("sender"));

    clear_sender_history(&ctx, "sender");
    assert!(!ctx.conversation_histories.lock().unwrap().contains_key("sender"));
}

#[test]
// 共享历史锁中毒时仍需降级恢复，不能让渠道永久停服。
fn history_operations_recover_from_poisoned_history_lock() {
    let ctx = test_context();
    let histories = Arc::clone(&ctx.conversation_histories);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _guard = histories.lock().unwrap();
        panic!("poison history lock");
    }));

    clear_sender_history(&ctx, "missing");
    append_sender_turn(&ctx, "sender", ChatMessage::user("hello"));
    assert!(rollback_orphan_user_turn(&ctx, "sender", "hello"));
    assert!(
        !ctx.conversation_histories
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key("sender")
    );
}

#[test]
fn rollback_orphan_user_turn_only_removes_matching_last_user_turn() {
    let ctx = test_context();

    assert!(!rollback_orphan_user_turn(&ctx, "sender", "hello"));

    append_sender_turn(&ctx, "sender", ChatMessage::user("hello"));
    append_sender_turn(&ctx, "sender", ChatMessage::assistant("reply"));
    assert!(!rollback_orphan_user_turn(&ctx, "sender", "hello"));

    append_sender_turn(&ctx, "sender", ChatMessage::user("latest"));
    assert!(!rollback_orphan_user_turn(&ctx, "sender", "other"));
    assert!(rollback_orphan_user_turn(&ctx, "sender", "latest"));

    let histories = ctx.conversation_histories.lock().unwrap();
    assert_eq!(histories["sender"].last().expect("last turn").role, "assistant");
}

#[test]
fn rollback_orphan_user_turn_removes_empty_history_entry() {
    let ctx = test_context();
    append_sender_turn(&ctx, "sender", ChatMessage::user("only"));

    assert!(rollback_orphan_user_turn(&ctx, "sender", "only"));
    assert!(!ctx.conversation_histories.lock().unwrap().contains_key("sender"));
}
