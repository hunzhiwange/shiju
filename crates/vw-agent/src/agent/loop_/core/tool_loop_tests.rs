//! 真实宿主子代理事件投递与完成检测出口契约。

use super::*;
use model_types::ProviderCapabilities;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio_util::sync::CancellationToken;

struct StaticProvider {
    completion: Option<(String, String)>,
    responses: Mutex<VecDeque<model_types::ChatResponse>>,
    calls: Arc<AtomicUsize>,
}

impl StaticProvider {
    fn from_responses(
        responses: Vec<model_types::ChatResponse>,
        calls: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            completion: None,
            responses: Mutex::new(responses.into()),
            calls,
        }
    }

}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for StaticProvider {
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
        anyhow::bail!("tool loop tests should use structured chat")
    }

    async fn chat(
        &self,
        request: model_types::ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<model_types::ChatResponse> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some((root, child)) = &self.completion {
            if call == 0 {
                crate::session::child_agent::coordinator::global_child_agent_coordinator()
                    .mark_succeeded(root, child, "channel_fact_alpha")
                    .unwrap();
            } else {
                assert!(request.messages.iter().any(|m| m.content.contains("channel_fact_alpha")));
            }
        }
        self.responses
            .lock()
            .expect("responses mutex poisoned")
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("static provider response script exhausted"))
    }
}

#[test]
fn completion_detection_is_reexported_for_tool_loop() {
    assert!(looks_like_unverified_action_completion_without_tool_call(
        "I have created the file successfully"
    ));
}

#[tokio::test]
async fn agcollab_channel_integrates_child_results() {
    let root = format!("channel-{}", uuid::Uuid::new_v4());
    let coordinator = crate::session::child_agent::coordinator::global_child_agent_coordinator();
    let child = format!("{root}-child");
    coordinator.register(&root, &child, "worker", "research").unwrap();
    let mut provider = StaticProvider::from_responses(
        ["premature answer", "integrated answer"]
            .into_iter()
            .map(|text| model_types::ChatResponse {
                text: Some(text.into()),
                tool_calls: vec![],
                usage: None,
                reasoning_content: None,
            })
            .collect(),
        Arc::new(AtomicUsize::new(0)),
    );
    provider.completion = Some((root.clone(), child));
    let mut history = vec![ChatMessage::user("research")];
    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let result = crate::tools::scope_tool_use_context(
        Arc::new(ToolUseContext::new(&root, None)),
        run_tool_call_loop(
            &provider,
            &mut history,
            &[],
            &crate::observability::NoopObserver,
            "static",
            "model",
            0.0,
            true,
            None,
            "channel",
            &crate::config::MultimodalConfig::default(),
            3,
            None,
            Some(tx),
            None,
            None,
            &[],
        ),
    )
    .await
    .unwrap();
    assert!(history.iter().any(|m| m.role == "user" && m.content.contains("channel_fact_alpha")));
    assert_eq!(result, "integrated answer");
    let mut streamed = String::new();
    while let Ok(delta) = rx.try_recv() {
        streamed.push_str(&delta);
    }
    assert!(streamed.contains("integrated answer"));
    assert!(!streamed.contains("premature answer"));
    assert!(!history.iter().any(|m| m.content == "premature answer"));
}
