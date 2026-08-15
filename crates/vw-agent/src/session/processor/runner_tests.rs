#[test]
fn runner_tests_module_is_wired() {
    let marker = String::from("runner_tests");
    assert_eq!(marker.as_str(), "runner_tests");
}

#[test]
fn full_access_option_enables_request_full_access() {
    let options = serde_json::json!({ "full_access": true });

    assert!(super::request_full_access_enabled(&options));
}

#[test]
fn missing_full_access_option_defaults_to_disabled() {
    let options = serde_json::json!({});

    assert!(!super::request_full_access_enabled(&options));
}

#[test]
fn request_model_context_keeps_provider_and_model_together() {
    let context = super::with_request_model_context(
        crate::app::agent::tools::ToolUseContext::new("parent", None),
        Some(" token-plan/qwen3.7-plus "),
    );

    assert_eq!(context.provider(), Some("token-plan"));
    assert_eq!(context.model(), Some("qwen3.7-plus"));
}

#[test]
fn request_model_context_retains_auto_selection_source() {
    let context = super::with_request_model_context(
        crate::app::agent::tools::ToolUseContext::new("parent", None),
        Some("token-plan/qwen3.7-plus"),
    )
    .with_auto_model_selection(true);

    assert!(context.model_selected_by_auto());
}

#[test]
fn processor_auto_resolves_model_before_tool_context() {
    let req = request_with_session("auto-before-tools");

    let resolved = super::resolve_request_model(&req, &crate::config::Config::default())
        .expect("Auto request should resolve before building tool context");

    assert!(resolved.model.contains('/'));
}

#[test]
fn processor_step_events_report_selected_model() {
    let req = request_with_session("selected-step-events");

    let resolved = super::resolve_request_model(&req, &crate::config::Config::default())
        .expect("Auto request should resolve once for all processor events");

    assert_eq!(resolved.step_event_model(), Some(resolved.model.as_str()));
}

#[test]
fn enabled_identity_files_follow_selected_agent_config_snapshot() {
    let mut config = crate::config::Config::default();
    config.agents.insert(
        "main".to_string(),
        crate::config::DelegateAgentConfig {
            enabled_identity_files: vec!["AGENTS.md".to_string()],
            ..Default::default()
        },
    );
    config.agents.insert(
        "reviewer".to_string(),
        crate::config::DelegateAgentConfig {
            enabled_identity_files: vec!["SOUL.md".to_string()],
            ..Default::default()
        },
    );

    assert_eq!(super::enabled_identity_files_for_agent(&config, None), vec!["AGENTS.md"]);
    assert_eq!(super::enabled_identity_files_for_agent(&config, Some("reviewer")), vec!["SOUL.md"]);
    assert!(super::enabled_identity_files_for_agent(&config, Some("missing")).is_empty());
}

#[test]
fn session_protocol_initial_step_contains_explicit_workspace_agents() {
    let project = tempfile::tempdir().expect("project workspace");
    std::fs::write(project.path().join("AGENTS.md"), "INITIAL_AGENTS_PROTOCOL")
        .expect("write project agents");
    let base_system = crate::session::prompt::system_with_metadata(None, project.path().to_str());
    let history = vec![serde_json::json!({ "role": "user", "content": "start task" })];
    let mut protocol = super::DesktopSessionProtocol::default();

    let messages = protocol
        .assemble(&history, base_system.instruction_context(), false, &[])
        .expect("assemble initial protocol messages");

    assert_eq!(messages.last(), history.last());
    assert!(messages.first().is_some_and(|message| {
        message.get("role").and_then(serde_json::Value::as_str) == Some("system")
            && message.get("content").and_then(serde_json::Value::as_str).is_some_and(|content| {
                content.contains("AGENTS.md") && content.contains("INITIAL_AGENTS_PROTOCOL")
            })
    }));
}

#[test]
fn agents_referenced_skill_is_preloaded_before_first_user_request() {
    let workspace = tempfile::tempdir().expect("skill workspace");
    let skill_path = workspace.path().join("rustcodegraph/SKILL.md");
    std::fs::create_dir_all(skill_path.parent().expect("skill parent")).expect("skill dir");
    std::fs::write(&skill_path, "# RustCodeGraph\nUSE_RUSTCODEGRAPH_FIRST").expect("write skill");
    let skills = vec![crate::app::agent::skills::Skill {
        name: "rustcodegraph".to_string(),
        location: Some(skill_path),
        ..Default::default()
    }];
    let instructions = vec![crate::session::instruction::Resolved {
        filepath: "/workspace/AGENTS.md".to_string(),
        content: "Instructions from: /workspace/AGENTS.md\nUse rustcodegraph for code navigation."
            .to_string(),
        sources: Vec::new(),
        diagnostics: Vec::new(),
    }];
    let referenced = crate::app::agent::tools::skill::explicitly_referenced_skill_names(
        &skills,
        instructions.iter().map(|instruction| instruction.content.as_str()),
    );
    let mut protocol = super::DesktopSessionProtocol::new(skills, "main", 20_000);

    protocol.preload_skills(&referenced).expect("preload AGENTS skill");
    let history = vec![serde_json::json!({"role": "user", "content": "inspect callers"})];
    let messages =
        protocol.assemble(&history, &instructions, true, &[]).expect("assemble protocol");

    assert!(
        messages[0]["content"]
            .as_str()
            .is_some_and(|content| { content.contains("USE_RUSTCODEGRAPH_FIRST") })
    );
    assert_eq!(messages[messages.len() - 2]["role"], "system");
    assert_eq!(messages.last(), history.last());
}

