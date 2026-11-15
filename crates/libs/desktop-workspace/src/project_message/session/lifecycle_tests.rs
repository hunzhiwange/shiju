use super::{reset_boundary_loaded_end, steps_from_gateway_messages};
use session_ui_types::ChatRole;
use serde_json::{Map, Value, json};
use message_types as agent_message;

fn user_info(id: &str) -> agent_message::Info {
    agent_message::Info::User(Box::new(agent_message::UserInfo {
        id: id.to_string(),
        session_id: "session-1".to_string(),
        time: agent_message::UserTime { created: 5 },
        summary: None,
        agent: "default".to_string(),
        model: agent_message::ModelRef {
            provider_id: "provider-a".to_string(),
            model_id: "model-a".to_string(),
        },
        system: None,
        tools: None,
        variant: None,
    }))
}

fn assistant_info(id: &str) -> agent_message::Info {
    agent_message::Info::Assistant(Box::new(agent_message::AssistantInfo {
        id: id.to_string(),
        session_id: "session-1".to_string(),
        time: agent_message::AssistantTime {
            created: 10,
            completed: Some(20),
            processing_elapsed_ms: Some(8),
            llm_elapsed_ms: None,
        },
        error: None,
        parent_id: "user-1".to_string(),
        model_id: "model-a".to_string(),
        provider_id: "provider-a".to_string(),
        mode: "chat".to_string(),
        agent: "default".to_string(),
        path: agent_message::PathInfo { cwd: "/tmp".to_string(), root: "/tmp".to_string() },
        summary: None,
        cost: 0.0,
        tokens: agent_message::TokenInfo {
            total: None,
            input: 0,
            output: 0,
            reasoning: 0,
            cache: agent_message::TokenCacheInfo { read: 0, write: 0 },
        },
        variant: None,
        finish: None,
    }))
}

fn user_info_at(id: &str, created: u64) -> agent_message::Info {
    let mut info = user_info(id);
    if let agent_message::Info::User(user) = &mut info {
        user.time.created = created;
    }
    info
}

fn assistant_info_at(id: &str, created: u64) -> agent_message::Info {
    let mut info = assistant_info(id);
    if let agent_message::Info::Assistant(assistant) = &mut info {
        assistant.time.created = created;
        assistant.time.completed = Some(created.saturating_add(5));
    }
    info
}

fn assistant_info_at_with_parent(id: &str, parent_id: &str, created: u64) -> agent_message::Info {
    let mut info = assistant_info_at(id, created);
    if let agent_message::Info::Assistant(assistant) = &mut info {
        assistant.parent_id = parent_id.to_string();
    }
    info
}

fn assistant_info_with_usage(id: &str) -> agent_message::Info {
    let mut info = assistant_info(id);
    if let agent_message::Info::Assistant(assistant) = &mut info {
        assistant.time.created = 100;
        assistant.time.completed = Some(150);
        assistant.provider_id = "provider-a".to_string();
        assistant.model_id = "model-a".to_string();
        assistant.cost = 0.25;
        assistant.finish = Some("stop".to_string());
        assistant.tokens = agent_message::TokenInfo {
            total: Some(30),
            input: 11,
            output: 12,
            reasoning: 4,
            cache: agent_message::TokenCacheInfo { read: 2, write: 1 },
        };
    }
    info
}

fn part_base(part_id: &str, message_id: &str) -> agent_message::PartBase {
    agent_message::PartBase {
        id: part_id.to_string(),
        session_id: "session-1".to_string(),
        message_id: message_id.to_string(),
    }
}

fn tool_payload(raw: &str) -> Value {
    let (_, payload) = raw.split_once('\n').expect("tool payload should contain a newline");
    serde_json::from_str(payload.trim()).expect("tool payload should be valid json")
}

fn tool_message(id: &str, state: agent_message::ToolState) -> agent_message::WithParts {
    agent_message::WithParts {
        info: assistant_info(id),
        parts: vec![agent_message::Part::Tool(agent_message::ToolPart {
            base: part_base(&format!("part-{id}"), id),
            call_id: format!("call-{id}"),
            tool: "bash".to_string(),
            state,
            metadata: None,
        })],
    }
}

