// 覆盖会话流事件转换、历史整理及代理选项合并。

use super::*;
use gateway_resumable_stream::STREAM_REQUEST_ID_HEADER;

use gateway_session::stream_acceptance::docs_stream_request;

#[tokio::test(flavor = "multi_thread")]
async fn gateway_chat_persists_and_reports_session_git_line_counts() {
    use crate::session::llm::{StreamEvent, ToolCall};

    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let _streams = agent_session::processor::test_llm_stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().canonicalize().unwrap();
    let file = directory.join("example.txt");
    std::fs::write(&file, "keep\nold\n").unwrap();
    {
        let repo = git2::Repository::init(&directory).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("example.txt")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let author = git2::Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &author, &author, "initial", &tree, &[]).unwrap();
    }
    crate::config::update(serde_json::json!({
        "agents": {
            "main": {
                "provider": "model-recovery", "model": "original",
                "allowed_tools": ["apply_patch"]
            }
        }
    }))
    .await
    .unwrap();

    agent_session::processor::enqueue_test_llm_stream(vec![
        StreamEvent::ToolCalls(vec![ToolCall {
            kind: Default::default(),
            namespace: None,
            id: "edit-example".into(),
            name: "apply_patch".into(),
            arguments: serde_json::json!({
                "patchText": "*** Begin Patch\n*** Update File: example.txt\n@@\n keep\n-old\n+new\n+extra\n*** End Patch"
            })
            .to_string(),
        }]),
        StreamEvent::Done { usage: Default::default(), finish_reason: Some("tool_calls".into()) },
    ]);
    agent_session::processor::enqueue_test_llm_stream(vec![
        StreamEvent::Delta("updated the example".into()),
        StreamEvent::Done { usage: Default::default(), finish_reason: Some("stop".into()) },
    ]);

    let session_id = format!("ses_git_stats_{}", uuid::Uuid::new_v4());
    let request = GatewayChatStreamRequest {
        session_id: Some(session_id.clone().into()),
        messages: vec![serde_json::json!({"role":"user","content":"Update the example text."})],
        model: Some("model-recovery/original".into()),
        agent: Some("main".into()),
        allowed_tools: Some(vec!["apply_patch".into()]),
        options: Some(serde_json::json!({"full_access":true})),
        ..Default::default()
    };
    let directory = directory.to_string_lossy().into_owned();
    let mut reported = None;
    let mut reported_during_tools = false;
    tokio::time::timeout(
        std::time::Duration::from_secs(20),
        collect_gateway_chat_stream_with_observer(directory.clone(), request, |event| {
            if event["type"] == "chat.session_updated" {
                assert_eq!(event["session_id"], session_id);
                reported = Some(event["session"]["summary"].clone());
            }
            if event["type"] == "chat.post_tool_round" {
                let summary =
                    reported.as_ref().expect("counts must arrive before the next model round");
                assert_eq!(summary["additions"], 2);
                assert_eq!(summary["deletions"], 1);
                reported_during_tools = true;
            }
        }),
    )
    .await
    .expect("stream should finish")
    .unwrap();

    assert!(reported_during_tools);
    assert_eq!(std::fs::read_to_string(file).unwrap(), "keep\nnew\nextra\n");
    let reported = reported.expect("stream should report the persisted summary");
    assert_eq!(
        (reported["additions"].as_i64(), reported["deletions"].as_i64()),
        (Some(2), Some(1))
    );

    let tenant = crate::session::ui_store::current_tenant_scope();
    let scope = crate::session::ui_store::resolve_session_scope_id_for_tenant(
        &tenant,
        Some(&directory),
        None,
    )
    .unwrap();
    let stored = crate::session::ui_store::load_agent_session_scoped_for_tenant(
        &tenant,
        &session_id,
        Some(&scope),
    )
    .unwrap()
    .summary
    .expect("persisted summary");
    assert_eq!((stored.additions, stored.deletions, stored.files), (2, 1, 1));
}

#[test]
fn codex_skills_defaults_gateway_main_and_child_preload_before_first_request() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    let _streams = agent_session::processor::test_llm_stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let source = workspace.path().join("skills/review/SKILL.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(
        &source,
        "---\nname: review\ndescription: Review fixture\n---\nDEFAULT_REVIEW_BODY\n",
    )
    .unwrap();
    runtime.block_on(crate::config::update_project(workspace.path(), serde_json::json!({
        "agents": {
            "main": {"provider":"model-recovery","model":"original","allowed_tools":["file_read"],"allowed_skills":["review"],"skill_policy":"strict"},
            "reviewer": {"provider":"model-recovery","model":"original","allowed_tools":["skill","file_read"],"allowed_skills":["review"],"skill_policy":"strict"}
        }
    }))).unwrap();
    for name in ["main", "reviewer"] {
        let session_id = format!("ses_defaults_{}", uuid::Uuid::new_v4());
        agent_session::processor::enqueue_test_llm_stream(vec![
            agent_session::llm::StreamEvent::Delta("ready".into()),
            agent_session::llm::StreamEvent::Done {
                usage: Default::default(),
                finish_reason: Some("stop".into()),
            },
        ]);
        let request = GatewayChatStreamRequest {
            session_id: Some(session_id.clone().into()),
            messages: vec![
                serde_json::json!({"role":"user","content":"inspect the requested change"}),
            ],
            model: Some("model-recovery/original".into()),
            agent: Some(name.into()),
            allowed_tools: Some(vec!["skill".into(), "file_read".into()]),
            options: Some(serde_json::json!({"full_access":true})),
            ..Default::default()
        };
        let directory = workspace.path().to_string_lossy().to_string();
        let tenant = crate::session::ui_store::current_tenant_scope();
        runtime.block_on(async {
            let mut receiver = with_tenant_instance(tenant.clone(), directory.clone(), move || {
                Box::pin(async move {
                    gateway_session::stream::start_gateway_chat_stream::<Host>(tenant, directory, request, None, None).await
                })
            })
            .await
            .unwrap();
            loop {
                let payload =
                    tokio::time::timeout(std::time::Duration::from_secs(20), receiver.recv())
                        .await
                        .unwrap()
                        .expect("terminal gateway event");
                assert_ne!(payload["type"], "chat.error", "{payload}");
                if payload["type"] == "chat.done" {
                    break;
                }
            }
        });
        let captured = agent_session::processor::take_test_llm_messages(&session_id);
        assert_eq!(captured.len(), 1, "{name} request capture");
        let bodies = captured[0]
            .messages
            .iter()
            .filter(|message| {
                message["content"]
                    .as_str()
                    .is_some_and(|content| content.contains("DEFAULT_REVIEW_BODY"))
            })
            .collect::<Vec<_>>();
        assert_eq!(bodies.len(), 1, "{name} must preload configured default once");
        assert_eq!(bodies[0]["role"], "user");
        let tool_ids = agent_session::processor::llm_runner::take_test_llm_stream_tool_ids();
        assert_eq!(tool_ids.len(), 1);
        assert!(tool_ids[0].contains(&"skill".to_string()), "{name} must keep requested tools");
        assert!(tool_ids[0].contains(&"file_read".to_string()));
    }
}