#[test]
fn delegate_skill_policy_preloads_defaults_and_honors_trusted_user_override() {
    let workspace = tempfile::tempdir().expect("skill workspace");
    let default_path = workspace.path().join("default/SKILL.md");
    let user_path = workspace.path().join("user/SKILL.md");
    std::fs::create_dir_all(default_path.parent().expect("default parent")).expect("default dir");
    std::fs::create_dir_all(user_path.parent().expect("user parent")).expect("user dir");
    std::fs::write(&default_path, "# Default\nDEFAULT_SKILL_BODY").expect("default skill");
    std::fs::write(&user_path, "# User\nUSER_SKILL_BODY").expect("user skill");
    let skills = vec![
        crate::app::agent::skills::Skill {
            name: "default".to_string(),
            location: Some(default_path),
            ..Default::default()
        },
        crate::app::agent::skills::Skill {
            name: "user".to_string(),
            location: Some(user_path),
            ..Default::default()
        },
    ];
    let options = serde_json::json!({
        "delegate_skill_policy": "user_override",
        "delegate_default_allowed_skills": ["default"],
        "delegate_trusted_user_selected_skills": ["user"],
    });
    let resolution = super::delegated_skill_policy_resolution(&skills, &options)
        .expect("delegated policy should resolve");
    assert_eq!(resolution.visible_ids, ["default", "user"].into_iter().map(String::from).collect());

    let mut protocol = super::DesktopSessionProtocol::new(skills, "delegate", 20_000);
    protocol.preload_skills(&resolution.default_injected_ids).expect("preload default");
    let history = vec![serde_json::json!({"role": "user", "content": "work"})];
    let first = protocol.assemble(&history, &[], true, &[]).expect("first protocol");
    let second = protocol.assemble(&history, &[], true, &[]).expect("second protocol");
    let default_count = |messages: &[serde_json::Value]| {
        messages
            .iter()
            .filter(|message| {
                message["content"]
                    .as_str()
                    .is_some_and(|content| content.contains("DEFAULT_SKILL_BODY"))
            })
            .count()
    };
    assert_eq!(default_count(&first), 1);
    assert_eq!(default_count(&second), 1);
    assert!(!first.iter().any(|message| {
        message["content"].as_str().is_some_and(|content| content.contains("USER_SKILL_BODY"))
    }));
}

use crate::app::agent::session::llm;
use crate::app::agent::tools::ToolRuntimeContext;
use crate::session::ui_types as models;
use std::collections::HashSet;

fn unique_session_id(name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    format!("runner-tests-{name}-{nanos}")
}

fn request_with_session(session: &str) -> super::Request {
    super::Request {
        stream: 1,
        session: session.to_string(),
        query: String::new(),
        root: None,
        model: None,
        options: serde_json::json!({}),
        approval: None,
        channel_name: None,
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: Vec::new(),
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools: crate::session::runtime::SessionToolset::default(),
    }
}

fn register_scope_project(scope: &str) -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("project tempdir");
    let info = crate::project::Info {
        id: scope.to_string(),
        worktree: temp.path().to_string_lossy().to_string(),
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: crate::project::TimeInfo { created: 10, updated: 20, initialized: None },
        sandboxes: Vec::new(),
    };
    crate::project::store_local_test_project(&info).expect("project should write");
    temp
}

fn llm_step(text: &str) -> super::llm_runner::LlmStep {
    super::llm_runner::LlmStep {
        usage: models::TokenUsage {
            input_tokens: 10,
            output_tokens: 5,
            cached_tokens: 2,
            reasoning_tokens: 1,
        },
        finish_reason: Some("stop".to_string()),
        reasoning_content: String::new(),
        text: text.to_string(),
        tool_calls: Vec::new(),
        full_messages: Vec::new(),
    }
}

