use super::*;
use serde_json::json;
use std::sync::{Arc, Mutex};

fn request(root: &std::path::Path, query: &str) -> Request {
    Request {
        stream: 0,
        session: "workflow-binding-test".into(),
        query: query.into(),
        root: Some(root.to_string_lossy().into_owned()),
        model: Some("unused-provider/unused-model".into()),
        reliability: Default::default(),
        options: json!({"agent":"main","allowed_tools":["bash","file_read"]}),
        approval: None,
        channel_name: None,
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: vec![],
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools: Default::default(),
    }
}

fn binding_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".shiju")).unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    std::fs::write(
        dir.path().join(".shiju/workflow_commands.json"),
        json!({
            "platform_id":tenant.platform_id().as_ref(),
            "company_id":tenant.company_id().as_ref(),
            "commands":[{"command":"/shiju 剪辑","workflow_file":"preview.yaml","inputs":{}}]
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("preview.yaml"),
        r#"
workflow:
  graph:
    nodes:
      - id: start
        data: {type: start}
      - id: answer
        data: {type: answer, answer: 'verified preview'}
    edges:
      - {source: start, target: answer}
"#,
    )
    .unwrap();
    dir
}

#[test]
fn native_workflow_bound_command_executes_graph_and_returns_answer() {
    let dir = binding_fixture();
    let request = request(dir.path(), "/shiju 剪辑 sp7");
    let context = ToolRuntimeContext::new(request.session.clone(), request.root.clone());
    let events = Arc::new(Mutex::new(Vec::new()));
    let recorded = events.clone();
    assert!(run_bound_command(&request, &context, &mut move |event| {
        recorded.lock().unwrap().push(event);
        true
    }));
    let events = events.lock().unwrap();
    assert!(
        events.iter().any(|e| matches!(e, StreamEvent::Delta(text) if text == "verified preview"))
    );
    assert!(matches!(events.last(), Some(StreamEvent::Done(_))));
}

#[test]
fn native_workflow_unmatched_command_preserves_normal_session() {
    let dir = binding_fixture();
    for query in ["/shiju 登录", "/shijux 剪辑 sp7", "普通消息", "/shiju 剪辑器"] {
        let request = request(dir.path(), query);
        let context = ToolRuntimeContext::new(request.session.clone(), request.root.clone());
        assert!(!run_bound_command(&request, &context, &mut |_| panic!("unexpected dispatch")));
    }
}

#[test]
fn native_workflow_skill_palette_selection_dispatches_same_graph() {
    let dir = binding_fixture();
    let events = run_events(&request(
        dir.path(),
        "剪辑 sp7\n\n<session_control_selection>\n用户选中了以下上下文。\n技能：shiju\n</session_control_selection>",
    ));
    assert!(
        events.iter().any(|e| matches!(e, StreamEvent::Delta(text) if text == "verified preview"))
    );
    assert!(matches!(events.last(), Some(StreamEvent::Done(_))));
}

fn run_events(request: &Request) -> Vec<StreamEvent> {
    let context = ToolRuntimeContext::new(request.session.clone(), request.root.clone());
    let events = Arc::new(Mutex::new(vec![]));
    let recorded = events.clone();
    assert!(run_bound_command(request, &context, &mut move |event| {
        recorded.lock().unwrap().push(event);
        true
    }));
    Arc::try_unwrap(events).unwrap().into_inner().unwrap()
}

#[test]
fn native_workflow_rejects_cross_tenant_binding() {
    let dir = binding_fixture();
    let path = dir.path().join(".shiju/workflow_commands.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["company_id"] = json!("other-company");
    std::fs::write(path, value.to_string()).unwrap();
    let events = run_events(&request(dir.path(), "/shiju 剪辑 sp7"));
    assert!(matches!(events.last(), Some(StreamEvent::Error(error)) if error.contains("租户")));
    assert!(!events.iter().any(|e| matches!(e, StreamEvent::Done(_))));
}

#[test]
fn native_workflow_rejects_file_outside_project() {
    let dir = binding_fixture();
    let outside = tempfile::NamedTempFile::new().unwrap();
    let path = dir.path().join(".shiju/workflow_commands.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["commands"][0]["workflow_file"] = json!(outside.path());
    std::fs::write(path, value.to_string()).unwrap();
    let events = run_events(&request(dir.path(), "/shiju 剪辑 sp7"));
    assert!(matches!(events.last(), Some(StreamEvent::Error(error)) if error.contains("目录内")));
}

#[test]
fn native_workflow_failed_node_never_reports_success() {
    let dir = binding_fixture();
    let path = dir.path().join("preview.yaml");
    let yaml =
        std::fs::read_to_string(&path).unwrap().replace("type: answer", "type: unsupported-node");
    std::fs::write(path, yaml).unwrap();
    let events = run_events(&request(dir.path(), "/shiju 剪辑 sp7"));
    assert!(matches!(events.last(), Some(StreamEvent::Error(_))));
    assert!(!events.iter().any(|e| matches!(e, StreamEvent::Delta(_) | StreamEvent::Done(_))));
}

#[test]
fn native_workflow_preserves_cancellation_and_tool_restrictions() {
    let dir = binding_fixture();
    let mut request = request(dir.path(), "/shiju 剪辑 sp7");
    let token = tokio_util::sync::CancellationToken::new();
    token.cancel();
    request.abort_token = Some(token);
    let events = run_events(&request);
    assert!(matches!(events.last(), Some(StreamEvent::Error(error)) if error == "cancelled"));
    assert!(agent::check_allowed("bash", Some(&json!(["file_read"]))).is_err());
    assert!(agent::check_allowed("file_read", Some(&json!(["file_read"]))).is_ok());
}

struct CloudFixtureProvider {
    invalid_assets: bool,
}

#[async_trait::async_trait]
impl crate::providers::Provider for CloudFixtureProvider {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("unexpected transport")
    }
    async fn chat(
        &self,
        request: crate::providers::ChatRequest<'_>,
        _: &str,
        _: f64,
    ) -> anyhow::Result<crate::providers::ChatResponse> {
        let prompt = &request.messages[0].content;
        let text = if prompt.contains("Return only a short brief") {
            "brief".to_string()
        } else if prompt.contains("keyword visual segments") {
            json!([{"anchor":"A","start":0,"end":1,"visual_type":"keyword",
                "assets":if self.invalid_assets {json!([])} else {json!(["title"])} }])
            .to_string()
        } else {
            json!([
                {"anchor":"B","start":1,"end":2,"visual_type":"b_roll","assets":["footage"]},
                {"anchor":"C","start":2,"end":3,"visual_type":"motion_graphic","assets":["diagram"]}
            ])
            .to_string()
        };
        Ok(crate::providers::ChatResponse {
            text: Some(text),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        })
    }
}

struct CloudFixtureAgent {
    executions: Arc<std::sync::atomic::AtomicUsize>,
    preview_ready: bool,
}

#[async_trait::async_trait]
impl crate::workflow::WorkflowAgentProvider for CloudFixtureAgent {
    async fn run(
        &self,
        request: crate::workflow::WorkflowAgentRequest,
        _: Arc<dyn crate::workflow::WorkflowToolProvider>,
    ) -> Result<crate::workflow::WorkflowAgentResult, String> {
        let prompt = &request.messages[0].content;
        let answer = if prompt.contains("素材准备节点") {
            json!({"project_id":"project-1","source_asset_id":"asset-1","transcript":"A B C",
                "timestamps":[{"word":"A","start":0,"end":1},{"word":"B","start":1,"end":2},{"word":"C","start":2,"end":3}]}).to_string()
        } else {
            let plan = prompt
                .split_once("<validated_edit_plan>")
                .unwrap()
                .1
                .split_once("</validated_edit_plan>")
                .unwrap()
                .0;
            let plan: serde_json::Value = serde_json::from_str(plan).unwrap();
            assert_eq!(plan["segments"].as_array().unwrap().len(), 3);
            assert!(prompt.contains("asset-1"));
            self.executions.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            json!({"project_id":"project-1","preview_url":"https://example.invalid/s/preview","preview_ready":self.preview_ready}).to_string()
        };
        Ok(crate::workflow::WorkflowAgentResult {
            answer,
            tool_outputs: vec![],
            reasoning: None,
            iterations: 1,
            success: true,
        })
    }
}

#[tokio::test]
async fn native_workflow_cloud_graph_gates_execution_on_valid_plan_and_preview() {
    for (invalid_assets, preview_ready) in [(false, true), (true, true), (false, false)] {
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let runtime = crate::workflow::WorkflowRuntime {
            provider: Arc::new(CloudFixtureProvider { invalid_assets }),
            model: "unused-model".into(),
            temperature: 0.0,
            tool_provider: Some(Arc::new(agent::NativeTools {
                context: ToolRuntimeContext::default(),
                allowed_tools: Some(json!([])),
            })),
            agent_provider: Some(Arc::new(CloudFixtureAgent {
                executions: count.clone(),
                preview_ready,
            })),
            knowledge_provider: None,
            document_extractor: None,
            pause_store: None,
        };
        let request = vw_api_types::workflow::WorkflowRunRequest {
            workflow_yaml: Some(
                include_str!("../../testdata/workflow/talking_head_cloud.yaml").into(),
            ),
            inputs: std::collections::BTreeMap::from([
                ("cloud_project_id".into(), json!("project-1")),
                ("project_directory".into(), json!("/fixture")),
                ("source_file".into(), json!("/fixture/input.mov")),
                ("shiju_skill_root".into(), json!("/fixture/skills/shiju")),
                ("style".into(), json!("clear")),
                ("requirements".into(), json!("preview only")),
            ]),
            max_steps: 30,
            ..Default::default()
        };
        let response = crate::workflow::run_workflow(runtime, request).await.unwrap();
        let success = !invalid_assets && preview_ready;
        assert_eq!(
            response.status == vw_api_types::workflow::WorkflowRunStatus::Succeeded,
            success
        );
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), usize::from(!invalid_assets));
        if success {
            assert!(response.answer.unwrap().contains("https://example.invalid/s/preview"));
        } else {
            assert!(response.answer.is_none());
        }
    }
}