fn text_message(info: agent_message::Info, text: &str) -> agent_message::WithParts {
    let message_id = info.id().to_string();
    agent_message::WithParts {
        info,
        parts: vec![agent_message::Part::Text(agent_message::TextPart {
            base: part_base(&format!("part-{message_id}"), &message_id),
            text: text.to_string(),
            synthetic: None,
            ignored: None,
            time: None,
            metadata: None,
        })],
    }
}

#[test]
fn loaded_chat_from_gateway_messages_orders_newest_first_gateway_results_chronologically() {
    let messages = vec![
        text_message(assistant_info_at_with_parent("assistant-2", "user-2", 40), "second answer"),
        text_message(user_info_at("user-2", 30), "follow up"),
        text_message(assistant_info_at_with_parent("assistant-1", "user-1", 20), "first answer"),
        text_message(user_info_at("user-1", 10), "first question"),
    ];

    let (chat, message_ids) = loaded_chat_from_gateway_messages(messages);

    let contents = chat.iter().map(|message| message.content.as_str()).collect::<Vec<_>>();
    assert_eq!(contents, vec!["first question", "first answer", "follow up", "second answer"]);
    assert_eq!(
        message_ids,
        vec![
            Some("user-1".to_string()),
            Some("assistant-1".to_string()),
            Some("user-2".to_string()),
            Some("assistant-2".to_string()),
        ]
    );
    assert!(chat.iter().all(|message| message.model.as_deref() == Some("provider-a/model-a")));
}

#[test]
fn loaded_chat_orders_parent_user_before_assistant_when_timestamps_conflict() {
    let messages = vec![
        text_message(assistant_info_at_with_parent("assistant-2", "user-2", 600), "second answer"),
        text_message(user_info_at("user-2", 600), "follow up"),
        text_message(assistant_info_at_with_parent("assistant-1", "user-1", 100), "first answer"),
        text_message(user_info_at("user-1", 500), "first question"),
    ];

    let (chat, message_ids) = loaded_chat_from_gateway_messages(messages);

    assert_eq!(
        chat.iter().map(|message| message.content.as_str()).collect::<Vec<_>>(),
        vec!["first question", "first answer", "follow up", "second answer"]
    );
    assert_eq!(
        message_ids,
        vec![
            Some("user-1".to_string()),
            Some("assistant-1".to_string()),
            Some("user-2".to_string()),
            Some("assistant-2".to_string()),
        ]
    );
}

#[test]
fn loaded_chat_from_gateway_messages_splits_tool_parts_into_tool_rows() {
    let mut metadata = Map::new();
    metadata.insert("summary".to_string(), Value::String("2 次匹配".to_string()));

    let messages = vec![agent_message::WithParts {
        info: assistant_info("assistant-1"),
        parts: vec![
            agent_message::Part::Reasoning(agent_message::ReasoningPart {
                base: part_base("part-r1", "assistant-1"),
                text: "先缩小范围".to_string(),
                metadata: None,
                time: agent_message::PartTime { start: 100, end: Some(140) },
            }),
            agent_message::Part::Text(agent_message::TextPart {
                base: part_base("part-t1", "assistant-1"),
                text: "先看搜索结果。".to_string(),
                synthetic: None,
                ignored: None,
                time: None,
                metadata: None,
            }),
            agent_message::Part::Tool(agent_message::ToolPart {
                base: part_base("part-tool-1", "assistant-1"),
                call_id: "call-1".to_string(),
                tool: "grep".to_string(),
                state: agent_message::ToolState::Completed(agent_message::ToolStateCompleted {
                    input: Map::from_iter([(
                        "pattern".to_string(),
                        Value::String("foo".to_string()),
                    )]),
                    output: "2 matches".to_string(),
                    title: "搜索 foo".to_string(),
                    metadata,
                    time: agent_message::ToolStateCompletedTime {
                        start: 150,
                        end: 180,
                        compacted: None,
                    },
                    attachments: None,
                }),
                metadata: None,
            }),
            agent_message::Part::Text(agent_message::TextPart {
                base: part_base("part-t2", "assistant-1"),
                text: "最终答案。".to_string(),
                synthetic: None,
                ignored: None,
                time: None,
                metadata: None,
            }),
        ],
    }];

    let (chat, message_ids) = loaded_chat_from_gateway_messages(messages);

    assert_eq!(chat.len(), 3);
    assert_eq!(chat[0].role, ChatRole::Assistant);
    assert_eq!(chat[0].content, "<think>先缩小范围</think>先看搜索结果。");
    assert_eq!(chat[0].think_timing.len(), 1);
    assert_eq!(chat[0].think_timing[0].start_ms, 100);
    assert_eq!(chat[0].think_timing[0].end_ms, Some(140));

    assert_eq!(chat[1].role, ChatRole::Tool);
    assert_eq!(chat[1].model, None);
    let payload = tool_payload(&chat[1].content);
    assert_eq!(payload.get("status"), Some(&json!("completed")));
    assert_eq!(payload.get("output"), Some(&json!("2 matches")));
    assert_eq!(payload.get("callID"), Some(&json!("call-1")));
    assert_eq!(payload.get("summary"), Some(&json!("2 次匹配")));

    assert_eq!(chat[2].role, ChatRole::Assistant);
    assert_eq!(chat[2].content, "最终答案。");
    assert_eq!(
        message_ids,
        vec![
            Some("assistant-1".to_string()),
            Some("assistant-1".to_string()),
            Some("assistant-1".to_string()),
        ]
    );
}