fn event_texts(events: &[super::StreamEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            super::StreamEvent::Delta(text) => Some(format!("delta:{text}")),
            super::StreamEvent::Tool(event) => {
                Some(format!("tool:{}:{}", event.name, event.payload))
            }
            super::StreamEvent::Error(text) => Some(format!("error:{text}")),
            super::StreamEvent::Done(_) => Some("done".to_string()),
            super::StreamEvent::PostToolRound { step_index } => {
                Some(format!("post-tool-round:{step_index}"))
            }
            super::StreamEvent::StepStart { step_index, .. } => {
                Some(format!("step-start:{step_index}"))
            }
            super::StreamEvent::StepFinish { step_index, .. } => {
                Some(format!("step-finish:{step_index}"))
            }
            super::StreamEvent::ContextLoaded { paths } => {
                Some(format!("context-loaded:{}", paths.join(",")))
            }
        })
        .collect()
}

#[test]
fn false_full_access_option_does_not_enable_request_full_access() {
    let options = serde_json::json!({ "full_access": false });

    assert!(!super::request_full_access_enabled(&options));
}

#[test]
fn non_boolean_full_access_option_defaults_to_disabled() {
    let options = serde_json::json!({ "full_access": "true" });

    assert!(!super::request_full_access_enabled(&options));
}

#[test]
fn non_object_full_access_options_default_to_disabled() {
    assert!(!super::request_full_access_enabled(&serde_json::json!(true)));
    assert!(!super::request_full_access_enabled(&serde_json::Value::Null));
}

#[test]
fn handle_step_error_emits_error_for_non_empty_response_failure() {
    let mut session = super::Session::new(unique_session_id("step-error"));
    let mut llm_messages = Vec::new();
    let mut retries = 0usize;
    let mut step = 0;
    let mut events = Vec::new();

    let should_continue = super::handle_step_error(
        &mut session,
        &mut llm_messages,
        &mut retries,
        &mut step,
        &mut |event| {
            events.push(event);
            true
        },
        "provider failed".to_string(),
    );

    assert!(!should_continue);
    assert_eq!(step, 0);
    assert_eq!(retries, 0);
    assert!(llm_messages.is_empty());
    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, super::Role::System);
    assert_eq!(event_texts(&events), vec!["error:provider failed"]);
}

#[test]
fn handle_step_error_retries_empty_llm_responses_until_limit() {
    let mut session = super::Session::new(unique_session_id("empty-step-error"));
    let mut llm_messages = Vec::new();
    let mut retries = 0usize;
    let mut step = 0;
    let mut events = Vec::new();

    let should_continue = super::handle_step_error(
        &mut session,
        &mut llm_messages,
        &mut retries,
        &mut step,
        &mut |event| {
            events.push(event);
            true
        },
        super::EMPTY_RESPONSE_ERR.to_string(),
    );

    assert!(should_continue);
    assert_eq!(retries, 1);
    assert_eq!(step, 1);
    assert_eq!(llm_messages.len(), 1);
    assert!(events.is_empty());
    assert!(session.messages[0].content.contains("自动重试中"));

    retries = super::EMPTY_RESPONSE_RETRY_LIMIT;
    let should_continue = super::handle_step_error(
        &mut session,
        &mut llm_messages,
        &mut retries,
        &mut step,
        &mut |event| {
            events.push(event);
            true
        },
        super::EMPTY_RESPONSE_ERR.to_string(),
    );

    assert!(!should_continue);
    assert_eq!(retries, super::EMPTY_RESPONSE_RETRY_LIMIT + 1);
    assert!(event_texts(&events).last().is_some_and(|text| text.contains("任务终止")));
}

#[test]
fn handle_step_error_recognizes_trimmed_empty_response_marker() {
    let mut session = super::Session::new(unique_session_id("trimmed-empty-step-error"));
    let mut llm_messages = Vec::new();
    let mut retries = 0usize;
    let mut step = 0;
    let mut events = Vec::new();

    let should_continue = super::handle_step_error(
        &mut session,
        &mut llm_messages,
        &mut retries,
        &mut step,
        &mut |event| {
            events.push(event);
            true
        },
        format!(" \n{}\t", super::EMPTY_RESPONSE_ERR),
    );

    assert!(should_continue);
    assert_eq!(retries, 1);
    assert_eq!(step, 1);
    assert_eq!(llm_messages[0]["content"].as_str(), Some(super::EMPTY_RESPONSE_RETRY_PROMPT));
    assert!(events.is_empty());
}

