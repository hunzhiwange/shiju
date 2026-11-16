//! 验证会话压缩阈值、消息保留、工具配对与持久化边界。

use crate::compaction::{ProcessInput,CompactionOutcome,now_ms};
use model_types as provider;
use crate::message;
use std::collections::HashMap;


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

pub async fn process_returns_stop_when_parent_user_message_is_missing<H: CompactionBoundaryHost>() {
    let result = H::process(
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

pub async fn process_does_not_compact_when_the_boundary_has_no_prior_history<H: CompactionBoundaryHost>() {
    let session_id = unique_session("compaction-process-auto");
    let parent_id = "msg_parent".to_string();
    let input_messages = vec![
        user_message(&session_id, &parent_id, "initial question"),
        assistant_message(&session_id, "msg_answer", &parent_id, "initial answer", None),
    ];

    let result = H::process(
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

pub async fn process_does_not_synthesize_a_summary_for_blank_history<H: CompactionBoundaryHost>() {
    let session_id = unique_session("compaction-process-manual");
    let parent_id = "msg_parent".to_string();

    let result = H::process(
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

/// Inject the real host process entrance; fixtures and expectations remain domain-owned.
pub trait CompactionBoundaryHost {
    fn process(input: ProcessInput, model: &provider::Model) -> impl Future<Output=Result<CompactionOutcome,session_info::Error>> + Send;
}
