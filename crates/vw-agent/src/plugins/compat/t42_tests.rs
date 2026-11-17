use crate::{provider::provider, session::message};
use crate::{
    session::compaction::{CompactionOutcome, ProcessInput},
    tools::ToolUseContext,
};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};

pub(crate) fn fixture(
    root: &std::path::Path,
    state: &std::path::Path,
    session: &str,
) -> Arc<ToolUseContext> {
    fixture_output(root, state, session, "", "")
}
fn fixture_output(
    root: &std::path::Path,
    state: &std::path::Path,
    session: &str,
    pre: &str,
    post: &str,
) -> Arc<ToolUseContext> {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let command = format!(
        "cat >> '{}'; printf '\\n' >> '{}'",
        state.join("events").display(),
        state.join("events").display()
    );
    let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{
        "PreCompact":[{"hooks":[{"type":"command","command":format!("{command}; printf '%s' '{pre}'")}]}],
        "PostCompact":[{"hooks":[{"type":"command","command":format!("{command}; printf '%s' '{post}'")}]}]}}))
    .unwrap();
    let mut trust = PluginHookTrustStore::load_or_default(&state.join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root, &manifest).unwrap();
    let mut runner = crate::hooks::new_hook_runner();
    for handler in codex_handlers("fixture", root, &manifest, &trust).unwrap() {
        runner.register(handler);
    }
    Arc::new(
        ToolUseContext::new(session, Some(root.to_string_lossy().into_owned()))
            .with_turn_id("turn")
            .with_model("fixture")
            .with_hook_runner(Arc::new(runner)),
    )
}
fn capability_io(text: bool) -> provider::CapabilityIO {
    provider::CapabilityIO { text, audio: false, image: false, video: false, pdf: false }
}

fn model_with_limits(context: u64, input: Option<u64>, output: u64) -> provider::Model {
    provider::Model {
        id: "model-fixture".to_string(),
        provider_id: "provider-fixture".to_string(),
        api: provider::ApiInfo {
            protocols: None,
            id: "api-fixture".to_string(),
            url: "https://example.test/v1".to_string(),
            adapter: provider::default_adapter(),
        },
        name: "Fixture Model".to_string(),
        family: None,
        capabilities: provider::Capabilities {
            native_tool_search: false,
            catalog_execution: None,
            temperature: true,
            reasoning: false,
            reasoning_options: None,
            attachment: false,
            toolcall: true,
            input: capability_io(true),
            output: capability_io(true),
            interleaved: provider::InterleavedCapability::Bool(false),
        },
        cost: provider::ModelCost {
            input: 0.0,
            output: 0.0,
            cache: provider::ModelCostCache { read: 0.0, write: 0.0 },
            experimental_over_200k: None,
        },
        limit: provider::ModelLimit { context, input, output },
        status: "stable".to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: "2026-01-01".to_string(),
        variants: HashMap::new(),
    }
}

fn model_ref() -> message::ModelRef {
    message::ModelRef {
        provider_id: "provider-fixture".to_string(),
        model_id: "model-fixture".to_string(),
    }
}

fn part_base(session_id: &str, message_id: &str, part_id: &str) -> message::PartBase {
    message::PartBase {
        id: part_id.to_string(),
        session_id: session_id.to_string(),
        message_id: message_id.to_string(),
    }
}

fn text_part(session_id: &str, message_id: &str, part_id: &str, text: &str) -> message::Part {
    message::Part::Text(message::TextPart {
        base: part_base(session_id, message_id, part_id),
        text: text.to_string(),
        synthetic: None,
        ignored: None,
        time: None,
        metadata: None,
    })
}

fn user_message(session_id: &str, id: &str, text: &str) -> message::WithParts {
    let parts = if text.is_empty() {
        Vec::new()
    } else {
        vec![text_part(session_id, id, &format!("{id}_text"), text)]
    };

    message::WithParts {
        info: message::Info::User(Box::new(message::UserInfo {
            id: id.to_string(),
            session_id: session_id.to_string(),
            time: message::UserTime { created: 1 },
            summary: None,
            agent: "agent-fixture".to_string(),
            model: model_ref(),
            system: None,
            tools: None,
            variant: Some("variant-a".to_string()),
        })),
        parts,
    }
}

async fn run(session: &str, auto: bool, fail: bool) -> CompactionOutcome {
    attempt(session, auto, fail).await.unwrap()
}
async fn attempt(
    session: &str,
    auto: bool,
    fail: bool,
) -> Result<CompactionOutcome, crate::session::session::Error> {
    session_engine::compaction::process_with_summarizer::<crate::tools::context::AgentContextHost>(
        ProcessInput {
            session_id: session.into(),
            parent_id: "boundary".into(),
            auto,
            messages: vec![
                user_message(session, "old", "fixture history"),
                user_message(session, "boundary", ""),
            ],
        },
        &model_with_limits(100_000, None, 10_000),
        |_| {
            if fail { Err("provider failed".into()) } else { Ok("fixture summary".into()) }
        },
    )
    .await
}
#[tokio::test]
async fn codex_plugin_compat_t42_emits_events_around_compaction() {
    for auto in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let session = format!("t42-{}", uuid::Uuid::new_v4());
        let ctx = fixture(root.path(), state.path(), &session);
        crate::tools::scope_tool_use_context(ctx, async {
            assert!(matches!(run(&session, auto, true).await, CompactionOutcome::Failed { .. }));
            assert!(matches!(
                run(&session, auto, false).await,
                CompactionOutcome::Compacted { .. }
            ));
        })
        .await;
        let contents = std::fs::read_to_string(state.path().join("events"))
            .expect("compaction must notify plugins");
        let events: Vec<Value> =
            contents.lines().map(|s| serde_json::from_str(s).unwrap()).collect();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0]["hook_event_name"], "PreCompact");
        assert_eq!(events[1]["hook_event_name"], "PreCompact");
        assert_eq!(events[2]["hook_event_name"], "PostCompact");
        assert_eq!(events[2]["trigger"], if auto { "auto" } else { "manual" });
        assert_eq!(events[2]["turn_id"], "turn");
        assert!(events[2].get("messages").is_none());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
#[tokio::test]
async fn codex_plugin_compat_t42_applies_stop_at_the_correct_boundary() {
    for post in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let session = format!("t42-stop-{}", uuid::Uuid::new_v4());
        let stop = r#"{"continue":false,"stopReason":"fixture stop"}"#;
        let ctx = fixture_output(
            root.path(),
            state.path(),
            &session,
            if post { "" } else { stop },
            if post { stop } else { "" },
        );
        let error = crate::tools::scope_tool_use_context(ctx, attempt(&session, false, false))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("fixture stop"));
        let events = std::fs::read_to_string(state.path().join("events")).unwrap();
        assert_eq!(events.lines().count(), if post { 2 } else { 1 });
        let source = vec![json!({"role":"user","content":"fixture history"})];
        let restored =
            crate::session::compaction_checkpoint::restore(&session, &source).await.unwrap();
        assert_eq!(restored[0]["content"].as_str().unwrap().contains("fixture summary"), post);
    }
}