#[test]
fn handle_step_error_saturates_retry_counter_before_terminal_error() {
    let mut session = super::Session::new(unique_session_id("saturated-step-error"));
    let mut llm_messages = Vec::new();
    let mut retries = usize::MAX;
    let mut step = 3;
    let mut events = Vec::new();

    let should_continue = super::handle_step_error(
        &mut session,
        &mut llm_messages,
        &mut retries,
        &mut step,
        &mut |event| {
            events.push(event);
            true
        },
        super::EMPTY_RESPONSE_ERR.to_string(),
    );

    assert!(!should_continue);
    assert_eq!(retries, usize::MAX);
    assert_eq!(step, 3);
    assert!(llm_messages.is_empty());
    assert_eq!(session.messages.len(), 1);
    assert!(event_texts(&events)[0].contains("任务终止"));
}

#[test]
fn handle_docs_request_lists_docs_files_and_finishes() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    std::fs::create_dir(workspace.path().join("docs")).expect("docs dir should be created");
    std::fs::write(workspace.path().join("docs/guide.md"), "# Guide\n")
        .expect("doc should be written");
    let ctx = ToolRuntimeContext::new(
        unique_session_id("docs"),
        Some(workspace.path().to_string_lossy().to_string()),
    );
    let mut session = super::Session::new(ctx.session.clone());
    let mut events = Vec::new();

    super::handle_docs_request(&mut session, &ctx, &mut |event| {
        events.push(event);
        true
    });

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, super::Role::Assistant);
    assert!(session.messages[0].content.contains("docs/guide.md"));
    assert_eq!(
        event_texts(&events),
        vec![format!("delta:{}", session.messages[0].content), "done".to_string(),]
    );
}

#[test]
fn handle_docs_request_emits_error_when_docs_directory_is_missing() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    let ctx = ToolRuntimeContext::new(
        unique_session_id("docs-missing"),
        Some(workspace.path().to_string_lossy().to_string()),
    );
    let mut session = super::Session::new(ctx.session.clone());
    let mut events = Vec::new();

    super::handle_docs_request(&mut session, &ctx, &mut |event| {
        events.push(event);
        true
    });

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, super::Role::System);
    assert_eq!(event_texts(&events), vec!["error:docs 目录不存在"]);
}

#[test]
fn handle_docs_request_stops_before_done_when_delta_is_rejected() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    std::fs::create_dir(workspace.path().join("docs")).expect("docs dir should be created");
    std::fs::write(workspace.path().join("docs/guide.md"), "# Guide\n")
        .expect("doc should be written");
    let ctx = ToolRuntimeContext::new(
        unique_session_id("docs-rejected-delta"),
        Some(workspace.path().to_string_lossy().to_string()),
    );
    let mut session = super::Session::new(ctx.session.clone());
    let mut events = Vec::new();

    super::handle_docs_request(&mut session, &ctx, &mut |event| {
        events.push(event);
        false
    });

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, super::Role::Assistant);
    assert_eq!(event_texts(&events), vec![format!("delta:{}", session.messages[0].content)]);
}

#[test]
fn run_routes_docs_request_without_starting_an_llm_step() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    std::fs::create_dir(workspace.path().join("docs")).expect("docs dir should be created");
    std::fs::write(workspace.path().join("docs/guide.md"), "# Guide\n")
        .expect("doc should be written");
    let mut req = request_with_session(&unique_session_id("run-docs"));
    req.query = "请列出 docs 文件列表".to_string();
    req.root = Some(workspace.path().to_string_lossy().to_string());
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded_events = std::sync::Arc::clone(&events);

    super::run(req, move |event| {
        recorded_events.lock().expect("events lock should not be poisoned").push(event);
        true
    });

    let events = events.lock().expect("events lock should not be poisoned");
    assert_eq!(event_texts(events.as_slice()).len(), 2);
    assert!(event_texts(events.as_slice())[0].contains("docs/guide.md"));
    assert_eq!(event_texts(events.as_slice())[1], "done");
    assert!(!events.iter().any(|event| matches!(event, super::StreamEvent::StepStart { .. })));
}

#[test]
fn raw_payload_is_visible_before_step_finish() {
    let _guard = super::llm_runner::test_llm_stream_guard();
    super::llm_runner::enqueue_test_llm_stream(vec![
        crate::app::agent::session::llm::StreamEvent::Delta("persisted answer".to_string()),
        crate::app::agent::session::llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("stop".to_string()),
        },
    ]);
    let session_id = unique_session_id("raw-before-step-finish");
    let scope = unique_session_id("raw-before-step-finish-scope");
    let _project = register_scope_project(&scope);
    let mut req = request_with_session(&session_id);
    req.query = "answer once".to_string();
    req.persist_app_session_artifacts = true;
    req.app_session_scope = Some(scope.clone());
    let raw_visible = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed_raw = std::sync::Arc::clone(&raw_visible);
    let observed_session_id = session_id.clone();

    super::run(req, move |event| {
        if let super::StreamEvent::StepFinish { step_index, .. } = event {
            let raw = crate::session::ui_store::load_llm_raw_step_scoped(
                &observed_session_id,
                step_index,
                Some(&scope),
            )
            .expect("raw payload read should succeed");
            observed_raw.store(raw.is_some(), std::sync::atomic::Ordering::SeqCst);
        }
        true
    });

    assert!(
        raw_visible.load(std::sync::atomic::Ordering::SeqCst),
        "raw payload must be readable before StepFinish is observed"
    );
}

