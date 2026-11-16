//! 历史裁剪的摘要、最小阈值和已压缩工具停止边界。

use crate::{compaction::prune, message};
use serde_json::Map;
use std::collections::HashMap;

// 路径和存储均有进程级缓存：子进程在首次访问之前设置临时根目录。
// 不修改并行测试所在进程的环境，也不替换被测存储或裁剪算法。
fn run_in_temporary_storage(test: &str) -> bool {
    const CHILD_TEST: &str = "SHIJU_HOST_SPLIT_8_CHILD_TEST";
    if std::env::var(CHILD_TEST).as_deref() == Ok(test) {
        return false;
    }
    let root = tempfile::tempdir().expect("temporary storage root");
    let status = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .arg(format!("{}::{test}", module_path!().split_once("::").expect("crate module").1))
        .args(["--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_TEST, test)
        .env("VIBEWINDOW_TEST_ROOT", root.path())
        .env("VIBEWINDOW_TEST_HOME", root.path())
        .status()
        .expect("isolated test process");
    assert!(status.success(), "isolated test failed: {test}: {status}");
    true
}

fn unique_session(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4())
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

async fn write_message_with_parts(message: &message::WithParts) {
    let message_id = message.info.id().to_string();
    let session_id = match &message.info {
        message::Info::User(info) => info.session_id.as_str(),
        message::Info::Assistant(info) => info.session_id.as_str(),
    };
    if let Ok(existing_parts) = message::parts(session_id, &message_id).await {
        for part in existing_parts {
            let _ = agent_storage::remove(&["part", &message_id, part.id()]).await;
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
                        agent_storage::remove(&["part", &message_id, part.id()]).await;
                    let _ = agent_storage::remove(&[
                        "part",
                        session_id,
                        &message_id,
                        part.id(),
                    ])
                    .await;
                }
            }
            let _ = agent_storage::remove(&["message", session_id, &message_id]).await;
        }
    }
}

#[tokio::test]
async fn prune_stops_at_summary_messages() {
    if run_in_temporary_storage("prune_stops_at_summary_messages") {
        return;
    }
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
    if run_in_temporary_storage("prune_keeps_candidate_when_prunable_tokens_equal_minimum") {
        return;
    }
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
    if run_in_temporary_storage("prune_stops_at_previously_compacted_tool") {
        return;
    }
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
