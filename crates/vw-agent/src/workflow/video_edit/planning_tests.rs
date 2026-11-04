use super::planning::*;
#[test]
fn video_edit_plan_json_rejects_boolean_time_and_unstructured_assets() {
    let effect = super::merge_tests::effect(0, VisualType::Keyword);
    let mut value = serde_json::to_value(effect).unwrap();
    value["start"] = serde_json::json!(true);
    assert!(serde_json::from_value::<VideoEditEffect>(value.clone()).is_err());
    value["start"] = serde_json::json!(0);
    value["assets"] = serde_json::json!([{"prompt":"x"}]);
    assert!(serde_json::from_value::<VideoEditEffect>(value).is_err());
}

use super::{
    merge_tests::{effect, source},
    planning::{VideoPlanningContext, plan_video_edit},
};
use crate::{
    providers::{ChatRequest, ChatResponse, Provider},
    workflow::WorkflowRuntime,
};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

/// Only external model transport is replaced; production graph and merger run unchanged.
struct Endpoint(String);
#[async_trait::async_trait]
impl Provider for Endpoint {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        unreachable!()
    }
    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        _: f64,
    ) -> anyhow::Result<ChatResponse> {
        assert_eq!(model, "editable-model");
        let prompt = &request.messages.last().unwrap().content;
        let response = reqwest::Client::new()
            .post(&self.0)
            .body(prompt.clone())
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(ChatResponse {
            text: Some(response),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        })
    }
}
fn runtime(url: String) -> WorkflowRuntime {
    WorkflowRuntime {
        video_edit_budget: Default::default(),
        provider: Arc::new(Endpoint(url)),
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "editable-model".into(),
        temperature: 0.0,
    }
}
type Entry = (serde_json::Value, oneshot::Sender<Option<String>>);
async fn fixture() -> (String, mpsc::UnboundedReceiver<Entry>, tokio::task::JoinHandle<()>) {
    let (tx, rx) = mpsc::unbounded_channel::<Entry>();
    let app = axum::Router::new().route(
        "/",
        axum::routing::post(move |body: String| {
            let tx = tx.clone();
            async move {
                let (reply, wait) = oneshot::channel();
                tx.send((serde_json::from_str(&body).unwrap(), reply)).unwrap();
                match wait.await.unwrap_or(None) {
                    Some(body) => (axum::http::StatusCode::OK, body),
                    None => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "model failure".into()),
                }
            }
        }),
    );
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", socket.local_addr().unwrap());
    let handle = tokio::spawn(async move { axum::serve(socket, app).await.unwrap() });
    (url, rx, handle)
}
async fn entered(rx: &mut mpsc::UnboundedReceiver<Entry>) -> Entry {
    tokio::time::timeout(std::time::Duration::from_secs(15), rx.recv()).await.unwrap().unwrap()
}
fn context() -> VideoPlanningContext {
    VideoPlanningContext { brief: "bounded summary".into(), max_prompt_bytes: 8192, model: None }
}
fn reply(prompt: &serde_json::Value) -> String {
    let i = prompt["segment"]["index"].as_u64().unwrap();
    let e = effect(
        i,
        [VisualType::Keyword, VisualType::BRoll, VisualType::MotionGraphic][i as usize % 3],
    );
    serde_json::to_string(&VideoChunkPlan { chunk_id: e.chunk_id.clone(), effects: vec![e] })
        .unwrap()
}
#[tokio::test]
async fn video_edit_plan_real_iteration_six_segments_bounded_requests_and_shared_budget() {
    let (url, mut rx, server) = fixture().await;
    let rt = runtime(url);
    let budget = rt.video_edit_budget.clone();
    // An existing leaf consumes the same run-wide budget.
    let external = budget.acquire().await.unwrap();
    let task = tokio::spawn(async move {
        let (t, s, r) = source();
        plan_video_edit(rt, &t, &s, &r, &context()).await
    });
    let mut gates = Vec::new();
    for _ in 0..4 {
        gates.push(entered(&mut rx).await);
    }
    assert!(tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await.is_err());
    drop(external);
    gates.push(entered(&mut rx).await);
    let (prompt, gate) = gates.pop().unwrap();
    gate.send(Some(reply(&prompt))).unwrap();
    gates.push(entered(&mut rx).await);
    // Finish the remaining model requests in reverse arrival order.
    for (prompt, gate) in gates.into_iter().rev() {
        assert_eq!(prompt["requirements"], "test");
        assert_eq!(prompt["brief"], "bounded summary");
        assert!(prompt.get("transcript").is_none());
        assert!(prompt["segment"]["words"].as_array().unwrap().len() < 6);
        gate.send(Some(reply(&prompt))).unwrap();
    }
    let plan = tokio::time::timeout(std::time::Duration::from_secs(20), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        plan.effects.iter().map(|e| e.word_index).collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4, 5]
    );
    server.abort();
    let _ = server.await;
}
#[tokio::test]
async fn video_edit_plan_failed_segment_never_returns_partial_plan() {
    let (url, mut rx, server) = fixture().await;
    let task = tokio::spawn(async move {
        let (t, s, r) = source();
        plan_video_edit(runtime(url), &t, &s, &r, &context()).await
    });
    let mut gates = Vec::new();
    for _ in 0..5 {
        gates.push(entered(&mut rx).await);
    }
    let (_, gate) = gates.pop().unwrap();
    gate.send(None).unwrap();
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(20), task).await.unwrap().unwrap();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("segment planning failed"));
    drop(gates);
    server.abort();
    let _ = server.await;
}
#[tokio::test]
async fn video_edit_plan_context_overflow_is_explicit_before_model_request() {
    let (url, mut rx, server) = fixture().await;
    let (t, s, r) = source();
    let mut context = context();
    context.max_prompt_bytes = 20;
    let result = plan_video_edit(runtime(url), &t, &s, &r, &context).await;
    assert!(result.unwrap_err().to_string().contains("context bound"));
    assert!(rx.try_recv().is_err());
    server.abort();
    let _ = server.await;
}
