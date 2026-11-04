use super::*;
use crate::providers::{ChatMessage, ChatRequest, ChatResponse};
use serde_json::json;

struct Echo;
#[async_trait::async_trait]
impl Provider for Echo {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        prompt: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        Ok(prompt.into())
    }
    async fn chat(
        &self,
        request: ChatRequest<'_>,
        _: &str,
        _: f64,
    ) -> anyhow::Result<ChatResponse> {
        Ok(ChatResponse {
            text: Some(request.messages.last().unwrap().content.clone()),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        })
    }
}
fn runtime(provider: Arc<dyn Provider>) -> WorkflowRuntime {
    WorkflowRuntime {
        video_edit_budget: crate::workflow::budget::VideoEditRunBudget::default(),
        provider,
        knowledge_provider: None,
        document_extractor: None,
        tool_provider: None,
        agent_provider: None,
        pause_store: None,
        model: "test".into(),
        temperature: 0.0,
    }
}
fn request() -> WorkflowRunRequest {
    let graph = json!({"workflow":{"graph": {
        "nodes":[{"id":"start","data":{"type":"start","variables":[{"variable":"items"}]}},
        {"id":"iter","data":{"type":"iteration","parallel":true,"input_selector":["start","items"],"output_selector":["model","text"],"graph":{
            "nodes":[{"id":"model","data":{"type":"llm","prompt_template":[{"role":"user","text":"{{#iter.item#}}"}]}}],"edges":[]
        }}}],"edges":[{"source":"start","target":"iter"}]
    }}});
    WorkflowRunRequest {
        workflow_yaml: Some(serde_json::to_string(&graph).unwrap()),
        inputs: BTreeMap::from([("items".into(), json!(["a", "b", "c", "d", "e", "f"]))]),
        max_steps: 100,
        ..Default::default()
    }
}
#[tokio::test]
async fn video_edit_concurrency_production_iteration_preserves_order_and_isolation() {
    let response = run_workflow(runtime(Arc::new(Echo)), request()).await.unwrap();
    assert_eq!(
        response.status,
        vw_api_types::workflow::WorkflowRunStatus::Succeeded,
        "{:?}",
        response.error
    );
    assert_eq!(
        response.nodes.last().unwrap().outputs["output"],
        json!(["a", "b", "c", "d", "e", "f"])
    );
}

