//! Plain LLM nodes retry DSML responses and only publish recovered task results.
use super::*;
use crate::providers::traits::{StreamChunk, StreamOptions, StreamResult, TokenUsage};
use crate::providers::{ChatMessage, ChatRequest, ChatResponse, Provider};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use vw_api_types::workflow::{WorkflowNodeRunStatus, WorkflowRunRequest, WorkflowRunStatus};

struct DsmlProvider {
    responses: Vec<String>,
    messages: Mutex<Vec<Vec<ChatMessage>>>,
    streaming: bool,
    calls: AtomicUsize,
}

#[async_trait::async_trait]
impl Provider for DsmlProvider {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        unreachable!("workflow uses chat or streaming")
    }

    async fn chat(
        &self,
        request: ChatRequest<'_>,
        _: &str,
        _: f64,
    ) -> anyhow::Result<ChatResponse> {
        self.messages.lock().unwrap().push(request.messages.to_vec());
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ChatResponse {
            text: Some(self.responses[index.min(self.responses.len() - 1)].clone()),
            tool_calls: vec![],
            usage: Some(TokenUsage { output_tokens: Some(1), ..Default::default() }),
            reasoning_content: None,
        })
    }

    fn supports_streaming(&self) -> bool {
        self.streaming
    }

    fn stream_chat_with_history(
        &self,
        messages: &[ChatMessage],
        _: &str,
        _: f64,
        _: StreamOptions,
    ) -> futures_util::stream::BoxStream<'static, StreamResult<StreamChunk>> {
        self.messages.lock().unwrap().push(messages.to_vec());
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        // Split every character so DSML markers span multiple transport chunks.
        let chunks: Vec<_> = self.responses[index.min(self.responses.len() - 1)]
            .chars()
            .map(|c| Ok(StreamChunk { delta: c.to_string(), is_final: false, token_count: 1 }))
            .collect();
        Box::pin(futures_util::stream::iter(chunks))
    }
}

const WORKFLOW: &str = r#"
workflow:
  graph:
    nodes:
      - id: start
        data: {type: start}
      - id: plan
        data:
          type: llm
          prompt_template: [{role: user, text: 'Return a short brief'}]
      - id: downstream
        data:
          type: llm
          prompt_template: [{role: user, text: 'Brief: {{#plan.text#}}'}]
    edges:
      - {source: start, target: plan}
      - {source: plan, target: downstream}
"#;

async fn run(
    responses: &[&str],
    streaming: bool,
) -> (vw_api_types::workflow::WorkflowRunResponse, Arc<DsmlProvider>, Vec<WorkflowRunEvent>) {
    let provider = Arc::new(DsmlProvider {
        responses: responses.iter().map(|s| s.to_string()).collect(),
        streaming,
        calls: AtomicUsize::new(0),
        messages: Mutex::new(Vec::new()),
    });
    let runtime = WorkflowRuntime {
        video_edit_budget: crate::workflow::budget::VideoEditRunBudget::default(),
        provider: provider.clone(),
        model: "deepseek".into(),
        temperature: 0.0,
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
    };
    let mut events = Vec::new();
    let result = run_workflow_with_events(
        runtime,
        WorkflowRunRequest { workflow_yaml: Some(WORKFLOW.into()), ..Default::default() },
        |event| events.push(event),
    )
    .await
    .expect("workflow response");
    (result, provider, events)
}

#[tokio::test]
async fn workflow_dsml_response_fails_without_running_downstream() {
    for streaming in [false, true] {
        for text in [
            "<｜｜DSML｜｜ calls>\n<｜｜DSML｜｜ invoke name=\"Bash\">\n<｜｜DSML｜｜ parameter name=\"command\" string=\"true\">pwd</｜｜DSML｜｜ parameter>\n</｜｜DSML｜｜ invoke>\n</｜｜DSML｜｜ calls>",
            "<｜DSML｜tool_calls><｜DSML｜invoke name=\"Bash\"><｜DSML｜parameter name=\"command\" string=\"true\">pwd</｜DSML｜parameter></｜DSML｜invoke></｜DSML｜tool_calls>",
            "short brief\n<｜｜DSML｜｜ invoke",
        ] {
            let (response, provider, _) = run(&[text], streaming).await;
            assert_eq!(response.status, WorkflowRunStatus::Failed);
            assert!(response.error.as_deref().unwrap().contains("DSML"));
            assert_eq!(provider.calls.load(Ordering::SeqCst), 3, "retry twice before failing");
            assert!(!response.nodes.iter().any(|node| node.node_id == "downstream"));
            let plan = response.nodes.iter().find(|node| node.node_id == "plan").unwrap();
            assert_eq!(plan.status, WorkflowNodeRunStatus::Failed);
            assert!(plan.outputs.is_empty());
        }
    }
}

#[tokio::test]
async fn workflow_dsml_mention_and_normal_json_reach_downstream() {
    for streaming in [false, true] {
        for text in
            ["Explain DSML in a short brief", r#"[{"anchor":"客户","assets":["订单画面"]}]"#]
        {
            let (response, provider, _) = run(&[text], streaming).await;
            assert_eq!(response.status, WorkflowRunStatus::Succeeded);
            assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
        }
    }
}

#[tokio::test]
async fn workflow_dsml_retry_recovers_before_publishing_downstream() {
    let malformed = "<｜｜DSML｜｜ calls><｜｜DSML｜｜ invoke name=\"Bash\">";
    let repaired = r#"[{"anchor":"客户","assets":["订单画面"]}]"#;
    for streaming in [false, true] {
        let (response, provider, events) = run(&[malformed, malformed, repaired], streaming).await;
        assert_eq!(response.status, WorkflowRunStatus::Succeeded);
        assert_eq!(provider.calls.load(Ordering::SeqCst), 4);
        let plan = response.nodes.iter().find(|n| n.node_id == "plan").unwrap();
        assert_eq!(plan.outputs["text"], repaired);
        assert_eq!(
            plan.outputs["json"],
            serde_json::from_str::<serde_json::Value>(repaired).unwrap()
        );
        let messages = provider.messages.lock().unwrap();
        assert_eq!(messages[0][0].content, messages[2][0].content, "preserve original task");
        assert_eq!(messages[1].len(), messages[0].len() + 1);
        assert!(messages[1].last().unwrap().content.contains("重新完成原始任务"));
        assert_eq!(messages[3][0].content, format!("Brief: {repaired}"));
        let mut preview = String::new();
        let mut resets = 0;
        for event in events {
            if let WorkflowRunEvent::NodeDelta(delta) = event {
                if delta.node_id == "plan" {
                    if delta.replace {
                        preview.clear();
                        resets += 1;
                    }
                    preview.push_str(&delta.text);
                }
            }
        }
        if streaming {
            assert!(resets >= 2);
            assert_eq!(preview, repaired, "failed DSML previews must be replaced");
        } else {
            assert_eq!(resets, 0);
        }
    }
}
