use super::hook_subagent::{emit, validate};
use crate::{hooks::HookRunner, tools::ToolUseContext};
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
    let mut runner = HookRunner::new();
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
async fn codex_plugin_compat_t43_binds_events_to_child_session() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let ctx = fixture(root.path(), state.path(), "parent");
    emit(Some(ctx.clone()), "parent", "child", "reviewer", None, false).await.unwrap();
    emit(Some(ctx), "parent", "child", "reviewer", Some("handoff"), true).await.unwrap();
    let events = std::fs::read_to_string(state.path().join("events"))
        .expect("child lifecycle must reach plugin");
    let events: Vec<Value> = events.lines().map(|s| serde_json::from_str(s).unwrap()).collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["hook_event_name"], "SubagentStart");
    assert_eq!(events[1]["hook_event_name"], "SubagentStop");
    for event in &events {
        assert_eq!(event["agent_id"], "child");
        assert_eq!(event["session_id"], "parent");
        assert_eq!(event["agent_type"], "reviewer");
    }
    assert_eq!(events[1]["last_assistant_message"], "handoff");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
#[test]
fn codex_plugin_compat_t43_rejects_invalid_input() {
    assert!(validate(&json!({})).is_err());
    assert!(validate(&json!({"session_id":"same","agent_id":"same","agent_type":"reviewer","cwd":"/tmp","hook_event_name":"SubagentStart"})).is_err());
}
#[tokio::test]
async fn codex_plugin_compat_t43_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let ctx = fixture(root.path(), state.path(), "parent");
    assert!(emit(Some(ctx.clone()), "other", "child", "reviewer", None, false).await.is_err());
    assert!(!state.path().join("events").exists());
    let (a, b) = tokio::join!(
        emit(Some(ctx.clone()), "parent", "child-a", "reviewer", None, false),
        emit(Some(ctx.clone()), "parent", "child-b", "reviewer", None, false)
    );
    a.unwrap();
    b.unwrap();
    let events = std::fs::read_to_string(state.path().join("events"))
        .expect("concurrent children must reach plugin");
    assert!(events.contains("child-a"));
    assert!(events.contains("child-b"));
    assert!(ctx.take_hook_contexts().is_empty());
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
async fn codex_plugin_compat_t43_routes_context_and_stop_to_child() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let ctx = fixture_output(
        root.path(),
        state.path(),
        "parent",
        r#"{"continue":false,"hookSpecificOutput":{"hookEventName":"SubagentStart","additionalContext":"child only"}}"#,
        r#"{"decision":"block","reason":"finish child work"}"#,
    );
    assert_eq!(
        emit(Some(ctx.clone()), "parent", "child", "reviewer", None, false).await.unwrap(),
        vec!["child only"]
    );
    assert_eq!(
        emit(Some(ctx.clone()), "parent", "child", "reviewer", Some("done"), true).await.unwrap(),
        vec!["finish child work"]
    );
    assert!(ctx.take_hook_contexts().is_empty());
    assert!(emit(Some(ctx), "parent", "child", "other", None, false).await.unwrap().is_empty());
}
#[test]
fn codex_plugin_compat_t43_output_controls_follow_frozen_baseline() {
    use super::hook_subagent::parse;
    assert!(parse(Some(0), br#"{"continue":false}"#, b"", false).is_ok());
    assert!(parse(Some(0), br#"{"continue":false}"#, b"", true).is_ok());
    assert_eq!(parse(Some(2), b"", b"continue child", true).unwrap()["reason"], "continue child");
    assert!(parse(Some(1), b"private payload", b"private error", false).unwrap().is_null());
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