#[test]
fn compaction_status_history_restores_completed_without_summary_text() {
    let messages = vec![agent_message::WithParts {
        info: assistant_info("assistant-compaction"),
        parts: vec![
            agent_message::Part::Compaction(agent_message::CompactionPart {
                base: part_base("part-compaction", "assistant-compaction"),
                auto: true,
            }),
            agent_message::Part::Text(agent_message::TextPart {
                base: part_base("part-summary", "assistant-compaction"),
                text: "private compacted summary".to_string(),
                synthetic: Some(true),
                ignored: None,
                time: None,
                metadata: None,
            }),
        ],
    }];

    let (chat, message_ids) = loaded_chat_from_gateway_messages(messages);

    assert_eq!(chat.len(), 1);
    assert_eq!(chat[0].role, ChatRole::System);
    assert_eq!(chat[0].content, "压缩完成");
    assert_eq!(message_ids, vec![Some("assistant-compaction".to_string())]);
}

#[test]
fn processing_timing_history_split_attaches_assistant_time_to_first_visible_row_only() {
    let messages = vec![agent_message::WithParts {
        info: assistant_info("assistant-timed"),
        parts: vec![
            agent_message::Part::Tool(agent_message::ToolPart {
                base: part_base("part-tool-timed", "assistant-timed"),
                call_id: "call-timed".to_string(),
                tool: "shell".to_string(),
                state: agent_message::ToolState::Completed(agent_message::ToolStateCompleted {
                    input: Map::new(),
                    output: "done".to_string(),
                    title: "run".to_string(),
                    metadata: Map::new(),
                    time: agent_message::ToolStateCompletedTime {
                        start: 12,
                        end: 18,
                        compacted: None,
                    },
                    attachments: None,
                }),
                metadata: None,
            }),
            agent_message::Part::Text(agent_message::TextPart {
                base: part_base("part-text-timed", "assistant-timed"),
                text: "answer".to_string(),
                synthetic: None,
                ignored: None,
                time: None,
                metadata: None,
            }),
        ],
    }];

    let (chat, _) = loaded_chat_from_gateway_messages(messages);

    assert_eq!(chat.len(), 2);
    assert_eq!(
        chat[0].processing_timing,
        Some(session_ui_types::ProcessingTiming {
            started_ms: 10,
            completed_ms: Some(20),
            processing_elapsed_ms: Some(8),
            llm_elapsed_ms: None,
        })
    );
    assert_eq!(chat[1].processing_timing, None);

    let mut untimed_info = assistant_info("assistant-untimed");
    if let agent_message::Info::Assistant(info) = &mut untimed_info {
        info.time.completed = None;
    }
    let (untimed_chat, _) =
        loaded_chat_from_gateway_messages(vec![text_message(untimed_info, "legacy answer")]);
    assert_eq!(untimed_chat.len(), 1);
    assert_eq!(untimed_chat[0].processing_timing, None);
}

