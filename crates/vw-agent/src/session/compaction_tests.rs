//! 验证会话压缩阈值、消息保留、工具配对与持久化边界。

use super::*;
use serde_json::json;
use crate::app::agent::provider::provider;
use crate::app::agent::session::message;
use std::collections::HashMap;

static COMPACTION_STORAGE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn capability_io(text: bool) -> provider::CapabilityIO {
    provider::CapabilityIO { text, audio: false, image: false, video: false, pdf: false }
}

pub(crate) fn model_with_limits(context: u64, input: Option<u64>, output: u64) -> provider::Model {
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

pub(crate) fn user_message(session_id: &str, id: &str, text: &str) -> message::WithParts {
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
    assert_eq!(stored.len(), 3);
    assert!(stored.iter().any(|message| message.info.id() == "msg_old_user"));
    assert!(stored.iter().all(|message| !matches!(&message.info, message::Info::Assistant(info) if info.summary == Some(true))));
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
async fn internal_compaction_single_summary_preserves_all_persisted_history() {
    let _storage_lock = COMPACTION_STORAGE_LOCK.lock().await;
    let session_id = unique_session("compaction-checkpoint");
    let history = vec![
        user_message(&session_id, "msg_01", &"中文😀".repeat(50_000)),
        user_message(&session_id, "msg_02", "FINAL_DECISION"),
        user_message(&session_id, "msg_03", "continue"),
    ];
    for message in &history {
        write_message_with_parts(message).await;
    }
    let before = serde_json::to_value(message::messages(&session_id, None).await.unwrap()).unwrap();
    let mut calls = 0;
    let outcome = process_with_summarizer(
        ProcessInput {
            parent_id: "msg_03".into(),
            messages: history.clone(),
            session_id: session_id.clone(),
            auto: false,
        },
        &model_with_limits(128_000, None, 4096),
        |request| {
            calls += 1;
            assert!(request.prompt.contains("FINAL_DECISION"));
            Ok("checkpoint decisions".into())
        },
    )
    .await
    .unwrap();
    assert!(matches!(outcome, CompactionOutcome::Compacted { .. }));
    assert_eq!(calls, 1);
    assert_eq!(
        serde_json::to_value(message::messages(&session_id, None).await.unwrap()).unwrap(),
        before
    );
    let model_context =
        super::super::compaction_checkpoint::restore(&session_id, &model_history(&history))
            .await
            .unwrap();
    assert!(model_context[0]["content"].as_str().unwrap().contains("checkpoint decisions"));
    assert_eq!(model_context.last().unwrap()["content"], "continue");
    cleanup_session(&session_id).await;
}

#[tokio::test]
async fn manual_compaction_checkpoint_matches_expanded_tool_history() {
    use crate::session::ui_types::{ChatMessage, ChatRole, ChatSession};
    let session_id = format!("manual-tools-{}", uuid::Uuid::new_v4());
    let snapshot = ChatSession {
        id: session_id.clone(),
        title: "tool history".into(),
        messages: [
            (ChatRole::User, "old request"),
            (ChatRole::Assistant, "analysis"),
            (ChatRole::Tool, "tool read\n{\"output\":\"essential fact\"}\n"),
            (ChatRole::Assistant, "answer"),
            (ChatRole::User, "latest request"),
        ]
        .into_iter()
        .map(|(role, content)| ChatMessage {
            role,
            content: content.into(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        })
        .collect(),
        message_ids: ["old-user", "assistant", "assistant", "assistant", "boundary"]
            .into_iter()
            .map(|id| Some(id.into()))
            .collect(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 0,
        updated_ms: 0,
    };
    let source = snapshot_checkpoint_source(&snapshot, "boundary").unwrap();
    assert_eq!(source.len(), 4);
    assert_eq!(source[2]["role"], "assistant");
    assert!(source[2]["content"].as_str().unwrap().contains("essential fact"));
    super::super::compaction_checkpoint::save(&session_id, &source, "checkpoint", None)
        .await
        .unwrap();
    let mut history = source.clone();
    history.push(json!({"role":"user","content":"latest request"}));
    let context =
        super::super::compaction_checkpoint::restore(&session_id, &history).await.unwrap();
    assert_eq!(context.len(), 2);
    assert_eq!(context[0]["content"], "[Compaction summary]\ncheckpoint");
    assert_eq!(context[1], history[4]);
    assert_eq!(snapshot.messages.len(), 5);
    assert!(snapshot_checkpoint_source(&snapshot, "missing").is_none());
    assert!(snapshot_checkpoint_source(&snapshot, "assistant").is_none());
}

#[tokio::test]
async fn codex_skills_replay_real_session_fork_preserves_sources_and_respects_cutoff() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let workspace = tempfile::tempdir().unwrap();
    let source = workspace.path().join("SKILL.md");
    std::fs::write(&source, "FORK_BODY $ARGUMENTS").unwrap();
    let record =
        crate::skills::InvokedSkillRecord::new("fork-fixture", &source, Some("preserved".into()))
            .unwrap();
    crate::project::instance::provide(workspace.path(), None, move || {
        Box::pin(async move {
            let original =
                crate::session::session::create_next(crate::session::session::CreateInput {
                    directory: crate::project::instance::directory(),
                    title: Some("skill replay fixture".into()),
                    ..Default::default()
                })
                .await
                .unwrap();
            let first = user_message(&original.id, "msg_01", "start");
            let second = assistant_message(&original.id, "msg_02", "msg_01", "skill loaded", None);
            for message in [&first, &second] {
                write_message_with_parts(message).await;
            }
            let history = model_history(&[first.clone(), second]);
            crate::session::skill_checkpoint::save(
                &original.id,
                "main",
                &history,
                &[record.clone()],
            )
            .await
            .unwrap();
            let full = crate::session::session::fork(&original.id, None).await.unwrap();
            let cut = crate::session::session::fork(&original.id, Some("msg_01")).await.unwrap();
            assert_eq!(
                crate::session::skill_checkpoint::restore(&full.id, "main", &history)
                    .await
                    .unwrap(),
                vec![record]
            );
            assert!(
                crate::session::skill_checkpoint::restore(
                    &cut.id,
                    "main",
                    &model_history(&[first])
                )
                .await
                .unwrap()
                .is_empty()
            );
            crate::session::session::remove(&full.id).await.unwrap();
            assert!(
                crate::session::skill_checkpoint::restore(&full.id, "main", &history)
                    .await
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(
                crate::session::skill_checkpoint::restore(&original.id, "main", &history)
                    .await
                    .unwrap()
                    .len(),
                1
            );
            crate::session::session::remove(&cut.id).await.unwrap();
            crate::session::session::remove(&original.id).await.unwrap();
        })
    })
    .await
    .unwrap();
}