#[tokio::test]
async fn resolve_delegate_request_overrides_without_agent_returns_explicit_values() {
    let overrides = gateway_session::stream::resolve_delegate_request_overrides::<Host>(
        Some("  ".to_string()),
        Some(vec![" shell ".to_string(), "shell".to_string(), " ".to_string()]),
        Some("provider/model".to_string()),
    )
    .await
    .expect("overrides");

    assert_eq!(overrides.agent, None);
    assert_eq!(overrides.model.as_deref(), Some("provider/model"));
    assert_eq!(overrides.allowed_tools, Some(vec!["shell".to_string()]));
    assert_eq!(overrides.chat_system_prompt, None);
    assert_eq!(overrides.temperature, None);
    assert_eq!(overrides.top_p, None);
}


#[tokio::test]
async fn resolve_delegate_request_overrides_rejects_unknown_agent() {
    let error = gateway_session::stream::resolve_delegate_request_overrides::<Host>(
        Some("agent-that-does-not-exist-for-stream-tests".to_string()),
        None,
        None,
    )
    .await
    .expect_err("unknown agent");

    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.to_string().contains("unknown agent:"));
}

#[tokio::test]
async fn chat_stream_rejects_missing_workspace_without_retryable_status() {
    gateway_session::stream_acceptance::chat_stream_rejects_missing_workspace_without_retryable_status::<Host>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn native_workflow_gateway_persists_bound_graph_without_llm_artifacts() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join(".shiju")).unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    std::fs::write(
        temp.path().join(".shiju/workflow_commands.json"),
        serde_json::json!({
            "platform_id":tenant.platform_id().as_ref(),
            "company_id":tenant.company_id().as_ref(),
            "commands":[{"command":"/shiju 校验","workflow_file":"preview.yaml"}]
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        temp.path().join("preview.yaml"),
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
    let session_id = format!("ses_workflow_gateway_{}", uuid::Uuid::new_v4());
    let request = GatewayChatStreamRequest {
        session_id: Some(session_id.clone().into()),
        messages: vec![serde_json::json!({"role":"user","content":"/shiju 校验 sp7"})],
        model: Some("model-recovery/original".into()),
        ..Default::default()
    };
    let directory = temp.path().to_string_lossy().to_string();
    let mut seen = Vec::new();
    let collected =
        collect_gateway_chat_stream_with_observer(directory.clone(), request, |event| {
            seen.push(event["type"].as_str().unwrap_or_default().to_string());
        })
        .await
        .expect("workflow gateway stream should finish and persist");
    assert!(collected.output.ends_with("verified preview"));
    assert_eq!(seen.last().map(String::as_str), Some("chat.done"));
    assert!(collected.message_id.is_some());
    let scope = crate::session::ui_store::resolve_session_scope_id_for_tenant(
        &tenant,
        Some(&directory),
        None,
    )
    .unwrap();
    let stored = crate::session::ui_store::load_session_scoped_for_tenant(
        &tenant,
        &session_id,
        Some(&scope),
    )
    .unwrap();
    assert_eq!(stored.steps.len(), 1);
    assert_eq!(stored.steps[0].finish_reason.as_deref(), Some("workflow"));
    assert!(stored.steps[0].tool_timings.iter().any(|tool| tool.name == "workflow"));
    assert!(stored.messages.last().unwrap().content.ends_with("verified preview"));
}

#[tokio::test]
async fn collect_gateway_chat_stream_runs_docs_request_and_persists_done_message_ids() {
    gateway_session::stream_acceptance::collect_gateway_chat_stream_runs_docs_request_and_persists_done_message_ids::<Host>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gateway_first_turn_registers_parent_before_thread_tool_use() {
    gateway_session::stream_acceptance::gateway_first_turn_registers_parent_before_thread_tool_use::<Host>().await;
}

#[tokio::test]
async fn chat_stream_returns_sse_response_for_docs_request() {
    gateway_session::stream_acceptance::chat_stream_returns_sse_response_for_docs_request::<Host>().await;
}

#[test]
fn agcollab_delivery_reaches_next_model_request() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    let _streams = agent_session::processor::test_llm_stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let source = workspace.path().join("skills/review/SKILL.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(
        &source,
        "---\nname: review\ndescription: Review fixture\n---\nDEFAULT_REVIEW_BODY\n",
    )
    .unwrap();
    runtime.block_on(crate::config::update_project(workspace.path(), serde_json::json!({
        "agents": {
            "main": {"provider":"model-recovery","model":"original","allowed_tools":["file_read"],"allowed_skills":["review"],"skill_policy":"strict"},
            "reviewer": {"provider":"model-recovery","model":"original","allowed_tools":["skill","file_read"],"allowed_skills":["review"],"skill_policy":"strict"}
        }
    }))).unwrap();
    for name in ["main"] {
        let session_id = format!("ses_defaults_{}", uuid::Uuid::new_v4());
        let coordinator =
            crate::session::child_agent::coordinator::global_child_agent_coordinator();
        let child = format!("child_{}", uuid::Uuid::new_v4());
        coordinator.register(&session_id, &child, "reviewer", "review").unwrap();
        let boundary = std::sync::Arc::new(
            agent_session::processor::llm_runner::ChannelCancelStream::default(),
        );
        agent_session::processor::llm_runner::register_channel_cancel_stream(
            &session_id,
            boundary.clone(),
        );
        agent_session::processor::enqueue_test_llm_stream(vec![
            agent_session::llm::StreamEvent::Delta("ready".into()),
            agent_session::llm::StreamEvent::Done {
                usage: Default::default(),
                finish_reason: Some("stop".into()),
            },
        ]);
        let request = GatewayChatStreamRequest {
            session_id: Some(session_id.clone().into()),
            messages: vec![
                serde_json::json!({"role":"user","content":"inspect the requested change"}),
            ],
            model: Some("model-recovery/original".into()),
            agent: Some(name.into()),
            allowed_tools: Some(vec!["skill".into(), "file_read".into()]),
            options: Some(serde_json::json!({"full_access":true})),
            ..Default::default()
        };
        let directory = workspace.path().to_string_lossy().to_string();
        let tenant = crate::session::ui_store::current_tenant_scope();
        runtime.block_on(async {
            let mut receiver = with_tenant_instance(tenant.clone(), directory.clone(), move || {
                Box::pin(async move {
                    gateway_session::stream::start_gateway_chat_stream::<Host>(tenant, directory, request, None, None).await
                })
            })
            .await
            .unwrap();
            tokio::time::timeout(std::time::Duration::from_secs(20), async {
                while !boundary.started.load(std::sync::atomic::Ordering::Acquire) {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            coordinator.mark_succeeded(&session_id, &child, "TEAM_RESULT_BODY").unwrap();
            boundary.release.store(true, std::sync::atomic::Ordering::Release);
            loop {
                let payload =
                    tokio::time::timeout(std::time::Duration::from_secs(20), receiver.recv())
                        .await
                        .unwrap()
                        .expect("terminal gateway event");
                assert_ne!(payload["type"], "chat.error", "{payload}");
                if payload["type"] == "chat.done" {
                    break;
                }
            }
        });
        let captured = agent_session::processor::take_test_llm_messages(&session_id);
        assert_eq!(captured.len(), 2, "{name} request capture");
        assert!(
            !captured[0]
                .messages
                .iter()
                .any(|m| m["content"].as_str().is_some_and(|s| s.contains("TEAM_RESULT_BODY")))
        );
        assert!(captured[1].messages.iter().any(|m| {
            m["role"] == "user"
                && m["content"]
                    .as_str()
                    .is_some_and(|s| s.contains("TEAM_RESULT_BODY") && s.contains(&child))
        }));
        let bodies = captured[0]
            .messages
            .iter()
            .filter(|message| {
                message["content"]
                    .as_str()
                    .is_some_and(|content| content.contains("DEFAULT_REVIEW_BODY"))
            })
            .collect::<Vec<_>>();
        assert_eq!(bodies.len(), 1, "{name} must preload configured default once");
        assert_eq!(bodies[0]["role"], "user");
        let tool_ids = agent_session::processor::llm_runner::take_test_llm_stream_tool_ids();
        assert_eq!(tool_ids.len(), 1);
        assert!(tool_ids[0].contains(&"skill".to_string()), "{name} must keep requested tools");
        assert!(tool_ids[0].contains(&"file_read".to_string()));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_entry_gateway_unbound_projects_template_edit_menu_and_persistence() {
    use crate::workflow::{templates::*, video_edit::entry_test_transport};
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let home = tempfile::tempdir().unwrap();
    let source = home.path().join("source");
    std::fs::create_dir(&source).unwrap();
    let a = source.join("first.mp4");
    let b = source.join("second.mp4");
    std::fs::write(&a, b"external video fixture one").unwrap();
    std::fs::write(&b, b"external video fixture two").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o555)).unwrap();
    }
    let tenant = crate::session::ui_store::current_tenant_scope();
    let database =
        crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home).unwrap();
    let seed = VideoEditTemplateSeed::production().unwrap();
    let initial = seed.load(database.clone(), tenant.clone()).await.unwrap();
    for index in 0..3 {
        let project = home.path().join(format!("unbound-{index}"));
        std::fs::create_dir(&project).unwrap();
        let path = if index == 2 { &b } else { &a };
        let query = if index == 1 {
            format!(
                "剪辑 \"{}\"\n\n<session_control_selection>\n技能：shiju\n</session_control_selection>",
                path.display()
            )
        } else {
            format!("/shiju 剪辑 \"{}\"", path.display())
        };
        let session = format!("ses_video_entry_{}", uuid::Uuid::new_v4());
        let _external = entry_test_transport::install(&session);
        let request = GatewayChatStreamRequest {
            session_id: Some(session.clone().into()),
            messages: vec![serde_json::json!({"role":"user","content":query})],
            model: Some("model-recovery/original".into()),
            options: Some(serde_json::json!({"full_access":true})),
            ..Default::default()
        };
        let directory = project.to_string_lossy().to_string();
        let mut events = Vec::new();
        let collected =
            collect_gateway_chat_stream_with_observer(directory.clone(), request, |event| {
                events.push(event.clone())
            })
            .await
            .unwrap();
        assert!(
            collected.output.contains("https://external.invalid/s/test-preview"),
            "{}",
            collected.output
        );
        assert_eq!(events.last().unwrap()["type"], "chat.done");
        let progress = events
            .iter()
            .filter_map(|e| e.get("payload")?.get("metadata")?.get("video_edit_progress"))
            .collect::<Vec<_>>();
        assert!(!progress.is_empty(), "native progress must reach the real chat channel");
        let last = progress.last().unwrap();
        assert_eq!(last["stage"], "预览完成");
        assert!(last["total"].as_u64().unwrap() > 0);
        assert_eq!(last["completed"], last["total"]);
        assert_eq!(last["failed"], 0);
        assert_eq!(
            last["template_sha256"],
            seed.load(database.clone(), tenant.clone()).await.unwrap().digest
        );
        assert!(last["peak"].as_u64().unwrap() <= 5);

        assert!(!project.join(".shiju/workflow_commands.json").exists());
        let scope = crate::session::ui_store::resolve_session_scope_id_for_tenant(
            &tenant,
            Some(&directory),
            None,
        )
        .unwrap();
        let stored = crate::session::ui_store::load_session_scoped_for_tenant(
            &tenant,
            &session,
            Some(&scope),
        )
        .unwrap();
        assert_eq!(stored.steps[0].finish_reason.as_deref(), Some("workflow"));
        assert!(stored.steps[0].tool_timings.len() >= 10, "real graph nodes must be visible");
        assert!(stored.messages.last().unwrap().content.contains("test-preview"));
        if index == 1 {
            let current = seed.load(database.clone(), tenant.clone()).await.unwrap();
            assert_eq!(current.digest, initial.digest);
            let payload = vw_api_workflow::WorkflowRecordUpsertBody {
                uuid: Some(VIDEO_EDIT_TEMPLATE_UUID.into()),
                name: VIDEO_EDIT_TEMPLATE_NAME.into(),
                description: String::new(),
                workflow_yaml: current
                    .record
                    .workflow_yaml
                    .replace("给出简洁的全片编辑简报", "EDITED_BRIEF"),
            };
            let router = crate::gateway::api::handlers::workflow::applications_router()
                .layer(axum::Extension(tenant.clone()))
                .layer(axum::Extension(seed.clone()));
            let path = format!("/workflow/applications/{VIDEO_EDIT_TEMPLATE_UUID}");
            let request = axum::http::Request::builder()
                .method("PUT")
                .uri(&path)
                .header("content-type", "application/json")
                .header("x-workflow-expected-updated-at-ms", current.record.updated_at_ms)
                .body(axum::body::Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap();
            let response = tower::ServiceExt::oneshot(router.clone(), request).await.unwrap();
            assert_eq!(response.status(), axum::http::StatusCode::OK);
            let response = tower::ServiceExt::oneshot(
                router,
                axum::http::Request::builder().uri(&path).body(axum::body::Body::empty()).unwrap(),
            )
            .await
            .unwrap();
            assert_eq!(response.status(), axum::http::StatusCode::OK);
            let record: vw_api_workflow::WorkflowRecord = serde_json::from_slice(
                &axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap(),
            )
            .unwrap();
            assert_eq!(record.uuid, VIDEO_EDIT_TEMPLATE_UUID);
            assert!(record.workflow_yaml.contains("EDITED_BRIEF"));
        }
    }
    let edited = seed.load(database.clone(), tenant.clone()).await.unwrap();
    assert_ne!(initial.digest, edited.digest);
    // The actual production preparation ledger proves path/template snapshots did not mix.
    let conn = database.connect_sqlite().unwrap();
    let rows = conn
        .prepare("SELECT payload FROM workflow_video_edit_runs")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let runs = rows
        .iter()
        .map(|r| serde_json::from_str::<serde_json::Value>(r).unwrap())
        .collect::<Vec<_>>();
    assert!(
        runs.iter()
            .any(|r| r["source"]["path"] == a.canonicalize().unwrap().to_string_lossy().as_ref()
                && r["template_sha256"] == initial.digest)
    );
    assert!(
        runs.iter()
            .any(|r| r["source"]["path"] == b.canonicalize().unwrap().to_string_lossy().as_ref()
                && r["template_sha256"] == edited.digest)
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_entry_gateway_explicit_unavailable_model_does_not_start_graph() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let request = GatewayChatStreamRequest {
        messages: vec![serde_json::json!({"role":"user","content":"/shiju 剪辑 missing.mp4"})],
        model: Some("missing-provider/missing-model".into()),
        ..Default::default()
    };
    let mut events = Vec::new();
    let result = collect_gateway_chat_stream_with_observer(
        root.path().to_string_lossy().into(),
        request,
        |e| events.push(e.clone()),
    )
    .await;
    assert!(result.is_err());
    assert!(!events.iter().any(|e| e["type"] == "chat.done"));
    assert!(!root.path().join(".shiju/workflow_commands.json").exists());
}

pub(crate) async fn collect_video_edit_test_stream(
    dir: String,
    body: GatewayChatStreamRequest,
    cancel: tokio_util::sync::CancellationToken,
    mut observe: impl FnMut(&Value),
) -> Result<GatewayChatStreamCollected, ApiError> {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (tx, rx) = mpsc::unbounded_channel();
    with_tenant_instance(tenant.clone(), dir.clone(), move || {
        Box::pin(async move {
            gateway_session::stream::start_gateway_chat_producer::<Host>(
                tenant,
                dir,
                body,
                None,
                Some(cancel),
                None,
                move |payload, _| tx.send(payload).is_ok(),
            )
            .await
        })
    })
    .await?;
    collect_gateway_chat_stream_payloads(rx, |e| observe(e)).await
}

#[test]
fn agent_native_defaults_reach_registered_tools_and_prompt() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    let _streams = agent_session::processor::test_llm_stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let script = workspace.path().join("fixture.py");
    std::fs::write(
        &script,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/session/native_defaults_mcp_fixture.py"
        )),
    )
    .unwrap();
    std::fs::write(
        workspace.path().join(".vwacprc.json"),
        serde_json::to_vec(&serde_json::json!({
            "mcpServers":[
                {"name":"selected", "command":"python3", "args":[script]},
                {"name":"unselected", "command":"python3", "args":[script]}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let source = workspace.path().join("skills/review/SKILL.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(
        &source,
        "---\nname: review\ndescription: Review fixture\n---\nDEFAULT_REVIEW_BODY\n",
    )
    .unwrap();
    runtime.block_on(crate::config::update(serde_json::json!({
        "agents": {
            "main": {"provider":"model-recovery","model":"unavailable-legacy","allowed_tools":["file_read"],"defaults":{"mcp":{"mode":"fixed","value":["selected"]},"model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}},"skills":{"mode":"fixed","value":["review"]}},"system_prompt":"NATIVE_DEFAULT_RULE","skill_policy":"strict", "allowed_skills":["review"]},
            "reviewer": {"provider":"model-recovery","model":"unavailable-legacy","allowed_tools":["skill","file_read"],"defaults":{"mcp":{"mode":"fixed","value":[]},"model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}},"skills":{"mode":"fixed","value":["review"]}},"system_prompt":"NATIVE_DEFAULT_RULE","skill_policy":"strict", "allowed_skills":["review"]}
        }
    }))).unwrap();
    for name in ["researcher", "coder", "explore"] {
        runtime.block_on(crate::config::update(serde_json::json!({
            "agents": {name: {"defaults": {
                "mcp":{"mode":"fixed","value":[]},
                "model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}},
                "skills":{"mode":"fixed","value":["review"]}
            }, "system_prompt":"NATIVE_DEFAULT_RULE", "skill_policy":"strict", "allowed_skills":["review"]}}
        }))).unwrap();
    }
    for name in ["main", "reviewer", "researcher", "coder", "explore"] {
        let session_id = format!("ses_defaults_{}", uuid::Uuid::new_v4());
        agent_session::processor::enqueue_test_llm_stream(vec![
            agent_session::llm::StreamEvent::Delta("ready".into()),
            agent_session::llm::StreamEvent::Done {
                usage: Default::default(),
                finish_reason: Some("stop".into()),
            },
        ]);
        let request = GatewayChatStreamRequest {
            session_id: Some(session_id.clone().into()),
            messages: vec![
                serde_json::json!({"role":"user","content":"inspect the requested change"}),
            ],
            model: None,
            agent: Some(name.into()),
            allowed_tools: if name == "main" {
                None
            } else {
                Some(vec!["skill".into(), "file_read".into(), "AgentTool".into()])
            },
            options: Some(serde_json::json!({"full_access":false})),
            ..Default::default()
        };
        let directory = workspace.path().to_string_lossy().to_string();
        let tenant = crate::session::ui_store::current_tenant_scope();
        runtime.block_on(async {
            let mut receiver = with_tenant_instance(tenant.clone(), directory.clone(), move || {
                Box::pin(async move {
                    gateway_session::stream::start_gateway_chat_stream::<Host>(tenant, directory, request, None, None).await
                })
            })
            .await
            .unwrap();
            loop {
                let payload =
                    tokio::time::timeout(std::time::Duration::from_secs(20), receiver.recv())
                        .await
                        .unwrap()
                        .expect("terminal gateway event");
                assert_ne!(payload["type"], "chat.error", "{payload}");
                if payload["type"] == "chat.done" {
                    break;
                }
            }
        });
        let captured = agent_session::processor::take_test_llm_messages(&session_id);
        assert_eq!(captured.len(), 1, "{name} request capture");
        assert!(captured[0].system.iter().any(|body| body.contains("NATIVE_DEFAULT_RULE")));
        assert_eq!(captured[0].model.as_deref(), Some("model-recovery/original"));
        let discovery = captured[0].tool_specs.iter().find(|spec| spec.id == "tool_search");
        if name == "main" {
            let discovery = discovery.expect("selected MCP must reach the first model request");
            assert!(discovery.description.contains("selected"));
            assert!(!discovery.description.contains("unselected"));
        } else {
            assert!(discovery.is_none(), "fixed empty MCP list must isolate worker sessions");
        }

        let bodies = captured[0]
            .messages
            .iter()
            .filter(|message| {
                message["content"]
                    .as_str()
                    .is_some_and(|content| content.contains("DEFAULT_REVIEW_BODY"))
            })
            .collect::<Vec<_>>();
        assert_eq!(bodies.len(), 1, "{name} must preload configured default once");
        assert_eq!(bodies[0]["role"], "user");
        let tool_ids = agent_session::processor::llm_runner::take_test_llm_stream_tool_ids();
        assert_eq!(tool_ids.len(), 1);
        assert!(tool_ids[0].contains(&"skill".to_string()), "{name} must keep requested tools");
        assert!(tool_ids[0].contains(&"file_read".to_string()));
        if name != "main" {
            assert!(
                !tool_ids[0].contains(&"AgentTool".to_string()),
                "{name} must not receive primary agent tool eligibility"
            );
        }
    }
}

#[tokio::test]
async fn agent_native_defaults_do_not_grant_disabled_mcp() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join(".vwacprc.json"), serde_json::to_vec(&serde_json::json!({
        "mcpServers":[{"name":"disabled-server", "command":"missing-disabled-command", "enabled":false}]
    })).unwrap()).unwrap();
    crate::config::update(serde_json::json!({
        "agents": {"main": {"defaults": {"mcp": {"mode":"fixed", "value":["disabled-server"]}}}}
    }))
    .await
    .unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    let directory = workspace.path().to_string_lossy().to_string();
    let result = with_tenant_instance(tenant, directory, || {
        Box::pin(async {
            gateway_session::stream::resolve_delegate_request_overrides::<Host>(Some("main".into()), None, None).await
        })
    })
    .await;
    assert!(result.is_err(), "a missing or disabled MCP default must reject admission");
}

#[tokio::test]
async fn agent_session_identity_rejects_forged_ui_binding() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    gateway_session::stream_acceptance::agent_session_identity_rejects_forged_ui_binding::<Host>().await;
}

#[test]
fn agent_session_identity_survives_profile_edits() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    let _streams = agent_session::processor::test_llm_stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let session_id = format!("ses_binding_{}", uuid::Uuid::new_v4());
    for rule in ["ORIGINAL_BINDING_RULE", "EDITED_PROFILE_RULE"] {
        runtime.block_on(crate::config::update(serde_json::json!({
            "agents":{"main":{
                "system_prompt":rule,
                "defaults":{
                    "model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}},
                    "skills":{"mode":"fixed","value":[]},
                    "mcp":{"mode":"fixed","value":[]}
                }
            }}
        }))).unwrap();
        agent_session::processor::enqueue_test_llm_stream(vec![
            agent_session::llm::StreamEvent::Delta("ready".into()),
            agent_session::llm::StreamEvent::Done {
                usage: Default::default(),
                finish_reason: Some("stop".into()),
            },
        ]);
        let request = GatewayChatStreamRequest {
            session_id: Some(session_id.clone().into()),
            agent: Some("main".into()),
            messages: vec![serde_json::json!({"role":"user","content":"hello"})],
            options: Some(serde_json::json!({"full_access":false})),
            ..Default::default()
        };
        let directory = workspace.path().to_string_lossy().into_owned();
        let tenant = crate::session::ui_store::current_tenant_scope();
        runtime.block_on(async {
            let mut receiver = with_tenant_instance(tenant.clone(), directory.clone(), move || {
                Box::pin(async move {
                    gateway_session::stream::start_gateway_chat_stream::<Host>(tenant, directory, request, None, None).await
                })
            })
            .await
            .unwrap();
            loop {
                let payload =
                    tokio::time::timeout(std::time::Duration::from_secs(20), receiver.recv())
                        .await
                        .unwrap()
                        .expect("terminal gateway event");
                assert_ne!(payload["type"], "chat.error", "{payload}");
                if payload["type"] == "chat.done" {
                    break;
                }
            }
        });
        let captured = agent_session::processor::take_test_llm_messages(&session_id);
        assert_eq!(captured.len(), 1);
        assert!(
            captured[0].system.iter().any(|body| body.contains("ORIGINAL_BINDING_RULE")),
            "resuming the same session must retain its original rules after profile edits"
        );
        assert!(!captured[0].system.iter().any(|body| body.contains("EDITED_PROFILE_RULE")));
    }
    let tenant = crate::session::ui_store::current_tenant_scope();
    let directory = workspace.path().to_string_lossy().into_owned();
    runtime
        .block_on(with_tenant_instance(tenant.clone(), directory, move || {
            Box::pin(async move {
                let fork = agent_session::session::fork(&session_id, None).await.unwrap();
                assert!(fork.parent_id.is_none(), "fork must retain existing parent semantics");
                let fork_binding =
                    crate::session::agent_binding::load(&fork.id).await.unwrap().unwrap();
                assert_eq!(fork_binding.system_prompt.as_deref(), Some("ORIGINAL_BINDING_RULE"));
                crate::session::ui_config::set_config_field_result(
                    &tenant,
                    "permission",
                    serde_json::json!({"*":"deny"}),
                )
                .unwrap();
                crate::config::update(serde_json::json!({
                    "agents":{"main":{"enabled":false}}
                }))
                .await
                .unwrap();
                let mut resumed = GatewayChatStreamRequest {
                    session_id: Some(session_id.clone().into()),
                    agent: Some("main".into()),
                    ..Default::default()
                };
                let overrides = gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut resumed, None).await.unwrap();
                assert_eq!(overrides.chat_system_prompt.as_deref(), Some("ORIGINAL_BINDING_RULE"));
                let mut switched = resumed.clone();
                switched.model = Some("model-recovery/different".into());
                let error = gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut switched, None).await.err().unwrap();
                assert!(error.message.contains("changing a session model requires a new session"));
                let mut config = crate::config::get_current().await.unwrap();
                crate::session::agent_binding::apply_to_config(&session_id, &mut config)
                    .await
                    .unwrap();
                let defaults = crate::session::capability_policy::native_agent_defaults(
                    &config,
                    "main",
                    None,
                    workspace.path(),
                )
                .await
                .unwrap()
                .unwrap();
                assert_eq!(
                    defaults.effective.permission_action("shell", "*"),
                    crate::permission::next::Action::Deny,
                    "the latest security revocation must still apply to the stored snapshot"
                );
                let mut new_session =
                    GatewayChatStreamRequest { agent: Some("main".into()), ..Default::default() };
                assert!(
                    gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut new_session, None).await.is_err(),
                    "disabled agents cannot start new sessions"
                );
                Ok(())
            })
        }))
        .unwrap();
}

#[tokio::test]
async fn agent_session_identity_derives_backend_and_rejects_conflicts() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    gateway_session::stream_acceptance::agent_session_identity_derives_backend_and_rejects_conflicts::<Host>().await;
}

#[tokio::test]
async fn acp_background_approval_gateway_uses_turn_grant_after_agent_resolution() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let workspace = tempfile::tempdir().unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    with_tenant_instance(tenant, workspace.path().to_string_lossy().into_owned(), || {
        Box::pin(async {
            let mut request = GatewayChatStreamRequest {
                agent: Some("acp-codex".into()),
                options: Some(serde_json::json!({"full_access":true})),
                ..Default::default()
            };
            gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut request, Some(false)).await.unwrap();
            assert_eq!(request.acp_agent.as_deref(), Some("codex"));
            let options = request.options.as_ref().unwrap();
            assert_eq!(options["full_access"], false);
            assert_eq!(options["acp_non_interactive_permissions"], "deny");

            request.options = Some(serde_json::json!({
                "acp_permission_mode":"deny-all",
                "acp_non_interactive_permissions":"fail"
            }));
            gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut request, Some(true)).await.unwrap();
            let options = request.options.as_ref().unwrap();
            assert_eq!(options["full_access"], true);
            assert_eq!(options["acp_permission_mode"], "deny-all");
            assert_eq!(options["acp_non_interactive_permissions"], "fail");
            let session = request.session_id.as_ref().unwrap().as_ref();
            let project = instance::project().unwrap();
            assert!(
                !crate::session::agent_binding::confirmed_full_access(&project.id, session)
                    .await
                    .unwrap()
            );
            Ok(())
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn agent_config_cleanup_full_access_gateway_freezes_defaults_and_session_choice() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"agents":{"main":{"defaults":{"full_access":true}}}}))
        .await
        .unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    with_tenant_instance(tenant, workspace.path().to_string_lossy().into_owned(), || {
        Box::pin(async {
            let mut request = GatewayChatStreamRequest::default();
            gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut request, None).await.unwrap();
            assert_eq!(request.options.as_ref().unwrap()["full_access"], true);
            request.options = Some(serde_json::json!({"full_access":false}));
            gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut request, None).await.unwrap();
            assert_eq!(request.options.as_ref().unwrap()["full_access"], false);
            let mut untouched = GatewayChatStreamRequest::default();
            gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut untouched, None).await.unwrap();
            crate::config::update(
                serde_json::json!({"agents":{"main":{"defaults":{"full_access":false}}}}),
            )
            .await
            .unwrap();
            request.options = None;
            gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut request, None).await.unwrap();
            assert_eq!(request.options.as_ref().unwrap()["full_access"], false);
            untouched.options = None;
            gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut untouched, None).await.unwrap();
            assert_eq!(untouched.options.as_ref().unwrap()["full_access"], true);
            let mut fresh = GatewayChatStreamRequest::default();
            gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut fresh, None).await.unwrap();
            assert_eq!(fresh.options.as_ref().unwrap()["full_access"], false);
            Ok(())
        })
    })
    .await
    .unwrap();
}