#[test]
fn raw_payload_survives_rejected_step_finish() {
    let _guard = super::llm_runner::test_llm_stream_guard();
    super::llm_runner::enqueue_test_llm_stream(vec![
        crate::app::agent::session::llm::StreamEvent::Delta("persisted answer".to_string()),
        crate::app::agent::session::llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("stop".to_string()),
        },
    ]);
    let session_id = unique_session_id("raw-rejected-step-finish");
    let scope = unique_session_id("raw-rejected-step-finish-scope");
    let _project = register_scope_project(&scope);
    let mut req = request_with_session(&session_id);
    req.query = "answer once".to_string();
    req.persist_app_session_artifacts = true;
    req.app_session_scope = Some(scope.clone());
    let rejected_step = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let observed_step = std::sync::Arc::clone(&rejected_step);

    super::run(req, move |event| {
        if let super::StreamEvent::StepFinish { step_index, .. } = event {
            observed_step.store(step_index, std::sync::atomic::Ordering::SeqCst);
            return false;
        }
        true
    });

    let step_index = rejected_step.load(std::sync::atomic::Ordering::SeqCst);
    assert_ne!(step_index, 0, "the fixture must observe a StepFinish event");
    let raw =
        crate::session::ui_store::load_llm_raw_step_scoped(&session_id, step_index, Some(&scope))
            .expect("raw payload read should succeed");
    assert!(raw.is_some(), "rejecting StepFinish must not discard the raw payload");
}

#[test]
fn context_preflight_runs_before_next_llm_step_after_real_input_usage() {
    let mut messages = vec![serde_json::json!({"role": "system", "content": "system"})];
    for index in 0..9 {
        let call_id = format!("call-{index}");
        messages.push(serde_json::json!({
            "role": "assistant",
            "tool_calls": [{"id": call_id, "type": "function"}]
        }));
        messages.push(serde_json::json!({
            "role": "tool",
            "tool_call_id": call_id,
            "content": if index == 0 { "old output ".repeat(80) } else { "recent".to_string() }
        }));
    }
    messages.push(serde_json::json!({"role": "user", "content": "latest question"}));

    let result = super::prepare_context_before_llm_step(
        &mut messages,
        &["system".to_string()],
        &[],
        &crate::app::agent::provider::provider::ModelLimit {
            context: 500,
            input: Some(500),
            output: 0,
        },
        Some(490),
    )
    .expect("the next LLM step should report whether internal compaction is required");

    assert!(result.needs_internal_compaction);
    assert!(messages.iter().any(|message| message["role"] == "system"));
    assert!(messages.iter().any(|message| message["role"] == "user"));
    assert_ne!(
        messages
            .iter()
            .find(|message| message["tool_call_id"] == "call-0")
            .and_then(|message| message["content"].as_str()),
        Some(
            "old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output "
        )
    );
}

#[test]
fn transient_internal_compaction_replaces_only_covered_history_without_tools() {
    let _guard = super::llm_runner::test_llm_stream_guard();
    super::llm_runner::enqueue_test_llm_stream(vec![
        crate::app::agent::session::llm::StreamEvent::Delta("remembered context".to_string()),
        crate::app::agent::session::llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("stop".to_string()),
        },
    ]);
    let mut messages = vec![
        serde_json::json!({ "role": "system", "content": "system" }),
        serde_json::json!({ "role": "user", "content": "old request" }),
        serde_json::json!({ "role": "assistant", "content": "old response" }),
        serde_json::json!({
            "role": "user",
            "content": "latest request",
            "_vw_source": {
                "kind": "user",
                "rpcId": "desktop-compaction-request",
                "clientTimeZone": "Asia/Shanghai",
            }
        }),
    ];

    let compacted = super::compact_transient_history_for_request(
        "runner-transient-compaction",
        &mut messages,
        "provider/model",
    )
    .expect("summary should succeed");

    assert!(compacted);
    assert_eq!(messages.len(), 2);
    assert!(
        messages[0]["content"].as_str().is_some_and(|text| text.contains("remembered context"))
    );
    assert_eq!(messages[1]["content"], "latest request");
    assert_eq!(messages[1]["_vw_source"]["rpcId"], "desktop-compaction-request");
    assert_eq!(super::llm_runner::take_test_llm_stream_tool_ids(), vec![Vec::<String>::new()]);
}