use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use tokio::sync::{mpsc as test_mpsc, oneshot};
struct Controlled {
    entered: test_mpsc::UnboundedSender<(String, oneshot::Sender<()>)>,
    completed: test_mpsc::UnboundedSender<String>,
    active: Arc<AtomicUsize>,
    peak: AtomicUsize,
}
struct Active(Arc<AtomicUsize>);
impl Drop for Active {
    fn drop(&mut self) {
        self.0.fetch_sub(1, AtomicOrdering::SeqCst);
    }
}
#[async_trait::async_trait]
impl Provider for Controlled {
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
        _: &str,
        _: f64,
    ) -> anyhow::Result<ChatResponse> {
        let prompt = request.messages.last().unwrap().content.clone();
        let active = self.active.fetch_add(1, AtomicOrdering::SeqCst) + 1;
        let _guard = Active(self.active.clone());
        self.peak.fetch_max(active, AtomicOrdering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.entered.send((prompt.clone(), tx)).unwrap();
        rx.await?;
        let _ = self.completed.send(prompt.clone());
        anyhow::ensure!(prompt != "fail", "controlled leaf failure");
        Ok(ChatResponse {
            text: Some(prompt),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        })
    }
}
type Entered = test_mpsc::UnboundedReceiver<(String, oneshot::Sender<()>)>;
fn controlled() -> (Arc<Controlled>, Entered, test_mpsc::UnboundedReceiver<String>) {
    let (tx, rx) = test_mpsc::unbounded_channel();
    let (completed, done) = test_mpsc::unbounded_channel();
    (
        Arc::new(Controlled {
            entered: tx,
            completed,
            active: Arc::new(AtomicUsize::new(0)),
            peak: AtomicUsize::new(0),
        }),
        rx,
        done,
    )
}
async fn enter(rx: &mut Entered) -> (String, oneshot::Sender<()>) {
    tokio::time::timeout(Duration::from_secs(10), rx.recv())
        .await
        .expect("leaf must enter")
        .expect("channel open")
}
async fn finish(
    task: tokio::task::JoinHandle<Result<WorkflowRunResponse, String>>,
) -> WorkflowRunResponse {
    tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .expect("run must settle")
        .unwrap()
        .unwrap()
}
fn edit_request(request: &mut WorkflowRunRequest, edit: impl FnOnce(&mut Value)) {
    let mut value: Value = serde_json::from_str(request.workflow_yaml.as_ref().unwrap()).unwrap();
    edit(&mut value["workflow"]["graph"]["nodes"][1]["data"]);
    request.workflow_yaml = Some(value.to_string());
}
#[tokio::test]
async fn video_edit_concurrency_five_enter_sixth_waits_reverse_completion_and_events() {
    let (provider, mut entered, mut done) = controlled();
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let capture = events.clone();
    let runtime = runtime(provider.clone());
    let task = tokio::spawn(run_workflow_with_events(runtime, request(), move |e| {
        capture.lock().unwrap().push(e)
    }));
    let mut gates = Vec::new();
    for _ in 0..5 {
        gates.push(enter(&mut entered).await);
    }
    assert_eq!(provider.active.load(AtomicOrdering::SeqCst), 5);
    assert!(matches!(entered.try_recv(), Err(test_mpsc::error::TryRecvError::Empty)));
    let (_, gate) = gates.pop().unwrap();
    gate.send(()).unwrap();
    done.recv().await.unwrap();
    let sixth = enter(&mut entered).await;
    assert_eq!(sixth.0, "f");
    sixth.1.send(()).unwrap();
    done.recv().await.unwrap();
    for (_, gate) in gates.into_iter().rev() {
        gate.send(()).unwrap();
        done.recv().await.unwrap();
    }
    let result = finish(task).await;
    assert_eq!(result.status, WorkflowRunStatus::Succeeded);
    assert_eq!(
        result.nodes.last().unwrap().outputs["output"],
        json!(["a", "b", "c", "d", "e", "f"])
    );
    assert_eq!(provider.peak.load(AtomicOrdering::SeqCst), 5);
    assert_eq!(provider.active.load(AtomicOrdering::SeqCst), 0);
    let events = events.lock().unwrap();
    let calls: BTreeSet<_> = events
        .iter()
        .filter_map(|event| match event {
            WorkflowRunEvent::Child { context, event }
                if matches!(event.as_ref(), WorkflowRunEvent::NodeStarted(_)) =>
            {
                assert_eq!(context.run_id, result.run_id);
                assert_eq!(context.parent_node_id, "iter");
                assert_eq!(context.node_id, "model");
                Some(context.call_id.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(calls.len(), 6);
}
#[tokio::test]
async fn video_edit_concurrency_nested_llm_branches_share_budget_with_external_leaf() {
    let (provider, mut entered, _) = controlled();
    let runtime = runtime(provider.clone());
    let external_leaf = runtime.video_edit_budget.acquire().await.unwrap();
    let mut request = request();
    edit_request(&mut request, |data| {
        let mut second = data["graph"]["nodes"][0].clone();
        second["id"] = json!("second");
        data["graph"]["nodes"].as_array_mut().unwrap().push(second);
    });
    let task = tokio::spawn(run_workflow(runtime, request));
    let mut gates = Vec::new();
    for _ in 0..4 {
        gates.push(enter(&mut entered).await);
    }
    assert!(entered.try_recv().is_err());
    assert_eq!(provider.active.load(AtomicOrdering::SeqCst), 4);
    drop(external_leaf);
    gates.push(enter(&mut entered).await);
    for (_, gate) in gates {
        gate.send(()).unwrap();
    }
    for _ in 5..12 {
        enter(&mut entered).await.1.send(()).unwrap();
    }
    assert_eq!(finish(task).await.status, WorkflowRunStatus::Succeeded);
    assert!(provider.peak.load(AtomicOrdering::SeqCst) <= 5);
}
#[tokio::test]
async fn video_edit_concurrency_cancellation_stops_active_and_queued_branches() {
    let (provider, mut entered, _) = controlled();
    let runtime = runtime(provider.clone());
    let budget = runtime.video_edit_budget.clone();
    let mut request = request();
    edit_request(&mut request, |data| {
        let mut second = data["graph"]["nodes"][0].clone();
        second["id"] = json!("second");
        data["graph"]["nodes"].as_array_mut().unwrap().push(second);
    });
    let task = tokio::spawn(run_workflow(runtime, request));
    let mut gates = Vec::new();
    for _ in 0..5 {
        gates.push(enter(&mut entered).await);
    }
    budget.cancel();
    assert_eq!(finish(task).await.status, WorkflowRunStatus::Failed);
    // JoinSet abort is drained in the iteration. No new provider call may start.
    assert_eq!(provider.active.load(AtomicOrdering::SeqCst), 0);
    assert!(entered.try_recv().is_err());
    drop(gates);
}
#[tokio::test]
async fn video_edit_concurrency_failure_stops_dispatch_and_does_not_succeed() {
    let (provider, mut entered, _) = controlled();
    let mut request = request();
    request.inputs.insert("items".into(), json!(["fail", "b", "c", "d", "e", "f"]));
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let capture = events.clone();
    let task =
        tokio::spawn(run_workflow_with_events(runtime(provider.clone()), request, move |event| {
            capture.lock().unwrap().push(event)
        }));
    let mut gates = Vec::new();
    for _ in 0..5 {
        gates.push(enter(&mut entered).await);
    }
    let fail = gates.iter().position(|(p, _)| p == "fail").unwrap();
    gates.swap_remove(fail).1.send(()).unwrap();
    let response = finish(task).await;
    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.unwrap().contains("controlled leaf failure"));
    assert!(events.lock().unwrap().iter().any(|event| matches!(event,
        WorkflowRunEvent::Child { event, .. } if matches!(event.as_ref(), WorkflowRunEvent::NodeFinished(finished) if finished.node.status == WorkflowNodeRunStatus::Failed)
    )));
    assert!(entered.try_recv().is_err());
    assert_eq!(provider.active.load(AtomicOrdering::SeqCst), 0);
}
#[tokio::test]
async fn video_edit_concurrency_invalid_limits_side_effects_and_failure_strategies_rejected() {
    for value in [json!(0), json!(-1), json!(1.5), json!(6), json!("5")] {
        let mut request = request();
        edit_request(&mut request, |data| data["max_parallel"] = value);
        assert_eq!(
            run_workflow(runtime(Arc::new(Echo)), request).await.unwrap().status,
            WorkflowRunStatus::Failed
        );
    }
    for kind in ["agent", "tool", "http-request", "code", "iteration", "loop", "variable-assigner"]
    {
        let mut request = request();
        edit_request(&mut request, |data| data["graph"]["nodes"][0]["data"]["type"] = json!(kind));
        let response = run_workflow(runtime(Arc::new(Echo)), request).await.unwrap();
        assert_eq!(response.status, WorkflowRunStatus::Failed, "{kind}");
        assert!(response.error.unwrap().contains("rejects node"));
    }
    for strategy in ["continue_on_error", "remove_failed"] {
        let mut request = request();
        edit_request(&mut request, |data| data["error_strategy"] = json!(strategy));
        assert_eq!(
            run_workflow(runtime(Arc::new(Echo)), request).await.unwrap().status,
            WorkflowRunStatus::Failed
        );
    }
}
#[tokio::test]
async fn video_edit_concurrency_lower_limits_and_shared_step_exhaustion() {
    for (run_limit, iteration_limit) in [(2, 5), (5, 1)] {
        let (provider, mut entered, _) = controlled();
        let mut runtime = runtime(provider.clone());
        runtime.video_edit_budget =
            crate::workflow::budget::VideoEditRunBudget::new(run_limit).unwrap();
        let mut request = request();
        edit_request(&mut request, |data| data["max_parallel"] = json!(iteration_limit));
        let limit = run_limit.min(iteration_limit);
        let task = tokio::spawn(run_workflow(runtime, request));
        let mut gates = Vec::new();
        for _ in 0..limit {
            gates.push(enter(&mut entered).await);
        }
        assert!(entered.try_recv().is_err());
        for (_, gate) in gates {
            gate.send(()).unwrap();
        }
        for _ in limit..6 {
            enter(&mut entered).await.1.send(()).unwrap();
        }
        assert_eq!(finish(task).await.status, WorkflowRunStatus::Succeeded);
        assert!(provider.peak.load(AtomicOrdering::SeqCst) <= limit);
    }
    let mut request = request();
    request.max_steps = 3;
    let response = run_workflow(runtime(Arc::new(Echo)), request).await.unwrap();
    assert_eq!(response.status, WorkflowRunStatus::Failed);
    assert!(response.error.unwrap().contains("shared execution step budget"));
}

struct Streaming(Arc<Controlled>);
#[async_trait::async_trait]
impl Provider for Streaming {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        unreachable!()
    }
    fn supports_streaming(&self) -> bool {
        true
    }
    fn stream_chat_with_history(
        &self,
        messages: &[ChatMessage],
        _: &str,
        _: f64,
        _: crate::providers::traits::StreamOptions,
    ) -> futures_util::stream::BoxStream<
        'static,
        crate::providers::traits::StreamResult<crate::providers::traits::StreamChunk>,
    > {
        let inner = self.0.clone();
        let messages = messages.to_vec();
        Box::pin(futures_util::stream::once(async move {
            inner
                .chat(ChatRequest { messages: &messages, tools: None }, "test", 0.0)
                .await
                .map(|r| crate::providers::traits::StreamChunk {
                    delta: r.text.unwrap(),
                    is_final: false,
                    token_count: 1,
                })
                .map_err(|e| crate::providers::traits::StreamError::Provider(e.to_string()))
        }))
    }
}
#[tokio::test]
async fn video_edit_concurrency_streaming_holds_permit_and_releases_after_failure_or_cancel() {
    for fail in [false, true] {
        let (provider, mut entered, _) = controlled();
        let mut runtime = runtime(Arc::new(Streaming(provider.clone())));
        runtime.video_edit_budget = crate::workflow::budget::VideoEditRunBudget::new(1).unwrap();
        let budget = runtime.video_edit_budget.clone();
        let mut request = request();
        request
            .inputs
            .insert("items".into(), if fail { json!(["fail", "b"]) } else { json!(["a", "b"]) });
        let task = tokio::spawn(run_workflow(runtime, request));
        let (_, gate) = enter(&mut entered).await;
        let mut external = Box::pin(budget.acquire());
        assert!(futures_util::poll!(&mut external).is_pending());
        if fail {
            gate.send(()).unwrap();
            assert_eq!(finish(task).await.status, WorkflowRunStatus::Failed);
            let permit =
                tokio::time::timeout(Duration::from_secs(10), external).await.unwrap().unwrap();
            drop(permit);
        } else {
            budget.cancel();
            assert_eq!(finish(task).await.status, WorkflowRunStatus::Failed);
            assert!(external.await.is_err());
            drop(gate);
        }
        tokio::task::yield_now().await;
        assert_eq!(provider.active.load(AtomicOrdering::SeqCst), 0);
        assert!(entered.try_recv().is_err());
    }
}