#[test]
fn native_tool_search_core_loop_gateway_history_reaches_model_as_native_item() {
    let item = serde_json::json!({"type":"tool_search_output", "call_id":"search-id", "status":"completed", "execution":"client", "tools":[]});
    let history = prepare_stream_history(
        vec![serde_json::json!({"role":"native", "content":item.to_string()})],
        true,
        true,
    );
    assert_eq!(history.history.len(), 1);
    assert!(matches!(history.history[0].role, ui_models::ChatRole::Tool));
    let message = crate::session::session::Message {
        role: crate::session::session::Role::Tool,
        content: history.history[0].content.clone(),
    };
    let converted =
        crate::session::processor::llm_messages::session_message_to_llm_message(&message).unwrap();
    assert_eq!(converted["role"], "native");
    let restored: crate::session::llm::native_tools::NativeHistoryItem =
        serde_json::from_str(converted["content"].as_str().unwrap()).unwrap();
    assert!(
        matches!(restored.into_message(), aisdk::core::Message::ToolSearchOutput(output) if output.call_id.as_deref() == Some("search-id"))
    );
    let user = crate::session::session::Message {
        role: crate::session::session::Role::User,
        content: item.to_string(),
    };
    assert_eq!(
        crate::session::processor::llm_messages::session_message_to_llm_message(&user).unwrap()["role"],
        "user"
    );
}