#[test]
fn transient_internal_compaction_keeps_history_when_summary_fails() {
    let _guard = super::llm_runner::test_llm_stream_guard();
    super::llm_runner::enqueue_test_llm_stream(vec![
        crate::app::agent::session::llm::StreamEvent::Error(
            crate::app::agent::session::message::AssistantError::Unknown {
                message: "summary unavailable".to_string(),
            },
        ),
    ]);
    let mut messages = vec![
        serde_json::json!({ "role": "system", "content": "system" }),
        serde_json::json!({ "role": "user", "content": "old request" }),
        serde_json::json!({ "role": "user", "content": "latest request" }),
    ];
    let before = messages.clone();

    let error = super::compact_transient_history_for_request(
        "runner-transient-compaction-failure",
        &mut messages,
        "provider/model",
    )
    .expect_err("failed summaries must not change history");

    assert!(error.contains("summary unavailable"));
    assert_eq!(messages, before);
}

#[test]
fn handle_structured_tool_calls_records_denied_result_for_unallowed_tool() {
    let session_id = unique_session_id("structured-denied");
    let req = request_with_session(&session_id);
    let ctx = ToolRuntimeContext::new(session_id.clone(), None);
    let mut session = super::Session::new(session_id);
    let mut llm_messages = Vec::new();
    let mut total_usage = models::TokenUsage::default();
    let mut tool_state = super::ToolSessionState::default();
    let mut session_protocol = super::DesktopSessionProtocol::default();
    let mut events = Vec::new();
    let step_out = super::llm_runner::LlmStep {
        tool_calls: vec![llm::ToolCall {
            id: "call-denied".to_string(),
            name: "not_allowed".to_string(),
            arguments: "{}".to_string(),
        }],
        ..llm_step("")
    };

    let ran_todo_update = super::handle_structured_tool_calls(
        &req,
        &mut session,
        &ctx,
        &HashSet::new(),
        "base system",
        &mut llm_messages,
        &mut total_usage,
        &mut |event| {
            events.push(event);
            true
        },
        &mut tool_state,
        &mut session_protocol,
        &step_out,
        "",
        false,
        1,
    );

    assert!(!ran_todo_update);
    assert!(session.messages.is_empty());
    assert!(events.is_empty());
    assert_eq!(llm_messages.len(), 2);
    assert_eq!(llm_messages[0]["role"].as_str(), Some("assistant"));
    assert_eq!(llm_messages[1]["role"].as_str(), Some("tool"));
    assert_eq!(llm_messages[1]["content"].as_str(), Some("tool denied: not_allowed"));
}

#[test]
fn handle_structured_tool_calls_denies_allowed_tool_on_last_step() {
    let session_id = unique_session_id("structured-last-step-denied");
    let req = request_with_session(&session_id);
    let ctx = ToolRuntimeContext::new(session_id.clone(), None);
    let mut session = super::Session::new(session_id);
    let mut llm_messages = Vec::new();
    let mut total_usage = models::TokenUsage::default();
    let mut tool_state = super::ToolSessionState::default();
    let mut session_protocol = super::DesktopSessionProtocol::default();
    let mut allowed_tools = HashSet::new();
    allowed_tools.insert("otherwise_allowed".to_string());
    let step_out = super::llm_runner::LlmStep {
        tool_calls: vec![llm::ToolCall {
            id: "call-last-step".to_string(),
            name: "otherwise_allowed".to_string(),
            arguments: "{}".to_string(),
        }],
        ..llm_step("")
    };

    let ran_todo_update = super::handle_structured_tool_calls(
        &req,
        &mut session,
        &ctx,
        &allowed_tools,
        "base system",
        &mut llm_messages,
        &mut total_usage,
        &mut |_| true,
        &mut tool_state,
        &mut session_protocol,
        &step_out,
        "",
        true,
        101,
    );

    assert!(!ran_todo_update);
    assert!(session.messages.is_empty());
    assert_eq!(tool_state.non_todo_tool_runs, 0);
    assert_eq!(llm_messages.len(), 2);
    assert_eq!(
        llm_messages[1]["content"].as_str(),
        Some(
            "tool unavailable: maximum tool iteration budget reached; \
             `otherwise_allowed` was not rejected by the permission policy"
        )
    );
}

