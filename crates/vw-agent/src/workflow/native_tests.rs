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
        session_skills: Default::default(),
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

#[tokio::test]
async fn native_workflow_provider_names_preserve_tool_restrictions() {
    use crate::workflow::{WorkflowToolProvider, WorkflowToolRequest};

    let tools = agent::NativeTools {
        context: ToolRuntimeContext::default(),
        allowed_tools: Some(json!([])),
    };
    for provider in ["shiju", "vibewindow", "other"] {
        for (action, credential_id) in [(None, None), (Some("run"), None), (None, Some("secret"))] {
            let error = tools
                .call(WorkflowToolRequest {
                    provider: provider.into(),
                    tool_name: "bash".into(),
                    action: action.map(str::to_string),
                    credential_id: credential_id.map(str::to_string),
                    inputs: Default::default(),
                })
                .await
                .unwrap_err();
            if provider == "shiju" && action.is_none() && credential_id.is_none() {
                assert!(error.contains("不在当前会话能力范围内"), "{error}");
            } else {
                assert!(error.contains("仅支持 shiju"), "{error}");
            }
        }
    }
}

#[tokio::test]
async fn native_workflow_provider_names_preserve_agent_restrictions() {
    use crate::workflow::{WorkflowAgentProvider, WorkflowAgentRequest, WorkflowAgentTool};

    let (events, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let native = agent::NativeAgent {
        parent: request(std::path::Path::new("."), "unused"),
        events,
        cancellation: tokio_util::sync::CancellationToken::new(),
    };
    let tools = Arc::new(agent::NativeTools {
        context: ToolRuntimeContext::default(),
        allowed_tools: Some(json!([])),
    });
    for provider in ["shiju", "vibewindow", "other"] {
        for (name, action, credential_id) in [
            ("bash", None, None),
            ("forbidden", None, None),
            ("bash", Some("run"), None),
            ("bash", None, Some("secret")),
        ] {
            let error = native
                .run(
                    WorkflowAgentRequest {
                        messages: vec![],
                        tools: vec![WorkflowAgentTool {
                            provider: provider.into(),
                            tool_name: name.into(),
                            action: action.map(str::to_string),
                            credential_id: credential_id.map(str::to_string),
                        }],
                        max_iterations: 1,
                        model: "unused".into(),
                        temperature: 0.0,
                        strategy: String::new(),
                    },
                    tools.clone(),
                )
                .await
                .unwrap_err();
            if provider != "shiju" || action.is_some() || credential_id.is_some() {
                assert!(error.contains("仅支持当前会话的 shiju 工具"), "{error}");
            } else if name == "forbidden" {
                assert!(error.contains("不在当前会话能力范围内"), "{error}");
            } else {
                assert_eq!(error, "工作流 agent 缺少消息");
            }
        }
    }
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
        assert!(request.tools.iter().all(|tool| tool.provider == "shiju"));
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
            video_edit_budget: crate::workflow::budget::VideoEditRunBudget::default(),
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

async fn action_graph(
    root: &std::path::Path,
    actions: Vec<serde_json::Value>,
    allowed: Vec<&str>,
) -> vw_api_types::workflow::WorkflowRunResponse {
    let mut nodes = vec![json!({"id":"start","data":{"type":"start"}})];
    let mut edges = vec![];
    let mut previous = "start".to_string();
    for (index, data) in actions.into_iter().enumerate() {
        let id = format!("action{index}");
        edges.push(json!({"source":previous,"target":id}));
        nodes.push(json!({"id":id,"data":data}));
        previous = id;
    }
    nodes.push(json!({"id":"answer","data":{"type":"answer","answer":"done"}}));
    edges.push(json!({"source":previous,"target":"answer"}));
    let root = Some(root.to_string_lossy().into_owned());
    // Keep audit storage outside the Git worktree, as in the application config directory.
    let config_dir = tempfile::tempdir().unwrap();
    let config = crate::config::Config {
        workspace_dir: root.as_ref().unwrap().into(),
        config_path: config_dir.path().join(".vibewindow.json"),
        ..Default::default()
    };
    let context = ToolRuntimeContext::new("actions", root.clone())
        .with_test_config(config)
        .with_tool_use_context(
            crate::tools::ToolUseContext::new("actions", root)
                .with_full_access_enabled(true)
                .with_bypass_non_cli_approval_for_turn(true),
        );
    let runtime = crate::workflow::WorkflowRuntime {
        video_edit_budget: Default::default(),
        provider: Arc::new(CloudFixtureProvider { invalid_assets: false }),
        model: "unused".into(),
        temperature: 0.0,
        tool_provider: Some(Arc::new(agent::NativeTools {
            context,
            allowed_tools: Some(json!(allowed)),
        })),
        agent_provider: None,
        knowledge_provider: None,
        document_extractor: None,
        pause_store: None,
    };
    crate::workflow::run_workflow(
        runtime,
        vw_api_types::workflow::WorkflowRunRequest {
            workflow_yaml: Some(
                json!({"workflow":{"graph":{"nodes":nodes,"edges":edges}}}).to_string(),
            ),
            max_steps: 20,
            ..Default::default()
        },
    )
    .await
    .unwrap()
}

fn action_repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Fixture").unwrap();
    config.set_str("user.email", "fixture@example.invalid").unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[]).unwrap();
    std::fs::write(dir.path().join("output.txt"), "result").unwrap();
    dir
}