#[tokio::test]
async fn agent_model_inheritance_acp_new_and_reopened_sessions_preserve_priority() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"agents":{
        "custom-model": {"runtime":{"kind":"acp","backend":"codex"},
            "defaults":{"model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}}}},
        "custom-inherit": {"runtime":{"kind":"acp","backend":"codex"}},
        "custom-forced": {"runtime":{"kind":"acp","backend":"codex"},
            "provider":"model-recovery","model":"original","force_model_selection":true}
    }})).await.unwrap();
    gateway_session::stream_acceptance::agent_model_inheritance_acp_new_and_reopened_sessions_preserve_priority::<Host>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gateway_first_response_replays_the_same_session_and_rejects_other_directories() {

    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "http://127.0.0.1:1/v1",
        None,
    )
    .await;
    gateway_session::stream_acceptance::gateway_first_response_replays_the_same_session_and_rejects_other_directories::<Host>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gateway_first_response_failure_preserves_history_until_same_session_retries() {
    use crate::session::llm::StreamEvent;
    use crate::session::ui_store;

    let _env = crate::test_support::global_async_env_lock().lock().await;
    struct RestoreTitleFixtureKey(Option<String>);
    impl Drop for RestoreTitleFixtureKey {
        fn drop(&mut self) {
            match self.0.take() {
                Some(key) => agent_env::set("VW_MODEL_RECOVERY_TEST_KEY", key),
                None => agent_env::remove("VW_MODEL_RECOVERY_TEST_KEY"),
            }
        }
    }
    let _key = RestoreTitleFixtureKey(agent_env::get("VW_MODEL_RECOVERY_TEST_KEY"));
    agent_env::set("VW_MODEL_RECOVERY_TEST_KEY", "fixture-key");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let (title_tx, mut title_requests) = tokio::sync::mpsc::unbounded_channel();
    let provider = axum::Router::new()
        .route(
            "/v1/models",
            axum::routing::get(|| async {
                Json(serde_json::json!({"data": [{"id": "original"}]}))
            }),
        )
        .fallback(move |Json(request): Json<Value>| {
            let title_tx = title_tx.clone();
            async move {
                let _ = title_tx.send(request);
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": {
                        "message": "title provider rejected the request"
                    }})),
                )
            }
        });
    let provider = tokio::spawn(async move { axum::serve(listener, provider).await });
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        &endpoint,
        Some("fixture-key"),
    )
    .await;
    let _streams = agent_session::processor::test_llm_stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().canonicalize().unwrap().to_string_lossy().into_owned();
    let tenant = ui_store::current_tenant_scope();
    let scope =
        ui_store::resolve_session_scope_id_for_tenant(&tenant, Some(&directory), None).unwrap();
    let session_id = format!("ses_retry_{}", uuid::Uuid::new_v4());
    let request = |content: &str| GatewayChatStreamRequest {
        session_id: Some(session_id.clone().into()),
        messages: vec![serde_json::json!({"role": "user", "content": content})],
        model: Some("model-recovery/original".into()),
        ..Default::default()
    };
    let query = || Query(InstanceQuery { directory: Some(directory.clone()) });
    agent_session::processor::enqueue_test_llm_stream(vec![StreamEvent::Error(
        agent_session::message::AssistantError::Unknown { message: "fixture failure".into() },
    )]);
    let response = chat_stream(query(), HeaderMap::new(), Json(request("failed prompt")))
        .await
        .unwrap()
        .into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(std::str::from_utf8(&body).unwrap().contains("chat.error"));
    let info =
        ui_store::load_agent_session_scoped_for_tenant(&tenant, &session_id, Some(&scope)).unwrap();
    assert!(info.is_first_response_pending());
    assert!(info.title.is_empty());
    assert!(
        ui_store::load_session_meta_scoped_for_tenant(&tenant, &session_id, Some(&scope))
            .unwrap()
            .is_some()
    );

    let failed = ui_store::load_session_scoped_for_tenant(&tenant, &session_id, Some(&scope))
        .expect("failed turn remains readable");
    assert!(failed.messages.iter().any(|message| message.content == "failed prompt"));
    assert!(failed.messages.iter().any(|message| message.content.contains("fixture failure")));

    assert!(title_requests.try_recv().is_err());

    agent_session::processor::enqueue_test_llm_stream(vec![
        StreamEvent::Delta("successful answer".into()),
        StreamEvent::Done { usage: Default::default(), finish_reason: Some("stop".into()) },
    ]);
    let response = chat_stream(query(), HeaderMap::new(), Json(request("successful retry")))
        .await
        .unwrap()
        .into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(std::str::from_utf8(&body).unwrap().contains("chat.done"), "{body:?}");
    let info =
        ui_store::load_agent_session_scoped_for_tenant(&tenant, &session_id, Some(&scope)).unwrap();
    assert!(!info.is_first_response_pending());
    assert!(info.title.is_empty());
    let preview = ui_store::load_session_meta_scoped_for_tenant(&tenant, &session_id, Some(&scope))
        .unwrap()
        .unwrap();
    assert_eq!(preview.last_content.as_deref(), Some("successful retry"));
    let title_request =
        tokio::time::timeout(std::time::Duration::from_secs(10), title_requests.recv())
            .await
            .expect("the first committed turn should trigger title generation")
            .unwrap();
    let title_prompt = title_request.to_string();
    assert!(title_prompt.contains("successful retry"));
    assert!(!title_prompt.contains("failed prompt"));
    assert!(title_request["tools"].as_array().is_none_or(Vec::is_empty));
    provider.abort();
    let _ = provider.await;
}