#[test]
fn handle_assistant_text_branch_accepts_plain_final_text() {
    let session_id = unique_session_id("plain-text");
    let req = request_with_session(&session_id);
    let ctx = ToolRuntimeContext::new(session_id.clone(), None);
    let mut session = super::Session::new(session_id);
    let mut llm_messages = Vec::new();
    let mut total_usage = models::TokenUsage::default();
    let mut tool_state = super::ToolSessionState::default();
    let mut retries = 2usize;
    let mut step = 0;
    let mut events = Vec::new();
    let empty_tools = HashSet::new();
    let step_out = llm_step("  final answer  ");

    let should_continue = super::handle_assistant_text_branch(
        &req,
        &mut session,
        &ctx,
        &empty_tools,
        &mut llm_messages,
        &mut total_usage,
        &mut |event| {
            events.push(event);
            true
        },
        &mut tool_state,
        &mut retries,
        &mut step,
        &step_out,
        "final answer",
        &empty_tools,
        false,
        1,
        None,
    );

    assert!(!should_continue);
    assert_eq!(retries, 0);
    assert_eq!(step, 0);
    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].content, "final answer");
    assert_eq!(llm_messages.len(), 1);
    assert_eq!(llm_messages[0]["content"].as_str(), Some("final answer"));
    assert_eq!(event_texts(&events), vec!["done"]);
}

#[test]
fn legacy_require_agent_tool_option_does_not_reject_plain_final_text() {
    let session_id = unique_session_id("required-agent-tool-plain-text");
    let mut req = request_with_session(&session_id);
    req.options = serde_json::json!({ "require_agent_tool": true });
    let ctx = ToolRuntimeContext::new(session_id.clone(), None);
    let mut session = super::Session::new(session_id);
    let mut llm_messages = Vec::new();
    let mut total_usage = models::TokenUsage::default();
    let mut tool_state = super::ToolSessionState::default();
    let mut retries = 0usize;
    let mut step = 0;
    let mut events = Vec::new();
    let empty_tools = HashSet::new();

    let step_out = llm_step("plain final answer");

    let should_continue = super::handle_assistant_text_branch(
        &req,
        &mut session,
        &ctx,
        &empty_tools,
        &mut llm_messages,
        &mut total_usage,
        &mut |event| {
            events.push(event);
            true
        },
        &mut tool_state,
        &mut retries,
        &mut step,
        &step_out,
        "plain final answer",
        &empty_tools,
        false,
        1,
        None,
    );

    assert!(!should_continue);
    assert_eq!(event_texts(&events), vec!["done"]);
}

#[test]
fn handle_assistant_text_branch_retries_empty_text_with_reasoning_message() {
    let session_id = unique_session_id("empty-text");
    let req = request_with_session(&session_id);
    let ctx = ToolRuntimeContext::new(session_id.clone(), None);
    let mut session = super::Session::new(session_id);
    let mut llm_messages = Vec::new();
    let mut total_usage = models::TokenUsage::default();
    let mut tool_state = super::ToolSessionState::default();
    let mut retries = 0usize;
    let mut step = 0;
    let mut events = Vec::new();
    let empty_tools = HashSet::new();
    let mut step_out = llm_step("   ");
    step_out.reasoning_content = "reasoning only".to_string();

    let should_continue = super::handle_assistant_text_branch(
        &req,
        &mut session,
        &ctx,
        &empty_tools,
        &mut llm_messages,
        &mut total_usage,
        &mut |event| {
            events.push(event);
            true
        },
        &mut tool_state,
        &mut retries,
        &mut step,
        &step_out,
        "",
        &empty_tools,
        false,
        1,
        None,
    );

    assert!(should_continue);
    assert_eq!(retries, 1);
    assert_eq!(step, 1);
    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, super::Role::System);
    assert_eq!(llm_messages.len(), 2);
    assert_eq!(llm_messages[0]["reasoning_content"].as_str(), Some("reasoning only"));
    assert_eq!(llm_messages[1]["role"].as_str(), Some("system"));
    assert!(events.is_empty());
}

#[test]
fn handle_assistant_text_branch_continues_when_incomplete_todos_remain() {
    use crate::app::agent::tools::{TODO_WRITE_TOOL_ID, todo};

    let session_id = unique_session_id("incomplete-todos");
    let req = request_with_session(&session_id);
    let ctx = ToolRuntimeContext::new(session_id.clone(), None);
    let mut session = super::Session::new(session_id);
    let mut llm_messages = Vec::new();
    let mut total_usage = models::TokenUsage::default();
    let mut tool_state = super::ToolSessionState::default();
    let mut retries = 0usize;
    let mut step = 0;
    let mut events = Vec::new();
    let empty_tools = HashSet::new();
    let mut allowed_tools = HashSet::new();
    allowed_tools.insert(TODO_WRITE_TOOL_ID.to_string());
    let step_out = llm_step("看起来已经完成了");

    let seed = serde_json::json!({
        "todos": [{
            "id": "1",
            "content": "still pending work",
            "status": "pending",
            "priority": "medium"
        }]
    })
    .to_string();
    todo::write(&seed, &ctx).expect("seed incomplete todos");

    let should_continue = super::handle_assistant_text_branch(
        &req,
        &mut session,
        &ctx,
        &allowed_tools,
        &mut llm_messages,
        &mut total_usage,
        &mut |event| {
            events.push(event);
            true
        },
        &mut tool_state,
        &mut retries,
        &mut step,
        &step_out,
        "看起来已经完成了",
        &empty_tools,
        false,
        1,
        None,
    );

    assert!(should_continue);
    assert_eq!(step, 1);
    assert!(session.messages.iter().any(|message| {
        message.role == super::Role::User && message.content.contains("任务未完成")
    }));
    assert!(!event_texts(&events).iter().any(|text| text == "done"));
}

