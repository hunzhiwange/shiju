use crate::tools::ToolUseContext;
use serde_json::{Value, json};
use std::sync::Arc;
fn fixture(root: &std::path::Path, state: &std::path::Path, parent: &str) -> Arc<ToolUseContext> {
    fixture_output(root, state, parent, "", "")
}
fn fixture_output(
    root: &std::path::Path,
    state: &std::path::Path,
    parent: &str,
    start: &str,
    stop: &str,
) -> Arc<ToolUseContext> {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let command = format!(
        "payload=$(cat); printf '%s\\n' \"$payload\" >> '{}'",
        state.join("events").display()
    );
    let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{
        "SubagentStart":[{"matcher":"reviewer", "hooks":[{"type":"command","command":format!("{command}; printf '%s' '{start}'")}]}],
        "SubagentStop":[{"matcher":"reviewer", "hooks":[{"type":"command","command":format!("{command}; case \"$payload\" in *'\"stop_hook_active\":true'*) ;; *) printf '%s' '{stop}' ;; esac")}]}]}}))
    .unwrap();
    let mut trust = PluginHookTrustStore::load_or_default(&state.join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root, &manifest).unwrap();
    let mut runner = crate::hooks::new_hook_runner();
    for handler in codex_handlers("fixture", root, &manifest, &trust).unwrap() {
        runner.register(handler);
    }
    Arc::new(
        ToolUseContext::new(parent, Some(root.to_string_lossy().into_owned()))
            .with_turn_id("parent-turn")
            .with_model("fixture")
            .with_hook_runner(Arc::new(runner)),
    )
}
#[tokio::test]
async fn codex_plugin_compat_t43_persistent_success_failure_and_cancellation() {
    use crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected;
    use crate::session::{
        child_agent::{self, ChildAgentRunInput},
        session,
    };
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let directory = root.path().to_string_lossy().into_owned();
    let d = directory.clone();
    let parent = crate::project::instance::provide(d.clone(), None, move || {
        Box::pin(async move {
            session::create_next(session::CreateInput {
                parent_id: None,
                title: None,
                directory: d,
                permission: None,
            })
            .await
        })
    })
    .await
    .unwrap()
    .unwrap();
    let ctx = fixture(root.path(), state.path(), &parent.id);
    for outcome in ["success", "failure", "cancel"] {
        let input = ChildAgentRunInput {
            parent_session_id: parent.id.clone(),
            directory: directory.clone(),
            agent_key: "reviewer".into(),
            model: None,
            prompt: "synthetic task".into(),
            context: vec![],
            allowed_tools: vec![],
            default_allowed_skills: vec![],
            skill_policy: Default::default(),
            trusted_user_selected_skills: vec![],
        };
        let result = crate::tools::scope_tool_use_context(ctx.clone(), async {
            if outcome == "cancel" {
                let created = child_agent::create(input).await.unwrap();
                child_agent::cancel_created(created, "synthetic cancellation").await.unwrap()
            } else {
                child_agent::run_with_stream_executor(input, move |_| async move {
                    if outcome == "failure" {
                        Err(crate::gateway::ApiError::internal("synthetic provider failure"))
                    } else {
                        Ok(GatewayChatStreamCollected {
                            output: "handoff".into(),
                            message_id: None,
                            parent_message_id: None,
                        })
                    }
                })
                .await
                .unwrap()
            }
        })
        .await;
        let child = result.child_session_id.clone();
        let expected_parent = parent.id.clone();
        crate::project::instance::provide(directory.clone(), None, move || {
            Box::pin(async move {
                assert_eq!(session::get(&child).await.unwrap().parent_id, Some(expected_parent));
            })
        })
        .await
        .unwrap();
        let events = std::fs::read_to_string(state.path().join("events"))
            .expect("persistent execution must notify plugin");
        let events: Vec<Value> = events
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .filter(|v: &Value| v["agent_id"] == result.child_session_id)
            .collect();
        assert_eq!(events.last().unwrap()["hook_event_name"], "SubagentStop");
        assert_eq!(events.len(), if outcome == "cancel" { 1 } else { 2 });
        assert_eq!(
            events.last().unwrap()["last_assistant_message"],
            if outcome == "success" { json!("handoff") } else { Value::Null }
        );
    }
}

#[tokio::test]
async fn codex_plugin_compat_t43_continues_only_the_blocked_child() {
    use crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected;
    use crate::session::{
        child_agent::{self, ChildAgentRunInput},
        session,
    };
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let directory = root.path().to_string_lossy().into_owned();
    let d = directory.clone();
    let parent = crate::project::instance::provide(d.clone(), None, move || {
        Box::pin(async move {
            session::create_next(session::CreateInput {
                parent_id: None,
                title: None,
                directory: d,
                permission: None,
            })
            .await
        })
    })
    .await
    .unwrap()
    .unwrap();
    let ctx = fixture_output(
        root.path(),
        state.path(),
        &parent.id,
        r#"{"continue":false,"hookSpecificOutput":{"hookEventName":"SubagentStart","additionalContext":"child only"}}"#,
        r#"{"decision":"block","reason":"finish child work"}"#,
    );
    let input = ChildAgentRunInput {
        parent_session_id: parent.id.clone(),
        directory,
        agent_key: "reviewer".into(),
        model: None,
        prompt: "synthetic task".into(),
        context: vec![],
        allowed_tools: vec![],
        default_allowed_skills: vec![],
        skill_policy: Default::default(),
        trusted_user_selected_skills: vec![],
    };
    let result = crate::tools::scope_tool_use_context(ctx.clone(), async {
        let created = child_agent::create(input).await.unwrap();
        let child = created.child_session_id.clone();
        let mut count = 0;
        child_agent::run_created_with_repeating_executor(created, move |request| {
            count += 1;
            assert!(count <= 2, "stop continuation must terminate");
            assert_eq!(request.session_id.as_ref().unwrap().as_ref(), child);
            if count == 1 {
                assert!(request.messages.iter().any(|m| m["content"] == "child only"));
            } else {
                assert_eq!(request.messages.last().unwrap()["content"], "finish child work");
            }
            async move {
                Ok(GatewayChatStreamCollected {
                    output: format!("handoff {count}"),
                    message_id: None,
                    parent_message_id: None,
                })
            }
        })
        .await
        .unwrap()
    })
    .await;
    assert_eq!(result.output.as_deref(), Some("handoff 2"));
    assert!(ctx.take_hook_contexts().is_empty());
    let contents = std::fs::read_to_string(state.path().join("events")).unwrap();
    let events: Vec<Value> = contents.lines().map(|s| serde_json::from_str(s).unwrap()).collect();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["hook_event_name"], "SubagentStart");
    assert_eq!(events[1]["stop_hook_active"], false);
    assert_eq!(events[2]["stop_hook_active"], true);
}
