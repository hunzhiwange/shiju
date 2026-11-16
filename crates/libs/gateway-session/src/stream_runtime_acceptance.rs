//! Domain assertions executed through the concrete native stream and fixture ports.
use crate::stream::*;
use vw_api_chat::chat::GatewayChatStreamRequest;
use crate::stream_runtime_acceptance_host::RuntimeAcceptanceHost;
use axum::{Json, Extension, extract::Query, http::{HeaderMap, StatusCode}, response::IntoResponse};
use gateway_instance::host::InstanceQuery;
use gateway_resumable_stream::STREAM_REQUEST_ID_HEADER;
use serde_json::Value;
use session_store::ui_store;

pub async fn codex_skills_defaults_gateway_main_and_child_preload_before_first_request<H: RuntimeAcceptanceHost>() {
    tokio::task::spawn_blocking(move || {

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    let _streams = H::stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let source = workspace.path().join("skills/review/SKILL.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(
        &source,
        "---\nname: review\ndescription: Review fixture\n---\nDEFAULT_REVIEW_BODY\n",
    )
    .unwrap();
    runtime.block_on(H::update_project(workspace.path(), serde_json::json!({
        "agents": {
            "main": {"provider":"model-recovery","model":"original","allowed_tools":["file_read"],"allowed_skills":["review"],"skill_policy":"strict"},
            "reviewer": {"provider":"model-recovery","model":"original","allowed_tools":["skill","file_read"],"allowed_skills":["review"],"skill_policy":"strict"}
        }
    }))).unwrap();
    for name in ["main", "reviewer"] {
        let session_id = format!("ses_defaults_{}", uuid::Uuid::new_v4());
        H::enqueue(vec![
            llm_runtime::StreamEvent::Delta("ready".into()),
            llm_runtime::StreamEvent::Done {
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
        let tenant = session_store::ui_store::current_tenant_scope();
        runtime.block_on(async {
            let mut receiver = H::with_tenant_instance(tenant.clone(), directory.clone(), move || {
                Box::pin(async move {
                    start_gateway_chat_stream::<H>(tenant, directory, request, None, None).await
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
        let captured = H::captured(&session_id);
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
        let tool_ids = H::tool_ids();
        assert_eq!(tool_ids.len(), 1);
        assert!(tool_ids[0].contains(&"skill".to_string()), "{name} must keep requested tools");
        assert!(tool_ids[0].contains(&"file_read".to_string()));
    }

    }).await.unwrap();
}
pub async fn native_workflow_gateway_persists_bound_graph_without_llm_artifacts<H: RuntimeAcceptanceHost>() {

    let _env = H::env_lock().lock().await;
    let _fixture = H::recovery(
        "https://example.test/v1",
        None,
    )
    .await;
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join(".shiju")).unwrap();
    let tenant = session_store::ui_store::current_tenant_scope();
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
        collect_gateway_chat_stream_with_observer::<H>(directory.clone(), request, |event| {
            seen.push(event["type"].as_str().unwrap_or_default().to_string());
        })
        .await
        .expect("workflow gateway stream should finish and persist");
    assert!(collected.output.ends_with("verified preview"));
    assert_eq!(seen.last().map(String::as_str), Some("chat.done"));
    assert!(collected.message_id.is_some());
    let scope = session_store::ui_store::resolve_session_scope_id_for_tenant(
        &tenant,
        Some(&directory),
        None,
    )
    .unwrap();
    let stored = session_store::ui_store::load_session_scoped_for_tenant(
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

pub async fn agcollab_delivery_reaches_next_model_request<H: RuntimeAcceptanceHost>() {
    tokio::task::spawn_blocking(move || {

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    let _streams = H::stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let source = workspace.path().join("skills/review/SKILL.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(
        &source,
        "---\nname: review\ndescription: Review fixture\n---\nDEFAULT_REVIEW_BODY\n",
    )
    .unwrap();
    runtime.block_on(H::update_project(workspace.path(), serde_json::json!({
        "agents": {
            "main": {"provider":"model-recovery","model":"original","allowed_tools":["file_read"],"allowed_skills":["review"],"skill_policy":"strict"},
            "reviewer": {"provider":"model-recovery","model":"original","allowed_tools":["skill","file_read"],"allowed_skills":["review"],"skill_policy":"strict"}
        }
    }))).unwrap();
    for name in ["main"] {
        let session_id = format!("ses_defaults_{}", uuid::Uuid::new_v4());
        let coordinator =
            session_engine::child_agent::coordinator::global_child_agent_coordinator();
        let child = format!("child_{}", uuid::Uuid::new_v4());
        coordinator.register(&session_id, &child, "reviewer", "review").unwrap();
        let boundary = H::register_boundary(&session_id);
        H::enqueue(vec![
            llm_runtime::StreamEvent::Delta("ready".into()),
            llm_runtime::StreamEvent::Done {
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
        let tenant = session_store::ui_store::current_tenant_scope();
        runtime.block_on(async {
            let mut receiver = H::with_tenant_instance(tenant.clone(), directory.clone(), move || {
                Box::pin(async move {
                    start_gateway_chat_stream::<H>(tenant, directory, request, None, None).await
                })
            })
            .await
            .unwrap();
            tokio::time::timeout(std::time::Duration::from_secs(20), async {
                while !H::boundary_started(&boundary) {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            coordinator.mark_succeeded(&session_id, &child, "TEAM_RESULT_BODY").unwrap();
            H::release_boundary(&boundary);
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
        let captured = H::captured(&session_id);
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
        let tool_ids = H::tool_ids();
        assert_eq!(tool_ids.len(), 1);
        assert!(tool_ids[0].contains(&"skill".to_string()), "{name} must keep requested tools");
        assert!(tool_ids[0].contains(&"file_read".to_string()));
    }

    }).await.unwrap();
}
pub async fn video_edit_entry_gateway_explicit_unavailable_model_does_not_start_graph<H: RuntimeAcceptanceHost>() {

    let _env = H::env_lock().lock().await;
    let _fixture = H::recovery(
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
    let result = collect_gateway_chat_stream_with_observer::<H>(
        root.path().to_string_lossy().into(),
        request,
        |e| events.push(e.clone()),
    )
    .await;
    assert!(result.is_err());
    assert!(!events.iter().any(|e| e["type"] == "chat.done"));
    assert!(!root.path().join(".shiju/workflow_commands.json").exists());
}

pub async fn agent_native_defaults_reach_registered_tools_and_prompt<H: RuntimeAcceptanceHost>(mcp_fixture: &'static str) {
    tokio::task::spawn_blocking(move || {

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(H::env_lock().lock());
    let _fixture =
        runtime.block_on(H::recovery(
            "https://example.test/v1",
            None,
        ));
    let _streams = H::stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let script = workspace.path().join("fixture.py");
    std::fs::write(
        &script,
        mcp_fixture,
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
    runtime.block_on(H::update_config(serde_json::json!({
        "agents": {
            "main": {"provider":"model-recovery","model":"unavailable-legacy","allowed_tools":["file_read"],"defaults":{"mcp":{"mode":"fixed","value":["selected"]},"model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}},"skills":{"mode":"fixed","value":["review"]}},"system_prompt":"NATIVE_DEFAULT_RULE","skill_policy":"strict", "allowed_skills":["review"]},
            "reviewer": {"provider":"model-recovery","model":"unavailable-legacy","allowed_tools":["skill","file_read"],"defaults":{"mcp":{"mode":"fixed","value":[]},"model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}},"skills":{"mode":"fixed","value":["review"]}},"system_prompt":"NATIVE_DEFAULT_RULE","skill_policy":"strict", "allowed_skills":["review"]}
        }
    }))).unwrap();
    for name in ["researcher", "coder", "explore"] {
        runtime.block_on(H::update_config(serde_json::json!({
            "agents": {name: {"defaults": {
                "mcp":{"mode":"fixed","value":[]},
                "model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}},
                "skills":{"mode":"fixed","value":["review"]}
            }, "system_prompt":"NATIVE_DEFAULT_RULE", "skill_policy":"strict", "allowed_skills":["review"]}}
        }))).unwrap();
    }
    for name in ["main", "reviewer", "researcher", "coder", "explore"] {
        let session_id = format!("ses_defaults_{}", uuid::Uuid::new_v4());
        H::enqueue(vec![
            llm_runtime::StreamEvent::Delta("ready".into()),
            llm_runtime::StreamEvent::Done {
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
        let tenant = session_store::ui_store::current_tenant_scope();
        runtime.block_on(async {
            let mut receiver = H::with_tenant_instance(tenant.clone(), directory.clone(), move || {
                Box::pin(async move {
                    start_gateway_chat_stream::<H>(tenant, directory, request, None, None).await
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
        let captured = H::captured(&session_id);
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
        let tool_ids = H::tool_ids();
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

    }).await.unwrap();
}
mod recovery;
pub use recovery::{gateway_first_response_failure_preserves_history_until_same_session_retries, gateway_first_response_disconnect_after_tool_start_replays_cancellation_without_publishing};

pub use recovery::{agent_native_defaults_do_not_grant_disabled_mcp, agent_config_cleanup_full_access_gateway_freezes_defaults_and_session_choice};
mod identity;
pub use identity::profile_edit_stream;

pub use recovery::gateway_chat_git_line_counts;

/// Check the observed real workflow stream against its persisted UI projection.
pub fn video_edit_progress_and_snapshot(
    collected: &GatewayChatStreamCollected,
    events: &[Value],
    project: &std::path::Path,
    directory: &str,
    tenant: &vw_api_transport::TenantScope,
    session: &str,
    expected_digest: &str,
) {
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
            expected_digest
        );
        assert!(last["peak"].as_u64().unwrap() <= 5);

        assert!(!project.join(".shiju/workflow_commands.json").exists());
        let scope = session_store::ui_store::resolve_session_scope_id_for_tenant(
            &tenant,
            Some(&directory),
            None,
        )
        .unwrap();
        let stored = session_store::ui_store::load_session_scoped_for_tenant(
            &tenant,
            &session,
            Some(&scope),
        )
        .unwrap();
        assert_eq!(stored.steps[0].finish_reason.as_deref(), Some("workflow"));
        assert!(stored.steps[0].tool_timings.len() >= 10, "real graph nodes must be visible");
        assert!(stored.messages.last().unwrap().content.contains("test-preview"));
}