#[test]
fn loaded_chat_from_gateway_messages_skips_empty_assistant_shell_for_tool_only_messages() {
    let messages = vec![agent_message::WithParts {
        info: assistant_info("assistant-2"),
        parts: vec![agent_message::Part::Tool(agent_message::ToolPart {
            base: part_base("part-tool-2", "assistant-2"),
            call_id: "call-2".to_string(),
            tool: "question".to_string(),
            state: agent_message::ToolState::Running(agent_message::ToolStateRunning {
                input: Map::from_iter([(
                    "questions".to_string(),
                    json!([{"header": "确认", "question": "继续吗？"}]),
                )]),
                title: Some("等待确认".to_string()),
                metadata: None,
                time: agent_message::PartTime { start: 200, end: None },
            }),
            metadata: None,
        })],
    }];

    let (chat, message_ids) = loaded_chat_from_gateway_messages(messages);

    assert_eq!(chat.len(), 1);
    assert_eq!(chat[0].role, ChatRole::Tool);
    let payload = tool_payload(&chat[0].content);
    assert_eq!(payload.get("status"), Some(&json!("running")));
    assert_eq!(payload.get("callID"), Some(&json!("call-2")));
    assert_eq!(message_ids, vec![Some("assistant-2".to_string())]);
}

#[test]
fn loaded_chat_from_gateway_messages_keeps_user_text_file_and_message_ids() {
    let messages = vec![agent_message::WithParts {
        info: user_info("user-1"),
        parts: vec![
            agent_message::Part::Text(agent_message::TextPart {
                base: part_base("part-u1", "user-1"),
                text: "看这个文件".to_string(),
                synthetic: None,
                ignored: None,
                time: None,
                metadata: None,
            }),
            agent_message::Part::File(agent_message::FilePart {
                base: part_base("part-f1", "user-1"),
                url: "file:///tmp/a.rs".to_string(),
                mime: "text/rust".to_string(),
                filename: Some("a.rs".to_string()),
                source: None,
            }),
        ],
    }];

    let (chat, ids) = loaded_chat_from_gateway_messages(messages);

    assert_eq!(chat.len(), 1);
    assert_eq!(chat[0].role, ChatRole::User);
    assert!(chat[0].content.contains("看这个文件"));
    assert!(chat[0].content.contains("[File: file:///tmp/a.rs]"));
    assert_eq!(ids, vec![Some("user-1".to_string())]);
}