#[tokio::test]
async fn gateway_first_response_missing_replay_does_not_start_a_new_turn() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    gateway_session::stream_acceptance::gateway_first_response_missing_replay_does_not_start_a_new_turn::<Host>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gateway_first_response_disconnect_after_tool_start_replays_cancellation_without_publishing()
 {
    use crate::session::llm::{StreamEvent, ToolCall};
    use crate::session::ui_store;
    use futures_util::StreamExt as _;

    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "http://127.0.0.1:1/v1",
        None,
    )
    .await;
    let _streams = agent_session::processor::test_llm_stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().canonicalize().unwrap().to_string_lossy().into_owned();
    crate::config::update(serde_json::json!({
        "agents": {
            "main": {
                "provider": "model-recovery", "model": "original", "allowed_tools": ["bash"]
            }
        }
    }))
    .await
    .unwrap();
    agent_session::processor::enqueue_test_llm_stream(vec![
        StreamEvent::ToolCalls(vec![ToolCall {
            kind: Default::default(),
            namespace: None,
            id: "first-response-tool".into(),
            name: "bash".into(),
            arguments: serde_json::json!({
                "command": "sleep 30",
                "description": "wait for stream cancellation",
                "yield_time_ms": 10000
            })
            .to_string(),
        }]),
        StreamEvent::Done { usage: Default::default(), finish_reason: Some("tool_calls".into()) },
    ]);

    let request_id = uuid::Uuid::new_v4().to_string();
    let mut headers = HeaderMap::new();
    headers.insert(STREAM_REQUEST_ID_HEADER, request_id.parse().unwrap());
    let request = GatewayChatStreamRequest {
        messages: vec![serde_json::json!({"role": "user", "content": "Run a tool"})],
        model: Some("model-recovery/original".into()),
        allowed_tools: Some(vec!["bash".into()]),
        options: Some(serde_json::json!({"full_access": true})),
        ..Default::default()
    };
    let query = || Query(InstanceQuery { directory: Some(directory.clone()) });
    let response =
        chat_stream(query(), headers.clone(), Json(request.clone())).await.unwrap().into_response();
    let session_id = response.headers()["x-vw-session-id"].to_str().unwrap().to_string();
    let mut stream = response.into_body().into_data_stream();
    let cursor = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        let mut body = String::new();
        while let Some(chunk) = stream.next().await {
            body.push_str(std::str::from_utf8(&chunk.unwrap()).unwrap());
            if let Some(event) = body.split("\n\n").find(|event| {
                event.contains(r#""type":"chat.tool""#) && event.contains(r#""kind":"start""#)
            }) {
                return event
                    .lines()
                    .find_map(|line| line.strip_prefix("id:").map(str::trim))
                    .unwrap()
                    .to_string();
            }
        }
        panic!("tool start not observed: {body}");
    })
    .await
    .unwrap();
    drop(stream);

    let tenant = ui_store::current_tenant_scope();
    let Json(stop) =
        chat_stream_stop(Extension(tenant.clone()), axum::extract::Path(request_id)).await;
    assert_eq!(stop["cancelled"], true);
    headers.insert("last-event-id", cursor.parse().unwrap());
    let resumed = chat_stream(query(), headers, Json(request)).await.unwrap().into_response();
    let body = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        axum::body::to_bytes(resumed.into_body(), usize::MAX),
    )
    .await
    .unwrap()
    .unwrap();
    let body = std::str::from_utf8(&body).unwrap();
    assert!(body.contains(r#""error":"cancelled""#), "{body}");
    assert!(!body.contains(r#""type":"chat.done""#), "{body}");
    let scope =
        ui_store::resolve_session_scope_id_for_tenant(&tenant, Some(&directory), None).unwrap();
    let info =
        ui_store::load_agent_session_scoped_for_tenant(&tenant, &session_id, Some(&scope)).unwrap();
    assert!(info.is_first_response_pending());
    assert!(info.title.is_empty());
    assert!(
        ui_store::load_session_meta_scoped_for_tenant(&tenant, &session_id, Some(&scope))
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn task_pool_agent_uses_existing_definition_and_defaults() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"agents":{"main":{"defaults":{"full_access":false},"allowed_tools":["read"],"permission":{"*":"deny"}},"task-reviewer":{"enabled":true,"runtime":{"kind":"native"},"provider":"model-recovery","model":"original","allowed_tools":["read"],"permission":{"*":"deny"},"defaults":{"full_access":false}}}})).await.unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    with_tenant_instance(tenant, workspace.path().to_string_lossy().into_owned(), move || {
        Box::pin(async move {
            for agent in ["main", "task-reviewer", "acp-codex"] {
                let mut request = GatewayChatStreamRequest {
                    agent: Some(agent.into()),
                    session_id: Some(format!("agent-scheduler-task-{agent}").into()),
                    ..Default::default()
                };
                let resolved = gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut request, None).await.unwrap();
                assert_eq!(resolved.agent.as_deref(), Some(agent));
                let binding = crate::session::agent_binding::load(
                    request.session_id.as_ref().unwrap().as_ref(),
                )
                .await
                .unwrap()
                .unwrap();
                assert_eq!(binding.agent, agent);
                if agent != "acp-codex" {
                    if agent == "task-reviewer" {
                        assert_eq!(resolved.model.as_deref(), Some("model-recovery/original"));
                    }
                    assert_eq!(binding.allowed_tools, vec!["read"]);
                    assert_eq!(request.options.as_ref().unwrap()["full_access"], false);
                    let rules = crate::agent::permission_rules(agent).await.unwrap();
                    assert_eq!(
                        crate::permission::next::evaluate("shell", "*", &[rules]).action,
                        crate::permission::next::Action::Deny
                    );
                } else {
                    assert_eq!(request.acp_agent.as_deref(), Some("codex"));
                }
            }
            Ok(())
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn task_pool_agent_rejects_unknown_or_disabled_definition() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"agents":{"main":{"enabled":false}}})).await.unwrap();
    gateway_session::stream_acceptance::task_pool_agent_rejects_unknown_or_disabled_definition::<Host>().await;
}
