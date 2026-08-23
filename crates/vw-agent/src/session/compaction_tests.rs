//! 验证会话压缩阈值、消息保留、工具配对与持久化边界。

use super::*;
use crate::app::agent::provider::provider;
use crate::app::agent::session::message;
use serde_json::Map;
use std::collections::HashMap;

static COMPACTION_STORAGE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn capability_io(text: bool) -> provider::CapabilityIO {
    provider::CapabilityIO { text, audio: false, image: false, video: false, pdf: false }
}

fn model_with_limits(context: u64, input: Option<u64>, output: u64) -> provider::Model {
    provider::Model {
        id: "model-fixture".to_string(),
        provider_id: "provider-fixture".to_string(),
        api: provider::ApiInfo {
            id: "api-fixture".to_string(),
            url: "https://example.test/v1".to_string(),
            adapter: provider::default_adapter(),
        },
        name: "Fixture Model".to_string(),
        family: None,
        capabilities: provider::Capabilities {
            temperature: true,
            reasoning: false,
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

fn assistant_message(
    session_id: &str,
    id: &str,
    parent_id: &str,
    text: &str,
    summary: Option<bool>,
) -> message::WithParts {
    let parts = if text.is_empty() {
        Vec::new()
    } else {
        vec![text_part(session_id, id, &format!("{id}_text"), text)]
    };

    message::WithParts {
        info: message::Info::Assistant(Box::new(message::AssistantInfo {
            id: id.to_string(),
            session_id: session_id.to_string(),
            time: message::AssistantTime {
                created: 1,
                completed: None,
                processing_elapsed_ms: None,
                llm_elapsed_ms: None,
            },
            error: None,
            parent_id: parent_id.to_string(),
            model_id: "model-fixture".to_string(),
            provider_id: "provider-fixture".to_string(),
            mode: "chat".to_string(),
            agent: "agent-fixture".to_string(),
            path: message::PathInfo { cwd: String::new(), root: String::new() },
            summary,
            cost: 0.0,
            tokens: message::TokenInfo {
                total: None,
                input: 0,
                output: 0,
                reasoning: 0,
                cache: message::TokenCacheInfo { read: 0, write: 0 },
            },
            variant: Some("variant-a".to_string()),
            finish: None,
        })),
        parts,
    }
}

fn completed_tool(
    session_id: &str,
    message_id: &str,
    part_id: &str,
    tool: &str,
    output: String,
    compacted: Option<u64>,
) -> message::Part {
    message::Part::Tool(message::ToolPart {
        base: part_base(session_id, message_id, part_id),
        call_id: format!("call-{part_id}"),
        tool: tool.to_string(),
        state: message::ToolState::Completed(message::ToolStateCompleted {
            input: Map::new(),
            output,
            title: "tool title".to_string(),
            metadata: Map::new(),
            time: message::ToolStateCompletedTime { start: 1, end: 2, compacted },
            attachments: None,
        }),
        metadata: None,
    })
}

fn pending_tool(session_id: &str, message_id: &str, part_id: &str, raw: String) -> message::Part {
    message::Part::Tool(message::ToolPart {
        base: part_base(session_id, message_id, part_id),
        call_id: format!("call-{part_id}"),
        tool: "shell".to_string(),
        state: message::ToolState::Pending(message::ToolStatePending { input: Map::new(), raw }),
        metadata: None,
    })
}

fn token_info(
    total: Option<i64>,
    input: i64,
    output: i64,
    read: i64,
    write: i64,
) -> message::TokenInfo {
    message::TokenInfo {
        total,
        input,
        output,
        reasoning: 0,
        cache: message::TokenCacheInfo { read, write },
    }
}

fn unique_session(prefix: &str) -> String {
    format!("{prefix}-{}-{}", std::process::id(), now_ms())
}

async fn write_message_with_parts(message: &message::WithParts) {
    let message_id = message.info.id().to_string();
    let session_id = match &message.info {
        message::Info::User(info) => info.session_id.as_str(),
        message::Info::Assistant(info) => info.session_id.as_str(),
    };
    if let Ok(existing_parts) = message::parts(session_id, &message_id).await {
        for part in existing_parts {
            let _ = crate::app::agent::storage::remove(&["part", &message_id, part.id()]).await;
        }
    }
    message::update_message(&message.info).await.expect("message write");
    for part in &message.parts {
        message::update_part(part).await.expect("part write");
    }
}

async fn cleanup_session(session_id: &str) {
    if let Ok(messages) = message::messages(session_id, None).await {
        for msg in messages {
            let message_id = msg.info.id().to_string();
            if let Ok(parts) = message::parts(session_id, &message_id).await {
                for part in parts {
                    let _ =
                        crate::app::agent::storage::remove(&["part", &message_id, part.id()]).await;
                    let _ = crate::app::agent::storage::remove(&[
                        "part",
                        session_id,
                        &message_id,
                        part.id(),
                    ])
                    .await;
                }
            }
            let _ = crate::app::agent::storage::remove(&["message", session_id, &message_id]).await;
        }
    }
}

#[tokio::test]
async fn overflow_is_false_for_unlimited_or_unusable_context() {
    let tokens = token_info(Some(1_000_000), 0, 0, 0, 0);

    assert!(!is_overflow(&tokens, &model_with_limits(0, None, 0)).await);
    assert!(!is_overflow(&tokens, &model_with_limits(10_000, Some(10), 50_000)).await);
}

#[tokio::test]
async fn overflow_uses_total_when_present_and_rounds_threshold() {
    let model = model_with_limits(100_000, None, 10_000);

    assert!(!is_overflow(&token_info(Some(80_999), 99_999, 0, 0, 0), &model).await);
    assert!(is_overflow(&token_info(Some(81_000), 1, 1, 1, 1), &model).await);
}

#[tokio::test]
async fn overflow_falls_back_to_component_sum_and_input_limit() {
    let model = model_with_limits(200_000, Some(100_000), 20_000);

    assert!(!is_overflow(&token_info(None, 80_000, 9_000, 900, 99), &model).await);
    assert!(is_overflow(&token_info(None, 80_000, 9_000, 900, 100), &model).await);
}

#[tokio::test]
async fn overflow_treats_negative_counts_as_zero() {
    let model = model_with_limits(100_000, None, 10_000);

    assert!(!is_overflow(&token_info(Some(-1), 100_000, 100_000, 0, 0), &model).await);
    assert!(!is_overflow(&token_info(None, -10, -20, -30, -40), &model).await);
}

#[tokio::test]
// 上下文预算先预留模型输出空间，不能把整个窗口都耗在输入上。
async fn overflow_reserves_model_output_space() {
    let model = model_with_limits(100_000, None, 50_000);

    assert!(!is_overflow(&token_info(Some(44_999), 0, 0, 0, 0), &model).await);
    assert!(is_overflow(&token_info(Some(45_000), 0, 0, 0, 0), &model).await);
}

#[tokio::test]
async fn instance_directory_opt_returns_directory_inside_project_context() {
    let project = tempfile::tempdir().expect("project dir");
    let expected = project.path().to_string_lossy().to_string();

    let directory = instance::provide(project.path(), None, move || {
        Box::pin(async move { instance_directory_opt() })
    })
    .await
    .expect("project instance should provide");

    assert_eq!(directory.as_deref(), Some(expected.as_str()));
}

#[test]
fn extract_recent_text_formats_roles_trims_and_skips_empty_text() {
    let session_id = "extract-session";
    let messages = vec![
        user_message(session_id, "msg_1", "  hello  "),
        assistant_message(session_id, "msg_2", "msg_1", "\nanswer\n", None),
        user_message(session_id, "msg_3", "   "),
    ];

    assert_eq!(extract_recent_text(&messages, 100), "user: hello\nassistant: answer");
}

#[test]
fn extract_recent_text_uses_recent_twenty_messages_and_truncates() {
    let session_id = "recent-session";
    let messages = (0..21)
        .map(|idx| user_message(session_id, &format!("msg_{idx:02}"), &format!("text-{idx:02}")))
        .collect::<Vec<_>>();

    let full = extract_recent_text(&messages, 1_000);
    assert!(!full.contains("text-00"));
    assert!(full.starts_with("user: text-01"));
    assert!(full.ends_with("user: text-20"));

    let truncated = extract_recent_text(&messages, 12);
    assert_eq!(truncated.len(), 12);
    assert_eq!(truncated, "user: text-0");
}

#[test]
fn extract_recent_text_returns_empty_at_zero_character_limit() {
    let messages = vec![user_message("zero-limit-session", "msg_1", "visible text")];

    assert_eq!(extract_recent_text(&messages, 0), "");
}

#[test]
// 恢复时先加载代理指令再重放当前技能正文，保持原始上下文优先级。
fn compaction_restore_reloads_agents_before_replaying_current_invoked_skill_body() {
    let temp = tempfile::tempdir().expect("temporary skills root");
    let skill_dir = temp.path().join("skills/compaction-skill");
    std::fs::create_dir_all(&skill_dir).expect("skill directory");
    let skill_file = skill_dir.join("SKILL.md");
    std::fs::write(&skill_file, "---\nallowed-tools: [read]\nmodel: provider/old\n---\n\nOLD")
        .expect("write initial skill");
    let mut skills = crate::app::agent::skills::SkillListingSessionState::default();
    skills.register_invoked(
        "main",
        crate::app::agent::skills::InvokedSkillRecord::new("compaction-skill", &skill_file, None)
            .expect("invocation record"),
    );
    std::fs::write(&skill_file, "---\nallowed-tools: [write]\nmodel: provider/new\n---\n\nNEW")
        .expect("write current skill");
    let mut messages = vec![json!({"role": "user", "content": "history"})];
    let mut reloaded_agents = false;

    let updates = restore_invoked_skills_after_compaction(&mut messages, &skills, "main", || {
        reloaded_agents = true
    })
    .expect("replay invoked skill");

    assert!(reloaded_agents);
    assert!(messages[0]["content"].as_str().is_some_and(|content| content.contains("NEW")));
    assert_eq!(messages[1]["content"], "history");
    assert!(updates.iter().any(|update| update.key == "skill.allowed_tools"));
    assert!(updates.iter().any(|update| update.value == json!("provider/new")));
}

#[tokio::test]
async fn process_returns_stop_when_parent_user_message_is_missing() {
    let result = process(
        ProcessInput {
            parent_id: "missing".to_string(),
            messages: vec![assistant_message("process-stop", "msg_1", "parent", "text", None)],
            session_id: "process-stop".to_string(),
            auto: true,
        },
        &model_with_limits(100_000, None, 10_000),
    )
    .await
    .expect("process should not fail for missing parent");

    assert!(matches!(result, CompactionOutcome::NotNeeded { .. }));
}

#[tokio::test]
async fn internal_compaction_replaces_covered_history_with_a_model_summary() {
    let _storage_lock = COMPACTION_STORAGE_LOCK.lock().await;
    let session_id = unique_session("internal-compaction-success");
    let compact_request_id = "msg_compact".to_string();
    let history = vec![
        user_message(&session_id, "msg_old_user", "Investigate the failing build"),
        assistant_message(
            &session_id,
            "msg_old_assistant",
            "msg_old_user",
            "The compiler reports an ownership error.",
            None,
        ),
        user_message(&session_id, &compact_request_id, ""),
    ];
    for message in &history {
        write_message_with_parts(message).await;
    }

    let result = process_with_summarizer(
        ProcessInput {
            parent_id: compact_request_id.clone(),
            messages: history,
            session_id: session_id.clone(),
            auto: false,
        },
        &model_with_limits(100_000, None, 10_000),
        |request| {
            assert!(request.prompt.contains("Investigate the failing build"));
            assert!(request.tools.is_empty());
            Ok("Build investigation: ownership error remains.".to_string())
        },
    )
    .await
    .expect("successful summary should persist atomically");

    let CompactionOutcome::Compacted { stats, .. } = result else {
        panic!("expected a completed compaction");
    };
    assert_eq!(stats.covered_messages, 2);
    assert_eq!(
        stats.summary_chars,
        "Build investigation: ownership error remains.".chars().count()
    );

    let stored = message::messages(&session_id, None).await.expect("stored history");
    assert_eq!(stored.len(), 2);
    assert!(
        stored
            .iter()
            .all(|message| !matches!(message.info.id(), "msg_old_user" | "msg_old_assistant"))
    );
    let summary = stored
        .iter()
        .find(|message| matches!(&message.info, message::Info::Assistant(info) if info.summary == Some(true)))
        .expect("summary boundary should be persisted");
    let parts = message::parts(&session_id, summary.info.id()).await.expect("summary parts");
    assert!(matches!(
        &parts[..],
        [message::Part::Compaction(_), message::Part::Text(text)]
            if text.text == "Build investigation: ownership error remains."
    ));

    cleanup_session(&session_id).await;
}

#[tokio::test]
// 摘要失败或为空时保留原历史，避免压缩操作造成不可逆信息丢失。
async fn internal_compaction_keeps_history_when_summary_is_blank_or_fails() {
    let _storage_lock = COMPACTION_STORAGE_LOCK.lock().await;
    let session_id = unique_session("internal-compaction-atomic-failure");
    let compact_request_id = "msg_compact".to_string();
    let history = vec![
        user_message(&session_id, "msg_old_user", "Keep this message"),
        user_message(&session_id, &compact_request_id, ""),
    ];
    for message in &history {
        write_message_with_parts(message).await;
    }

    for response in [Ok("   ".to_string()), Err("provider unavailable".to_string())] {
        let result = process_with_summarizer(
            ProcessInput {
                parent_id: compact_request_id.clone(),
                messages: history.clone(),
                session_id: session_id.clone(),
                auto: false,
            },
            &model_with_limits(100_000, None, 10_000),
            |_| response.clone(),
        )
        .await
        .expect("model failure is a recoverable compaction outcome");
        assert!(matches!(result, CompactionOutcome::Failed { .. }));
        let stored = message::messages(&session_id, None).await.expect("history remains readable");
        assert_eq!(stored.len(), 2);
        assert!(stored.iter().any(|message| message.info.id() == "msg_old_user"));
    }

    cleanup_session(&session_id).await;
}

#[test]
fn internal_compaction_profile_has_no_tool_or_delegation_capability() {
    let profile = internal_compaction_profile();

    assert_eq!(profile.name, "session-compaction");
    assert!(profile.prompt.is_none());
    assert!(profile.options.is_empty());
}

#[tokio::test]
async fn process_does_not_compact_when_the_boundary_has_no_prior_history() {
    let session_id = unique_session("compaction-process-auto");
    let parent_id = "msg_parent".to_string();
    let input_messages = vec![
        user_message(&session_id, &parent_id, "initial question"),
        assistant_message(&session_id, "msg_answer", &parent_id, "initial answer", None),
    ];

    let result = process(
        ProcessInput {
            parent_id: parent_id.clone(),
            messages: input_messages,
            session_id: session_id.clone(),
            auto: true,
        },
        &model_with_limits(100_000, None, 10_000),
    )
    .await
    .expect("process should recognize insufficient history");

    assert!(matches!(result, CompactionOutcome::NotNeeded { .. }));
    let stored = message::messages(&session_id, None).await.expect("stored messages");
    assert!(stored.is_empty());

    cleanup_session(&session_id).await;
}

#[tokio::test]
async fn process_does_not_synthesize_a_summary_for_blank_history() {
    let session_id = unique_session("compaction-process-manual");
    let parent_id = "msg_parent".to_string();

    let result = process(
        ProcessInput {
            parent_id: parent_id.clone(),
            messages: vec![user_message(&session_id, &parent_id, "")],
            session_id: session_id.clone(),
            auto: false,
        },
        &model_with_limits(100_000, None, 10_000),
    )
    .await
    .expect("blank history should not call a model");

    assert!(matches!(result, CompactionOutcome::NotNeeded { .. }));
    let stored = message::messages(&session_id, None).await.expect("stored messages");
    assert!(stored.is_empty());

    cleanup_session(&session_id).await;
}

#[tokio::test]
async fn create_writes_user_message_with_compaction_part() {
    let session_id = unique_session("compaction-create");

    create(CreateInput {
        session_id: session_id.clone(),
        agent: "agent-fixture".to_string(),
        model: model_ref(),
        auto: true,
    })
    .await
    .expect("create should write message and part");

    let stored = message::messages(&session_id, None).await.expect("stored messages");
    assert_eq!(stored.len(), 1);
    let message::Info::User(info) = &stored[0].info else {
        panic!("compaction create should write user message");
    };
    assert_eq!(info.agent, "agent-fixture");
    assert_eq!(info.model.model_id, "model-fixture");

    let parts = message::parts(&session_id, &info.id).await.expect("parts");
    assert_eq!(parts.len(), 1);
    let message::Part::Compaction(part) = &parts[0] else {
        panic!("part should be compaction");
    };
    assert!(part.auto);
    assert_eq!(part.base.session_id, session_id);
    assert_eq!(part.base.message_id, info.id);

    cleanup_session(&session_id).await;
}

#[tokio::test]
async fn create_preserves_manual_compaction_flag() {
    let session_id = unique_session("compaction-create-manual");

    create(CreateInput {
        session_id: session_id.clone(),
        agent: "agent-fixture".to_string(),
        model: model_ref(),
        auto: false,
    })
    .await
    .expect("create should write manual compaction request");

    let stored = message::messages(&session_id, None).await.expect("stored messages");
    let message_id = stored[0].info.id().to_string();
    let parts = message::parts(&session_id, &message_id).await.expect("parts");
    let message::Part::Compaction(part) = &parts[0] else {
        panic!("part should be compaction");
    };
    assert!(!part.auto);

    cleanup_session(&session_id).await;
}

#[tokio::test]
async fn prune_marks_only_old_unprotected_outputs_after_thresholds() {
    let _storage_lock = COMPACTION_STORAGE_LOCK.lock().await;
    let session_id = unique_session("compaction-prune");
    let mut old_assistant = assistant_message(&session_id, "msg_002", "msg_001", "", None);
    old_assistant.parts = vec![
        completed_tool(
            &session_id,
            "msg_002",
            "part_old_pruned",
            "shell",
            "x".repeat(100_004),
            None,
        ),
        completed_tool(
            &session_id,
            "msg_002",
            "part_old_protected_budget",
            "shell",
            "y".repeat(100_004),
            None,
        ),
        completed_tool(&session_id, "msg_002", "part_skill", "skill", "z".repeat(200_000), None),
    ];
    let mut recent_assistant = assistant_message(&session_id, "msg_004", "msg_003", "", None);
    recent_assistant.parts = vec![completed_tool(
        &session_id,
        "msg_004",
        "part_recent",
        "shell",
        "r".repeat(200_000),
        None,
    )];

    for msg in [
        user_message(&session_id, "msg_001", "first"),
        old_assistant,
        user_message(&session_id, "msg_003", "second"),
        recent_assistant,
        user_message(&session_id, "msg_005", "third"),
    ] {
        write_message_with_parts(&msg).await;
    }

    prune(&session_id).await.expect("prune should complete");

    let old_parts = message::parts(&session_id, "msg_002").await.expect("old parts");
    let compacted = old_parts
        .iter()
        .filter_map(|part| match part {
            message::Part::Tool(tp) => match &tp.state {
                message::ToolState::Completed(state) => {
                    Some((tp.base.id.as_str(), state.time.compacted.is_some()))
                }
                _ => None,
            },
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    assert_eq!(compacted.get("part_old_pruned"), Some(&false));
    assert_eq!(compacted.get("part_old_protected_budget"), Some(&true));
    assert_eq!(compacted.get("part_skill"), Some(&false));

    let recent_parts = message::parts(&session_id, "msg_004").await.expect("recent parts");
    let message::Part::Tool(recent_tool) = &recent_parts[0] else {
        panic!("recent part should be tool");
    };
    let message::ToolState::Completed(recent_state) = &recent_tool.state else {
        panic!("recent tool should be completed");
    };
    assert!(recent_state.time.compacted.is_none());

    cleanup_session(&session_id).await;
}

#[tokio::test]
async fn prune_stops_at_summary_messages() {
    let _storage_lock = COMPACTION_STORAGE_LOCK.lock().await;
    let session_id = unique_session("compaction-prune-summary");
    let mut old_assistant = assistant_message(&session_id, "msg_002", "msg_001", "", None);
    old_assistant.parts = vec![
        completed_tool(
            &session_id,
            "msg_002",
            "part_before_summary_a",
            "shell",
            "a".repeat(100_004),
            None,
        ),
        completed_tool(
            &session_id,
            "msg_002",
            "part_before_summary_b",
            "shell",
            "b".repeat(100_004),
            None,
        ),
    ];

    for msg in [
        user_message(&session_id, "msg_001", "first"),
        old_assistant,
        assistant_message(&session_id, "msg_003", "msg_001", "summary", Some(true)),
        user_message(&session_id, "msg_004", "second"),
        user_message(&session_id, "msg_005", "third"),
    ] {
        write_message_with_parts(&msg).await;
    }

    prune(&session_id).await.expect("prune should stop at summary");

    let parts = message::parts(&session_id, "msg_002").await.expect("old parts");
    for part in parts {
        let message::Part::Tool(tool) = part else {
            continue;
        };
        let message::ToolState::Completed(state) = tool.state else {
            continue;
        };
        assert!(state.time.compacted.is_none());
    }

    cleanup_session(&session_id).await;
}

#[tokio::test]
async fn prune_keeps_candidate_when_prunable_tokens_equal_minimum() {
    let _storage_lock = COMPACTION_STORAGE_LOCK.lock().await;
    let session_id = unique_session("compaction-prune-minimum");
    let mut old_assistant = assistant_message(&session_id, "msg_002", "msg_001", "", None);
    old_assistant.parts = vec![
        completed_tool(
            &session_id,
            "msg_002",
            "part_a_candidate",
            "shell",
            "a".repeat(80_000),
            None,
        ),
        completed_tool(
            &session_id,
            "msg_002",
            "part_z_protected_budget",
            "shell",
            "z".repeat(120_000),
            None,
        ),
    ];

    for msg in [
        user_message(&session_id, "msg_001", "first"),
        old_assistant,
        user_message(&session_id, "msg_003", "second"),
        user_message(&session_id, "msg_004", "third"),
    ] {
        write_message_with_parts(&msg).await;
    }

    prune(&session_id).await.expect("prune should keep the exact minimum");

    let parts = message::parts(&session_id, "msg_002").await.expect("old parts");
    for part in parts {
        let message::Part::Tool(tool) = part else {
            continue;
        };
        let message::ToolState::Completed(state) = tool.state else {
            continue;
        };
        assert!(state.time.compacted.is_none());
    }

    cleanup_session(&session_id).await;
}

#[tokio::test]
async fn prune_stops_at_previously_compacted_tool() {
    let _storage_lock = COMPACTION_STORAGE_LOCK.lock().await;
    let session_id = unique_session("compaction-prune-compacted");
    let mut old_assistant = assistant_message(&session_id, "msg_002", "msg_001", "", None);
    old_assistant.parts = vec![
        completed_tool(&session_id, "msg_002", "part_a_older", "shell", "a".repeat(200_000), None),
        completed_tool(&session_id, "msg_002", "part_b_older", "shell", "b".repeat(200_000), None),
        completed_tool(
            &session_id,
            "msg_002",
            "part_z_compacted",
            "shell",
            "z".repeat(200_000),
            Some(42),
        ),
    ];

    for msg in [
        user_message(&session_id, "msg_001", "first"),
        old_assistant,
        user_message(&session_id, "msg_003", "second"),
        user_message(&session_id, "msg_004", "third"),
    ] {
        write_message_with_parts(&msg).await;
    }

    prune(&session_id).await.expect("prune should stop at compacted tool");

    let parts = message::parts(&session_id, "msg_002").await.expect("old parts");
    let compacted = parts
        .iter()
        .filter_map(|part| match part {
            message::Part::Tool(tool) => match &tool.state {
                message::ToolState::Completed(state) => {
                    Some((tool.base.id.as_str(), state.time.compacted))
                }
                _ => None,
            },
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    assert_eq!(compacted.get("part_a_older"), Some(&None));
    assert_eq!(compacted.get("part_b_older"), Some(&None));
    assert_eq!(compacted.get("part_z_compacted"), Some(&Some(42)));

    cleanup_session(&session_id).await;
}

#[tokio::test]
// 未完成工具调用仍参与协议配对，不能作为旧输出提前裁剪。
async fn prune_skips_unfinished_tool_states() {
    let _storage_lock = COMPACTION_STORAGE_LOCK.lock().await;
    let session_id = unique_session("compaction-prune-pending");
    let mut old_assistant = assistant_message(&session_id, "msg_002", "msg_001", "", None);
    old_assistant.parts =
        vec![pending_tool(&session_id, "msg_002", "part_pending", "p".repeat(400_000))];

    for msg in [
        user_message(&session_id, "msg_001", "first"),
        old_assistant,
        user_message(&session_id, "msg_003", "second"),
        user_message(&session_id, "msg_004", "third"),
    ] {
        write_message_with_parts(&msg).await;
    }

    prune(&session_id).await.expect("prune should ignore unfinished tools");

    let parts = message::parts(&session_id, "msg_002").await.expect("old parts");
    let message::Part::Tool(tool) = &parts[0] else {
        panic!("part should be tool");
    };
    assert!(matches!(&tool.state, message::ToolState::Pending(_)));

    cleanup_session(&session_id).await;
}