#[test]
fn loaded_chat_from_gateway_messages_covers_pending_running_completed_and_error_payloads() {
    let mut running_metadata = Map::new();
    running_metadata.insert("permissionRequest".to_string(), json!({"id": "perm-1"}));
    running_metadata.insert("render_hint".to_string(), json!("compact"));

    let mut completed_metadata = Map::new();
    completed_metadata.insert("toolCallId".to_string(), json!("explicit-call"));

    let messages = vec![
        tool_message(
            "pending",
            agent_message::ToolState::Pending(agent_message::ToolStatePending {
                input: Map::from_iter([("arg".to_string(), json!("value"))]),
                raw: " raw input ".to_string(),
            }),
        ),
        tool_message(
            "running",
            agent_message::ToolState::Running(agent_message::ToolStateRunning {
                input: Map::new(),
                title: Some("Running".to_string()),
                metadata: Some(running_metadata),
                time: agent_message::PartTime { start: 1, end: Some(2) },
            }),
        ),
        tool_message(
            "completed",
            agent_message::ToolState::Completed(agent_message::ToolStateCompleted {
                input: Map::from_iter([("path".to_string(), json!("/tmp/a"))]),
                output: "done".to_string(),
                title: "Done".to_string(),
                metadata: completed_metadata,
                time: agent_message::ToolStateCompletedTime {
                    start: 3,
                    end: 4,
                    compacted: Some(5),
                },
                attachments: Some(vec![agent_message::ToolAttachment {
                    url: "file:///tmp/out.txt".to_string(),
                    mime: "text/plain".to_string(),
                }]),
            }),
        ),
        tool_message(
            "denied",
            agent_message::ToolState::Error(agent_message::ToolStateError {
                input: Map::new(),
                error: "Approval required".to_string(),
                metadata: None,
                time: agent_message::PartTime { start: 6, end: None },
            }),
        ),
        tool_message(
            "error",
            agent_message::ToolState::Error(agent_message::ToolStateError {
                input: Map::from_iter([("cmd".to_string(), json!("false"))]),
                error: "process failed".to_string(),
                metadata: Some(Map::from_iter([("summary".to_string(), json!("失败"))])),
                time: agent_message::PartTime { start: 7, end: Some(8) },
            }),
        ),
    ];

    let (chat, ids) = loaded_chat_from_gateway_messages(messages);

    assert_eq!(chat.len(), 5);
    assert_eq!(ids.len(), 5);

    let pending = tool_payload(&chat[0].content);
    assert_eq!(pending.get("status"), Some(&json!("pending")));
    assert_eq!(pending.get("input"), Some(&json!(" raw input ")));

    let running = tool_payload(&chat[1].content);
    assert_eq!(running.get("status"), Some(&json!("running")));
    assert_eq!(running.get("title"), Some(&json!("Running")));
    assert_eq!(running.get("permission_request"), Some(&json!({"id": "perm-1"})));
    assert_eq!(running.get("renderHint"), Some(&json!("compact")));

    let completed = tool_payload(&chat[2].content);
    assert_eq!(completed.get("status"), Some(&json!("completed")));
    assert_eq!(completed.get("toolCallId"), Some(&json!("call-completed")));
    assert_eq!(completed.pointer("/metadata/toolCallId"), Some(&json!("explicit-call")));
    assert!(completed.get("attachments").is_some());

    let denied = tool_payload(&chat[3].content);
    assert_eq!(denied.get("status"), Some(&json!("denied")));

    let error = tool_payload(&chat[4].content);
    assert_eq!(error.get("status"), Some(&json!("error")));
    assert_eq!(error.get("summary"), Some(&json!("失败")));
}

#[test]
fn steps_from_gateway_messages_restores_step_parts_with_assistant_usage() {
    let messages = vec![agent_message::WithParts {
        info: assistant_info_with_usage("assistant-step"),
        parts: vec![
            agent_message::Part::StepStart(agent_message::StepStartPart {
                base: part_base("part-step-start", "assistant-step"),
                snapshot: Some("start.snap".to_string()),
            }),
            agent_message::Part::StepFinish(agent_message::StepFinishPart {
                base: part_base("part-step-finish", "assistant-step"),
                reason: "tool_round".to_string(),
                snapshot: Some("finish.snap".to_string()),
                cost: 0.0,
                tokens: agent_message::TokenInfo {
                    total: Some(0),
                    input: 0,
                    output: 0,
                    reasoning: 0,
                    cache: agent_message::TokenCacheInfo { read: 0, write: 0 },
                },
            }),
        ],
    }];

    let steps = steps_from_gateway_messages(&messages);

    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].index, 1);
    assert_eq!(steps[0].started_ms, 100);
    assert_eq!(steps[0].finished_ms, Some(150));
    assert_eq!(steps[0].start_snapshot_path.as_deref(), Some("start.snap"));
    assert_eq!(steps[0].finish_snapshot_path.as_deref(), Some("finish.snap"));
    assert_eq!(steps[0].usage.input_tokens, 11);
    assert_eq!(steps[0].usage.cached_tokens, 3);
    assert_eq!(steps[0].finish_reason.as_deref(), Some("stop"));
    assert_eq!(steps[0].model.as_deref(), Some("provider-a/model-a"));
}

#[test]
fn steps_from_gateway_messages_falls_back_to_assistant_token_metadata() {
    let messages = vec![agent_message::WithParts {
        info: assistant_info_with_usage("assistant-usage"),
        parts: vec![agent_message::Part::Text(agent_message::TextPart {
            base: part_base("part-text", "assistant-usage"),
            text: "answer".to_string(),
            synthetic: None,
            ignored: None,
            time: None,
            metadata: None,
        })],
    }];

    let steps = steps_from_gateway_messages(&messages);

    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].usage.output_tokens, 12);
    assert_eq!(steps[0].finish_reason.as_deref(), Some("stop"));
}