#[test]
fn unfinished_todos_do_not_auto_complete_from_assistant_text() {
    use crate::app::agent::tools::{TODO_WRITE_TOOL_ID, todo};

    let session_id = unique_session_id("assistant-text-does-not-complete-todos");
    let req = request_with_session(&session_id);
    let ctx = ToolRuntimeContext::new(session_id.clone(), None);
    let mut session = super::Session::new(session_id);
    let mut llm_messages = Vec::new();
    let mut total_usage = models::TokenUsage::default();
    let mut tool_state = super::ToolSessionState { non_todo_tool_runs: 1, ..Default::default() };
    let mut retries = 0usize;
    let mut step = 0;
    let mut events = Vec::new();
    let empty_tools = HashSet::new();
    let mut allowed_tools = HashSet::new();
    allowed_tools.insert(TODO_WRITE_TOOL_ID.to_string());
    let answer = "I completed the requested work after inspecting the repository, applying the changes, and verifying the result.";
    let step_out = llm_step(answer);

    let seed = serde_json::json!({
        "todos": [{
            "id": "1",
            "content": "still pending work",
            "status": "pending",
            "priority": "medium"
        }]
    })
    .to_string();
    todo::write(&seed, &ctx).expect("seed incomplete todos");

    let should_continue = super::handle_assistant_text_branch(
        &req,
        &mut session,
        &ctx,
        &allowed_tools,
        &mut llm_messages,
        &mut total_usage,
        &mut |event| {
            events.push(event);
            true
        },
        &mut tool_state,
        &mut retries,
        &mut step,
        &step_out,
        answer,
        &empty_tools,
        false,
        1,
        None,
    );

    let todos: Vec<todo::Todo> =
        serde_json::from_str(&todo::read(&ctx).expect("read todos")).expect("parse todos");
    assert!(should_continue);
    assert_eq!(todos[0].status, "pending");
    assert_eq!(step, 1);
    assert!(!event_texts(&events).iter().any(|text| text == "done"));
}

#[test]
fn handle_empty_assistant_text_emits_terminal_error_after_retry_limit() {
    let session_id = unique_session_id("empty-limit");
    let req = request_with_session(&session_id);
    let mut session = super::Session::new(session_id);
    let mut llm_messages = Vec::new();
    let mut retries = super::EMPTY_RESPONSE_RETRY_LIMIT;
    let mut step = 7;
    let mut events = Vec::new();

    let should_continue = super::handle_empty_assistant_text(
        &req,
        &mut session,
        &mut llm_messages,
        &mut |event| {
            events.push(event);
            true
        },
        &mut retries,
        &mut step,
        "unused reasoning",
        true,
        101,
    );

    assert!(!should_continue);
    assert_eq!(step, 7);
    assert!(llm_messages.is_empty());
    assert_eq!(session.messages[0].role, super::Role::System);
    assert!(event_texts(&events)[0].contains("任务终止"));
}

#[test]
fn handle_empty_assistant_text_retries_at_limit_boundary() {
    let session_id = unique_session_id("empty-limit-boundary");
    let req = request_with_session(&session_id);
    let mut session = super::Session::new(session_id);
    let mut llm_messages = Vec::new();
    let mut retries = super::EMPTY_RESPONSE_RETRY_LIMIT - 1;
    let mut step = 9;
    let mut events = Vec::new();

    let should_continue = super::handle_empty_assistant_text(
        &req,
        &mut session,
        &mut llm_messages,
        &mut |event| {
            events.push(event);
            true
        },
        &mut retries,
        &mut step,
        "boundary reasoning",
        false,
        100,
    );

    assert!(should_continue);
    assert_eq!(retries, super::EMPTY_RESPONSE_RETRY_LIMIT);
    assert_eq!(step, 10);
    assert_eq!(llm_messages.len(), 2);
    assert_eq!(llm_messages[0]["reasoning_content"].as_str(), Some("boundary reasoning"));
    assert_eq!(llm_messages[1]["content"].as_str(), Some(super::EMPTY_RESPONSE_RETRY_PROMPT));
    assert!(events.is_empty());
}
