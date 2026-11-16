//! 真实宿主插件进程的 Stop/Interrupt 接线验收。

use super::*;
use model_types::ProviderCapabilities;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

struct RecordingProvider {
    responses: Mutex<VecDeque<model_types::ChatResponse>>,
    calls: Arc<AtomicUsize>,
}

impl RecordingProvider {
    fn from_responses(
        responses: Vec<model_types::ChatResponse>,
        calls: Arc<AtomicUsize>,
    ) -> Self {
        Self { responses: Mutex::new(responses.into()), calls }
    }

}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for RecordingProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities { native_tool_search: false, native_tool_calling: true, vision: false }
    }

    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("turn tests use structured chat")
    }

    async fn chat(
        &self,
        _request: model_types::ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<model_types::ChatResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.responses
            .lock()
            .expect("responses mutex poisoned")
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("recording provider response script exhausted"))
    }
}

#[tokio::test]
async fn codex_plugin_compat_t44_actual_turn_continues_after_stop_block() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let runner = crate::plugins::t44_tests::runner(root.path(), state.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let response = model_types::ChatResponse {
        text: Some("finished".into()),
        tool_calls: Vec::new(),
        usage: None,
        reasoning_content: None,
    };
    let provider =
        RecordingProvider::from_responses(vec![response.clone(), response], calls.clone());
    let mut history = vec![ChatMessage::user("hello")];
    let mut security = SecurityPolicy::default();
    security.workspace_dir = root.path().to_path_buf();
    let result = super::run_tool_call_loop_with_reply_target(
        &provider,
        &mut history,
        &[],
        &crate::observability::NoopObserver,
        "fixture",
        "fixture",
        0.0,
        true,
        None,
        "cli",
        None,
        &Default::default(),
        4,
        None,
        None,
        Some(runner),
        Some(Arc::new(security)),
        &[],
    )
    .await
    .unwrap();
    assert_eq!(result, "finished");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(
        history.iter().any(|message| message.role == "user" && message.content == "verify work")
    );
    let events: Vec<serde_json::Value> = std::fs::read_to_string(state.path().join("events"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["stop_hook_active"], false);
    assert_eq!(events[1]["stop_hook_active"], true);
}

#[tokio::test]
async fn codex_plugin_compat_t44_actual_turn_failure_and_cancellation() {
    for cancelled in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let runner = crate::plugins::t44_tests::runner(root.path(), state.path());
        let provider = RecordingProvider::from_responses(Vec::new(), Arc::default());
        let mut history = vec![ChatMessage::user("hello")];
        let mut security = SecurityPolicy::default();
        security.workspace_dir = root.path().to_path_buf();
        let token = CancellationToken::new();
        if cancelled {
            token.cancel();
        }
        let result = super::run_tool_call_loop_with_reply_target(
            &provider,
            &mut history,
            &[],
            &crate::observability::NoopObserver,
            "fixture",
            "fixture",
            0.0,
            true,
            None,
            "cli",
            None,
            &Default::default(),
            4,
            Some(token),
            None,
            Some(runner),
            Some(Arc::new(security)),
            &[],
        )
        .await;
        assert!(result.is_err());
        let marker = state.path().join("events");
        if cancelled {
            let events = std::fs::read_to_string(marker).unwrap();
            let event: serde_json::Value = serde_json::from_str(events.trim()).unwrap();
            assert_eq!(event["hook_event_name"], "Interrupt");
            assert!(event.get("stop_hook_active").is_none());
        } else {
            assert!(!marker.exists());
        }
    }
}