#[test]
fn steps_from_gateway_messages_restores_orphan_finish_with_part_metadata() {
    let mut info = assistant_info("assistant-orphan-finish");
    if let agent_message::Info::Assistant(assistant) = &mut info {
        assistant.provider_id.clear();
        assistant.model_id = "model-only".to_string();
    }
    let messages = vec![agent_message::WithParts {
        info,
        parts: vec![agent_message::Part::StepFinish(agent_message::StepFinishPart {
            base: part_base("part-orphan-finish", "assistant-orphan-finish"),
            reason: "completed_without_start".to_string(),
            snapshot: Some("finish-only.snap".to_string()),
            cost: 0.75,
            tokens: agent_message::TokenInfo {
                total: Some(9),
                input: 3,
                output: 4,
                reasoning: 2,
                cache: agent_message::TokenCacheInfo { read: 1, write: 1 },
            },
        })],
    }];

    let steps = steps_from_gateway_messages(&messages);

    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].started_ms, 20);
    assert_eq!(steps[0].finished_ms, Some(20));
    assert_eq!(steps[0].usage.input_tokens, 3);
    assert_eq!(steps[0].usage.cached_tokens, 2);
    assert_eq!(steps[0].finish_reason.as_deref(), Some("completed_without_start"));
    assert_eq!(steps[0].cost_usd, Some(0.75));
    assert_eq!(steps[0].model.as_deref(), Some("model-only"));
    assert_eq!(steps[0].finish_snapshot_path.as_deref(), Some("finish-only.snap"));
}

#[test]
fn steps_from_gateway_messages_ignores_messages_without_step_parts_or_usage() {
    let messages = vec![
        text_message(user_info("user-no-usage"), "question"),
        text_message(assistant_info("assistant-no-usage"), "answer"),
    ];

    assert!(steps_from_gateway_messages(&messages).is_empty());
}

#[test]
fn loaded_chat_from_gateway_messages_uses_input_fallback_and_permission_metadata() {
    let pending = tool_message(
        "pending-fallback",
        agent_message::ToolState::Pending(agent_message::ToolStatePending {
            input: Map::from_iter([("path".to_string(), json!("/tmp/input"))]),
            raw: "   ".to_string(),
        }),
    );
    let denied = tool_message(
        "metadata-denied",
        agent_message::ToolState::Error(agent_message::ToolStateError {
            input: Map::new(),
            error: "request stopped".to_string(),
            metadata: Some(Map::from_iter([(
                "permission_request".to_string(),
                json!({"id": "permission-2"}),
            )])),
            time: agent_message::PartTime { start: 9, end: Some(10) },
        }),
    );

    let (chat, _) = loaded_chat_from_gateway_messages(vec![pending, denied]);

    let pending_payload = tool_payload(&chat[0].content);
    assert_eq!(pending_payload.get("input"), Some(&json!(r#"{"path":"/tmp/input"}"#)));
    let denied_payload = tool_payload(&chat[1].content);
    assert_eq!(denied_payload.get("status"), Some(&json!("denied")));
    assert_eq!(denied_payload.get("permission_request"), Some(&json!({"id": "permission-2"})));
}

#[test]
fn reset_boundary_loaded_end_prefers_last_known_id_and_falls_back_to_bounded_length() {
    let loaded_ids = vec![
        Some("message-1".to_string()),
        Some("message-2".to_string()),
        Some("message-3".to_string()),
    ];

    assert_eq!(
        reset_boundary_loaded_end(
            &loaded_ids,
            &[Some("missing".to_string()), Some("message-2".to_string())],
        ),
        2
    );
    assert_eq!(
        reset_boundary_loaded_end(
            &loaded_ids,
            &[Some("missing-1".to_string()), Some("missing-2".to_string())],
        ),
        2
    );
    assert_eq!(reset_boundary_loaded_end(&loaded_ids[..1], &[None, None, None]), 1);
}


fn loaded_chat_from_gateway_messages(
    messages: Vec<agent_message::WithParts>,
) -> (Vec<session_ui_types::ChatMessage>, Vec<Option<String>>) {
    super::loaded_chat_from_gateway_messages(messages, "压缩完成")
}