#[tokio::test]
async fn task_pool_actions_absent_steps_have_no_side_effects() {
    let dir = action_repository();
    let repo = git2::Repository::open(dir.path()).unwrap();
    let before = repo.head().unwrap().target();
    let response = action_graph(dir.path(), vec![], vec![]).await;
    assert_eq!(response.status, vw_api_types::workflow::WorkflowRunStatus::Succeeded);
    assert_eq!(repo.head().unwrap().target(), before);
    assert!(repo.index().unwrap().is_empty());
    assert_eq!(std::fs::read_to_string(dir.path().join("output.txt")).unwrap(), "result");
    // The same graph runs in an ordinary directory without creating Git resources.
    let ordinary = tempfile::tempdir().unwrap();
    assert_eq!(action_graph(ordinary.path(), vec![], vec![]).await.status, response.status);
    assert!(!ordinary.path().join(".git").exists());
}

#[tokio::test]
async fn task_pool_actions_explicit_git_steps_use_native_tool_permissions() {
    let dir = action_repository();
    let repo = git2::Repository::open(dir.path()).unwrap();
    let before = repo.head().unwrap().target();
    let base = repo.head().unwrap().peel_to_commit().unwrap();
    let mut builder = repo.treebuilder(Some(&base.tree().unwrap())).unwrap();
    builder.insert("source.txt", repo.blob(b"source").unwrap(), 0o100644).unwrap();
    let tree = repo.find_tree(builder.write().unwrap()).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(Some("refs/heads/feature"), &signature, &signature, "source", &tree, &[&base])
        .unwrap();
    let target = repo.head().unwrap().shorthand().unwrap().to_string();
    let steps = vec![
        json!({"type":"tool","provider":"shiju","tool_name":"git_operations","inputs":{"operation":"add","paths":"output.txt"}}),
        json!({"type":"tool","provider":"shiju","tool_name":"git_operations","inputs":{"operation":"commit","message":"explicit"}}),
        json!({"type":"tool","provider":"shiju","tool_name":"git_operations","inputs":{"operation":"merge","source":"feature","target":target}}),
    ];
    let denied = action_graph(dir.path(), steps.clone(), vec![]).await;
    assert_eq!(denied.status, vw_api_types::workflow::WorkflowRunStatus::Failed);
    assert_eq!(repo.head().unwrap().target(), before);
    let allowed = action_graph(dir.path(), steps, vec!["git_operations"]).await;
    assert_eq!(allowed.status, vw_api_types::workflow::WorkflowRunStatus::Succeeded, "{allowed:?}");
    assert_ne!(repo.head().unwrap().target(), before);
    let merged = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(merged.parent_count(), 2);
    assert!(merged.tree().unwrap().get_name("source.txt").is_some());
    assert!(merged.tree().unwrap().get_name("output.txt").is_some());
}

#[tokio::test]
async fn task_pool_actions_upstream_failure_skips_downstream_send() {
    let dir = tempfile::tempdir().unwrap();
    let response = action_graph(dir.path(), vec![
        json!({"type":"tool","provider":"shiju","tool_name":"git_operations","inputs":{"operation":"commit","message":"non git"}}),
        json!({"type":"tool","provider":"shiju","tool_name":"send_message_to_thread","inputs":{"thread_id":"unavailable","prompt":"never send"}}),
    ], vec!["git_operations","send_message_to_thread"]).await;
    assert_eq!(response.status, vw_api_types::workflow::WorkflowRunStatus::Failed);
    assert!(!response.nodes.iter().any(|node| node.node_id == "action1"));
}
