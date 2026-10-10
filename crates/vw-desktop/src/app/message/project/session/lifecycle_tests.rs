//! 覆盖项目会话生命周期逻辑，验证打开、加载和重置流程。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use super::lifecycle::{
    loaded_chat_from_gateway_messages, reset_boundary_loaded_end, steps_from_gateway_messages,
};
use crate::app::models::{ChatRenderCacheEntry, ChatRole};
use serde_json::{Map, Value, json};
use vw_shared::message::types as agent_message;

fn app() -> crate::app::App {
    crate::app::App::new().0
}

#[test]
fn stale_stream_ui_phase_is_ignored() {
    let mut app = app();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.chat = vec![crate::app::models::ChatMessage {
        role: ChatRole::Assistant,
        content: "current tail".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    app.chat_render_cache.insert(
        0,
        ChatRenderCacheEntry {
            content_hash: 77,
            display_text: "kept".to_string(),
            ..ChatRenderCacheEntry::default()
        },
    );
    let phase = crate::app::session::prepare_chat_ui_message_phase(&app.chat[0], 0, false);
    app.get_session_runtime_mut(&session_id).stream_tail_state.advance_ui_generation();

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionUiPrepared {
            session_id,
            stream_generation: Some(0),
            phase,
        },
    );

    assert!(task.is_some());
    assert_eq!(app.chat_render_cache[&0].content_hash, 77);
    assert_eq!(app.chat_render_cache[&0].display_text, "kept");
}

fn session_info(id: &str, title: &str, directory: &str) -> vw_shared::session::info::Info {
    vw_shared::session::info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "1".to_string(),
        time: vw_shared::session::info::TimeInfo {
            created: 1,
            updated: 1,
            compacting: None,
            archived: None,
        },
        permission: None,
        revert: None,
    }
}

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
fn session_messages_loaded_preserves_parent_order_after_completion_refresh() {
    let mut app = app();
    app.active_session_id = Some("session-1".to_string());
    let messages = vec![
        text_message(assistant_info_at_with_parent("assistant-1", "user-1", 1_000), "answer"),
        text_message(user_info_at("user-1", 9_000), "question"),
    ];

    let _ = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionMessagesLoaded(Ok((
            "session-1".to_string(),
            messages,
            crate::app::models::TokenUsage::default(),
        ))),
    );

    assert_eq!(
        app.chat.iter().map(|message| message.content.as_str()).collect::<Vec<_>>(),
        vec!["question", "answer"]
    );
    assert_eq!(
        app.chat_message_ids,
        vec![Some("user-1".to_string()), Some("assistant-1".to_string())]
    );
}

#[test]
fn session_messages_loaded_same_content_preserves_scroll_and_ui_cache() {
    let mut app = app();
    app.active_session_id = Some("session-1".to_string());
    let messages = vec![
        text_message(assistant_info_with_usage("assistant-1"), "answer"),
        text_message(user_info_at("user-1", 900), "question"),
    ];
    let (live_chat, _) = loaded_chat_from_gateway_messages(messages.clone());
    app.chat = live_chat;
    app.chat_message_ids = vec![None; app.chat.len()];
    app.chat_virtualizer.sync_len(app.chat.len(), 120.0);
    app.chat_virtualizer.set_viewport_height(100.0);
    app.chat_virtualizer.focus_turn_on_user(0);
    app.active_session_view_state.base_ready = true;
    app.active_session_view_state.updated_ms = 77;
    app.active_session_view_state.calls.push(json!({ "id": "call-live" }));
    app.active_session_view_state.message_meta_texts =
        vec![Some("stale".to_string()); app.chat.len()];
    app.chat_render_cache.insert(0, Default::default());
    let previous_scroll_id = app.chat_scroll_id.clone();
    let usage = crate::app::models::TokenUsage {
        input_tokens: 2,
        output_tokens: 3,
        cached_tokens: 4,
        reasoning_tokens: 5,
    };

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionMessagesLoaded(Ok((
            "session-1".to_string(),
            messages,
            usage,
        ))),
    );

    assert!(task.is_some());
    assert_eq!(app.chat_scroll_id, previous_scroll_id);
    assert_eq!(app.chat_virtualizer.turn_focus_user_index(), Some(0));
    assert!(app.active_session_view_state.base_ready);
    assert!(!app.active_session_view_state.ui_preparing);
    assert_eq!(app.active_session_view_state.updated_ms, 77);
    assert_eq!(app.active_session_view_state.calls, vec![json!({ "id": "call-live" })]);
    assert!(app.chat_render_cache.contains_key(&0));
    assert_eq!(
        app.chat_message_ids,
        vec![Some("user-1".to_string()), Some("assistant-1".to_string())]
    );
    assert_eq!(app.usage.input_tokens, 2);
    assert_eq!(app.usage.output_tokens, 3);
    assert_eq!(app.usage.cached_tokens, 4);
    assert_eq!(app.usage.reasoning_tokens, 5);
    assert_eq!(app.active_session_view_state.steps.len(), 1);
    assert_eq!(app.active_session_view_state.steps[0].usage.input_tokens, 11);
    assert_eq!(app.active_session_view_state.steps[0].usage.output_tokens, 12);
    assert_eq!(app.active_session_view_state.steps[0].usage.cached_tokens, 3);
    assert_eq!(app.active_session_view_state.steps[0].usage.reasoning_tokens, 4);
    assert_eq!(app.active_session_view_state.message_meta_texts.len(), app.chat.len());
    assert!(
        app.active_session_view_state
            .message_meta_texts
            .iter()
            .all(|meta| meta.as_deref() != Some("stale"))
    );
}

#[test]
fn session_messages_loaded_changed_content_replaces_history_and_invalidates_ui_cache() {
    for (stale_role, stale_content) in
        [(ChatRole::Assistant, "stale answer"), (ChatRole::User, "answer")]
    {
        let mut app = app();
        app.active_session_id = Some("session-1".to_string());
        let messages = vec![
            text_message(assistant_info_at_with_parent("assistant-1", "user-1", 1_000), "answer"),
            text_message(user_info_at("user-1", 900), "question"),
        ];
        let (live_chat, _) = loaded_chat_from_gateway_messages(messages.clone());
        app.chat = live_chat;
        app.chat[1].role = stale_role;
        app.chat[1].content = stale_content.to_string();
        app.chat_message_ids = vec![None; app.chat.len()];
        app.active_session_view_state.base_ready = true;
        app.chat_render_cache.insert(0, Default::default());
        let previous_scroll_id = app.chat_scroll_id.clone();

        let task = super::lifecycle::handle(
            &mut app,
            crate::app::message::project::ProjectMessage::SessionMessagesLoaded(Ok((
                "session-1".to_string(),
                messages,
                crate::app::models::TokenUsage::default(),
            ))),
        );

        assert!(task.is_some());
        assert_ne!(app.chat_scroll_id, previous_scroll_id);
        assert_eq!(app.chat[1].role, ChatRole::Assistant);
        assert_eq!(app.chat[1].content, "answer");
        assert!(app.chat_render_cache.is_empty());
        assert!(!app.active_session_view_state.base_ready);
        assert!(app.active_session_view_state.ui_preparing);
        assert_eq!(
            app.chat_message_ids,
            vec![Some("user-1".to_string()), Some("assistant-1".to_string())]
        );
    }
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
        Some(vw_shared::session::ui_types::ProcessingTiming {
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
fn session_context_menu_and_rename_state_are_updated_synchronously() {
    let mut app = app();
    app.sessions = vec![session_info("s1", "First", "/tmp/project")];
    app.project_sessions
        .insert("/tmp/other".to_string(), vec![session_info("s2", "Second", "/tmp/other")]);

    let _ = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionRightClicked(
            "s1".to_string(),
            10.0,
            20.0,
        ),
    );
    assert_eq!(app.session_menu_id.as_deref(), Some("s1"));
    assert_eq!(app.session_menu_anchor.expect("anchor").x, 10.0);

    let _ = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionMenuClose,
    );
    assert!(app.session_menu_id.is_none());
    assert!(app.session_menu_anchor.is_none());

    let _ = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionRenamePressed("s2".to_string()),
    );
    assert_eq!(app.session_rename_id.as_deref(), Some("s2"));
    assert_eq!(app.session_rename_value, "Second");

    let _ = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionRenameChanged(" Renamed ".to_string()),
    );
    assert_eq!(app.session_rename_value, " Renamed ");

    let _ = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionRenameCancel,
    );
    assert!(app.session_rename_id.is_none());
    assert!(app.session_rename_value.is_empty());
}

#[test]
fn session_tree_toggle_updates_non_persistent_expansion_state() {
    let mut app = app();
    assert!(app.session_tree_expanded.is_empty());

    let first = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionTreeToggled("parent".to_string()),
    );
    assert!(first.is_some());
    assert!(app.session_tree_expanded.contains("parent"));

    let second = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionTreeToggled("parent".to_string()),
    );
    assert!(second.is_some());
    assert!(!app.session_tree_expanded.contains("parent"));
}

#[test]
fn session_rename_save_ignores_missing_or_empty_title_and_updates_local_titles() {
    let mut app = app();
    app.sessions = vec![session_info("s1", "First", "/tmp/project")];
    app.project_sessions
        .insert("/tmp/project".to_string(), vec![session_info("s1", "First", "/tmp/project")]);

    let _ = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionRenameSave,
    );
    assert_eq!(app.sessions[0].title, "First");

    app.session_rename_id = Some("s1".to_string());
    app.session_rename_value = "   ".to_string();
    let _ = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionRenameSave,
    );
    assert_eq!(app.sessions[0].title, "First");
    assert_eq!(app.session_rename_id.as_deref(), Some("s1"));

    app.session_rename_value = "Updated".to_string();
    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionRenameSave,
    );

    assert!(task.is_some());
    assert!(app.session_rename_id.is_none());
    assert!(app.session_rename_value.is_empty());
    assert_eq!(app.sessions[0].title, "Updated");
    assert_eq!(app.project_sessions.get("/tmp/project").unwrap()[0].title, "Updated");
}

#[test]
fn session_created_inserts_unique_session_and_resets_active_chat_state() {
    let mut app = app();
    let info = session_info("s1", "First", "/tmp/project");
    app.new_session_creating = true;
    app.project_path = Some("/tmp/project".to_string());
    app.sessions = vec![info.clone()];
    app.chat.push(crate::app::models::ChatMessage {
        role: ChatRole::User,
        content: "old".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    app.chat_message_ids = vec![Some("old".to_string())];

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionCreated(info.clone()),
    );

    assert!(task.is_some());
    assert_eq!(app.new_session_last_directory.as_deref(), Some("/tmp/project"));
    assert_eq!(app.sessions.iter().filter(|s| s.id == "s1").count(), 1);
    assert_eq!(app.project_sessions.get("/tmp/project").unwrap()[0].id, "s1");
    assert_eq!(app.active_session_id.as_deref(), Some("s1"));
    assert!(app.chat.is_empty());
    assert!(app.chat_message_ids.is_empty());
    assert_eq!(app.usage, crate::app::models::TokenUsage::default());
    assert!(!app.new_session_creating);
}

#[test]
fn session_created_without_project_path_uses_session_directory_bucket() {
    let mut app = app();
    let info = session_info("s-dir", "Directory Session", "/tmp/session-dir");

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionCreated(info),
    );

    assert!(task.is_some());
    assert_eq!(app.active_session_id.as_deref(), Some("s-dir"));
    assert_eq!(
        app.project_sessions.get("/tmp/session-dir").expect("directory bucket")[0].id,
        "s-dir"
    );
}

#[test]
fn discovered_child_session_does_not_replace_active_parent() {
    let mut app = app();
    let parent = session_info("parent", "Parent", "/tmp/project");
    let mut child = session_info("child", "Child", "/tmp/project");
    child.parent_id = Some(parent.id.clone());
    app.project_path = Some("/tmp/project".to_string());
    app.active_session_id = Some(parent.id.clone());
    app.sessions = vec![parent];
    app.chat.push(crate::app::models::ChatMessage {
        role: ChatRole::User,
        content: "parent chat".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    app.chat_message_ids = vec![Some("parent-message".to_string())];

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionCreated(child),
    );

    assert!(task.is_some());
    assert_eq!(app.active_session_id.as_deref(), Some("parent"));
    assert_eq!(app.chat[0].content, "parent chat");
    assert_eq!(app.chat_message_ids, vec![Some("parent-message".to_string())]);
    assert!(app.sessions.iter().any(|session| session.id == "child"));
    assert!(
        app.project_sessions
            .get("/tmp/project")
            .expect("project session bucket")
            .iter()
            .any(|session| session.id == "child")
    );
}

#[test]
fn session_messages_loaded_keeps_reset_local_history_when_gateway_returns_stale_tail() {
    let mut app = app();
    app.active_session_id = Some("session-1".to_string());
    app.sessions = vec![session_info("session-1", "First", "/tmp/project")];
    app.chat = vec![
        crate::app::models::ChatMessage {
            role: ChatRole::User,
            content: "keep".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        crate::app::models::ChatMessage {
            role: ChatRole::User,
            content: "new question".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        crate::app::models::ChatMessage {
            role: ChatRole::Assistant,
            content: "new answer".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];
    app.chat_message_ids = vec![
        Some("user-keep".to_string()),
        Some("user-new".to_string()),
        Some("assistant-new".to_string()),
    ];
    app.store_session_chat_snapshot(
        "session-1".to_string(),
        crate::app::session::shared_chat_messages(app.chat.clone()),
        app.chat_message_ids.clone(),
    );
    app.mark_session_history_reset_boundary(
        "session-1".to_string(),
        vec![Some("user-keep".to_string())],
    );

    let stale_messages = vec![
        text_message(user_info("user-keep"), "keep"),
        text_message(assistant_info("assistant-old"), "old answer"),
        text_message(user_info("user-old"), "old follow-up"),
        text_message(user_info("user-new"), "new question"),
        text_message(assistant_info("assistant-new"), "new answer"),
    ];

    let _ = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionMessagesLoaded(Ok((
            "session-1".to_string(),
            stale_messages,
            crate::app::models::TokenUsage::default(),
        ))),
    );

    let contents = app.chat.iter().map(|message| message.content.as_str()).collect::<Vec<_>>();
    assert_eq!(contents, vec!["keep", "new question", "new answer"]);
    assert_eq!(
        app.chat_message_ids,
        vec![
            Some("user-keep".to_string()),
            Some("user-new".to_string()),
            Some("assistant-new".to_string()),
        ]
    );
}

#[test]
fn session_copied_adds_to_current_and_project_session_lists_without_duplicates() {
    let mut app = app();
    let info = session_info("s1", "Copy", "/tmp/project");
    app.project_path = Some("/tmp/project".to_string());
    app.sessions = vec![info.clone()];

    let _ = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionCopied(info.clone()),
    );
    assert_eq!(app.sessions.iter().filter(|s| s.id == "s1").count(), 1);
    assert_eq!(app.project_sessions.get("/tmp/project").unwrap().len(), 1);

    let second = session_info("s2", "Copy 2", "/tmp/project");
    let _ = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionCopied(second),
    );
    assert_eq!(app.sessions[0].id, "s2");
    assert_eq!(app.project_sessions.get("/tmp/project").unwrap()[0].id, "s2");
}

#[test]
fn session_copied_without_active_project_uses_info_directory_bucket_only() {
    let mut app = app();
    let info = session_info("copy-dir", "Copy", "/tmp/copied");

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionCopied(info),
    );

    assert!(task.is_some());
    assert!(app.sessions.is_empty());
    assert_eq!(app.project_sessions.get("/tmp/copied").unwrap()[0].id, "copy-dir");
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
fn session_title_double_click_enters_rename_mode_and_closes_actions() {
    let mut app = app();
    app.sessions = vec![session_info("s1", "First", "/tmp/project")];
    app.session_menu_id = Some("s1".to_string());
    app.session_menu_anchor = Some(iced::Point::new(1.0, 2.0));
    app.show_session_actions_popover = true;

    let _ = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionTitleClicked("s1".to_string()),
    );
    let _ = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionTitleClicked("s1".to_string()),
    );

    assert_eq!(app.session_rename_id.as_deref(), Some("s1"));
    assert_eq!(app.session_rename_value, "First");
    assert!(app.session_menu_id.is_none());
    assert!(app.session_menu_anchor.is_none());
    assert!(!app.show_session_actions_popover);
}

#[test]
fn session_archive_active_without_replacement_clears_active_view() {
    let mut app = app();
    let info = session_info("s1", "First", "/tmp/project");
    app.sessions = vec![info.clone()];
    app.project_sessions.insert("/tmp/project".to_string(), vec![info]);
    app.active_session_id = Some("s1".to_string());
    app.chat.push(crate::app::models::ChatMessage {
        role: ChatRole::User,
        content: "old".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionArchivePressed("s1".to_string()),
    );

    assert!(task.is_some());
    assert!(app.sessions[0].time.archived.is_some());
    assert!(app.project_sessions.get("/tmp/project").unwrap()[0].time.archived.is_some());
    assert!(app.active_session_id.is_none());
    assert!(app.chat.is_empty());
    assert_eq!(app.usage, crate::app::models::TokenUsage::default());
    assert!(app.active_session_view_state.steps.is_empty());
    assert!(app.active_session_view_state.calls.is_empty());
    assert!(!app.active_session_view_state.ui_preparing);
    assert!(!app.active_session_view_state.base_ready);
}

#[test]
fn session_archive_active_switches_to_next_unarchived_session() {
    let mut app = app();
    let active = session_info("active", "Active", "/tmp/project");
    let mut archived = session_info("archived", "Archived", "/tmp/project");
    archived.time.archived = Some(2);
    let next = session_info("next", "Next", "/tmp/project");
    app.sessions = vec![active, archived, next];
    app.active_session_id = Some("active".to_string());

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionArchivePressed("active".to_string()),
    );

    assert!(task.is_some());
    assert!(app.sessions[0].time.archived.is_some());
    assert_eq!(app.active_session_id.as_deref(), Some("next"));
}

#[test]
fn session_delete_active_switches_to_remaining_unarchived_session() {
    let mut app = app();
    let deleted = session_info("deleted", "Deleted", "/tmp/project");
    let next = session_info("next", "Next", "/tmp/project");
    app.sessions = vec![deleted.clone(), next.clone()];
    app.project_sessions.insert("/tmp/project".to_string(), vec![deleted, next]);
    app.active_session_id = Some("deleted".to_string());

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionDeletePressed("deleted".to_string()),
    );

    assert!(task.is_some());
    assert_eq!(app.active_session_id.as_deref(), Some("next"));
    assert_eq!(
        app.sessions.iter().map(|session| session.id.as_str()).collect::<Vec<_>>(),
        vec!["next"]
    );
    assert_eq!(
        app.project_sessions
            .get("/tmp/project")
            .unwrap()
            .iter()
            .map(|session| session.id.as_str())
            .collect::<Vec<_>>(),
        vec!["next"]
    );
}

#[test]
fn session_delete_active_without_replacement_clears_chat_usage_and_runtime_state() {
    let mut app = app();
    let deleted = session_info("deleted", "Deleted", "/tmp/project");
    app.sessions = vec![deleted.clone()];
    app.project_sessions.insert("/tmp/project".to_string(), vec![deleted]);
    app.active_session_id = Some("deleted".to_string());
    app.chat.push(crate::app::models::ChatMessage {
        role: ChatRole::Assistant,
        content: "old answer".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    app.usage.output_tokens = 42;
    app.active_session_view_state.updated_ms = 99;

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionDeletePressed("deleted".to_string()),
    );

    assert!(task.is_some());
    assert!(app.sessions.is_empty());
    assert!(app.project_sessions.get("/tmp/project").unwrap().is_empty());
    assert!(app.active_session_id.is_none());
    assert!(app.chat.is_empty());
    assert_eq!(app.usage, crate::app::models::TokenUsage::default());
    assert!(app.active_session_view_state.steps.is_empty());
    assert!(app.active_session_view_state.calls.is_empty());
    assert!(!app.active_session_view_state.ui_preparing);
    assert!(!app.active_session_view_state.base_ready);
}

#[test]
fn session_messages_loaded_empty_history_marks_base_ready_without_ui_task() {
    let mut app = app();
    app.active_session_id = Some("session-1".to_string());
    let usage = crate::app::models::TokenUsage {
        input_tokens: 2,
        output_tokens: 3,
        cached_tokens: 4,
        reasoning_tokens: 5,
    };

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionMessagesLoaded(Ok((
            "session-1".to_string(),
            Vec::new(),
            usage,
        ))),
    );

    assert!(task.is_some());
    assert!(app.chat.is_empty());
    assert!(app.chat_message_ids.is_empty());
    assert_eq!(app.usage.input_tokens, 2);
    assert_eq!(app.usage.output_tokens, 3);
    assert_eq!(app.usage.cached_tokens, 4);
    assert_eq!(app.usage.reasoning_tokens, 5);
    assert!(app.active_session_view_state.base_ready);
    assert!(!app.active_session_view_state.ui_preparing);
}

#[test]
fn session_messages_loaded_restores_the_historical_session_model() {
    let mut app = app();
    let previous_scroll_id = app.chat_scroll_id.clone();
    app.model = "provider/current-model".to_string();
    app.auto_model = false;
    app.prepare_historical_session_model("session-1");
    app.active_session_id = Some("session-1".to_string());
    app.sync_active_session_preferences();

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionMessagesLoaded(Ok((
            "session-1".to_string(),
            vec![text_message(assistant_info_with_usage("assistant-history"), "answer")],
            crate::app::models::TokenUsage::default(),
        ))),
    );

    assert!(task.is_some());
    assert_eq!(app.model, "provider/current-model");
    assert!(!app.auto_model);
    assert_eq!(app.get_session_runtime("session-1").model, "provider-a/model-a");
    assert_ne!(app.chat_scroll_id, previous_scroll_id);
}

#[test]
fn session_messages_loaded_restores_model_without_usage_or_step_parts() {
    let mut app = app();
    app.model = "provider/global-model".to_string();
    app.auto_model = false;
    app.prepare_historical_session_model("session-1");
    app.active_session_id = Some("session-1".to_string());

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionMessagesLoaded(Ok((
            "session-1".to_string(),
            vec![text_message(assistant_info("assistant-history"), "answer")],
            crate::app::models::TokenUsage::default(),
        ))),
    );

    assert!(task.is_some());
    assert_eq!(app.model, "provider/global-model");
    assert!(!app.auto_model);
    assert_eq!(app.get_session_runtime("session-1").model, "provider-a/model-a");
}

#[test]
fn session_messages_loaded_empty_gateway_history_restores_cached_chat() {
    let mut app = app();
    app.active_session_id = Some("session-1".to_string());
    let cached = vec![crate::app::models::ChatMessage {
        role: ChatRole::User,
        content: "cached question".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    app.store_session_chat_snapshot(
        "session-1".to_string(),
        crate::app::session::shared_chat_messages(cached),
        vec![Some("cached-user".to_string())],
    );

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionMessagesLoaded(Ok((
            "session-1".to_string(),
            Vec::new(),
            crate::app::models::TokenUsage::default(),
        ))),
    );

    assert!(task.is_some());
    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.chat[0].content, "cached question");
    assert_eq!(app.chat_message_ids, vec![Some("cached-user".to_string())]);
    assert!(app.active_session_view_state.ui_preparing);
    assert!(!app.active_session_view_state.base_ready);
}

#[test]
fn session_messages_loaded_shorter_gateway_history_preserves_cached_chat() {
    let mut app = app();
    let session_id = "session-shorter-gateway-history";
    app.active_session_id = Some(session_id.to_string());
    let cached = vec![
        crate::app::models::ChatMessage {
            role: ChatRole::User,
            content: "original question".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        crate::app::models::ChatMessage {
            role: ChatRole::Assistant,
            content: "original answer".to_string(),
            model: Some("provider-a/model-a".to_string()),
            think_timing: Vec::new(),
            processing_timing: None,
        },
        crate::app::models::ChatMessage {
            role: ChatRole::User,
            content: "latest question".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];
    app.store_session_chat_snapshot(
        session_id.to_string(),
        crate::app::session::shared_chat_messages(cached),
        vec![
            Some("original-user".to_string()),
            Some("original-assistant".to_string()),
            Some("latest-user".to_string()),
        ],
    );

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionMessagesLoaded(Ok((
            session_id.to_string(),
            vec![text_message(user_info("latest-user"), "latest question")],
            crate::app::models::TokenUsage::default(),
        ))),
    );

    assert!(task.is_some());
    assert_eq!(app.chat.len(), 3);
    assert_eq!(app.chat[0].content, "original question");
    assert_eq!(app.chat[1].content, "original answer");
    assert_eq!(app.chat[2].content, "latest question");
    assert_eq!(
        app.chat_message_ids,
        vec![
            Some("original-user".to_string()),
            Some("original-assistant".to_string()),
            Some("latest-user".to_string()),
        ]
    );
}

#[test]
fn session_messages_loaded_error_without_cached_history_sets_error_message() {
    let mut app = app();
    app.active_session_id = Some("session-1".to_string());

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionMessagesLoaded(Err(
            "gateway unavailable".to_string(),
        )),
    );

    assert!(task.is_some());
    assert_eq!(
        app.error_message.as_deref(),
        Some("Failed to load session messages: gateway unavailable")
    );
}

#[test]
fn session_messages_loaded_error_restores_cached_history_when_available() {
    let mut app = app();
    app.active_session_id = Some("session-1".to_string());
    let cached = vec![crate::app::models::ChatMessage {
        role: ChatRole::Assistant,
        content: "cached answer".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    app.store_session_chat_snapshot(
        "session-1".to_string(),
        crate::app::session::shared_chat_messages(cached),
        Vec::new(),
    );

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionMessagesLoaded(Err(
            "gateway unavailable".to_string(),
        )),
    );

    assert!(task.is_some());
    assert!(app.error_message.is_none());
    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.chat[0].content, "cached answer");
    assert_eq!(app.chat_message_ids, vec![None]);
    assert!(app.active_session_view_state.ui_preparing);
    assert!(!app.active_session_view_state.base_ready);
}

#[test]
fn session_messages_loaded_for_inactive_session_preserves_active_state() {
    let mut app = app();
    app.active_session_id = Some("active".to_string());
    app.chat.push(crate::app::models::ChatMessage {
        role: ChatRole::User,
        content: "keep".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    app.usage.input_tokens = 7;

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionMessagesLoaded(Ok((
            "inactive".to_string(),
            vec![text_message(user_info("inactive-message"), "discard")],
            crate::app::models::TokenUsage::default(),
        ))),
    );

    assert!(task.is_some());
    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.chat[0].content, "keep");
    assert_eq!(app.usage.input_tokens, 7);
}

#[test]
fn prepared_session_open_for_inactive_target_preserves_active_state() {
    let mut app = app();
    app.active_session_id = Some("active".to_string());
    app.chat.push(crate::app::models::ChatMessage {
        role: ChatRole::User,
        content: "keep".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    let prepared = crate::app::session::prepare_session_open(
        "inactive".to_string(),
        vec![crate::app::models::ChatMessage {
            role: ChatRole::Assistant,
            content: "discard".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }],
        vec![Some("inactive-message".to_string())],
        crate::app::models::TokenUsage::default(),
        Vec::new(),
        Vec::new(),
        0,
        None,
        false,
    );

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "inactive".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::RemoteHistory,
            result: Ok(prepared),
        },
    );

    assert!(task.is_some());
    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.chat[0].content, "keep");
}

#[test]
fn loaded_session_preserves_step_llm_raw_for_active_view() {
    let mut app = app();
    app.active_session_id = Some("session-1".to_string());
    let mut prepared = crate::app::session::prepare_session_open(
        "session-1".to_string(),
        Vec::new(),
        Vec::new(),
        crate::app::models::TokenUsage::default(),
        Vec::new(),
        Vec::new(),
        0,
        None,
        false,
    );
    let raw_key = ("session-1".to_string(), "turn-3".to_string(), 3);
    let other_key = ("session-1".to_string(), "turn-8".to_string(), 8);
    prepared.step_llm_raw = std::collections::HashMap::from([(
        raw_key.clone(),
        json!({"output": {"reasoning_content": "kept"}}),
    )]);
    prepared.llm_raw_step_indices = vec![raw_key.clone(), other_key.clone()];

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "session-1".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::RemoteHistory,
            result: Ok(prepared),
        },
    );

    assert!(task.is_some());
    assert_eq!(
        app.active_session_view_state.step_llm_raw[&raw_key]["output"]["reasoning_content"],
        "kept"
    );
    assert_eq!(
        app.active_session_view_state.llm_raw_step_indices,
        std::collections::HashSet::from([raw_key, other_key])
    );
}

#[test]
fn loaded_session_refreshes_raw_when_trajectory_is_already_open() {
    let mut app = app();
    app.active_session_id = Some("session-1".to_string());
    app.chat_panel_mode = crate::app::state::ChatPanelMode::Trajectory;
    let mut prepared = crate::app::session::prepare_session_open(
        "session-1".to_string(),
        Vec::new(),
        Vec::new(),
        crate::app::models::TokenUsage::default(),
        Vec::new(),
        Vec::new(),
        0,
        None,
        false,
    );
    prepared.llm_raw_step_indices = vec![
        ("session-1".to_string(), "turn-3".to_string(), 3),
        ("session-1".to_string(), "turn-8".to_string(), 8),
    ];

    let task = super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "session-1".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::RemoteHistory,
            result: Ok(prepared),
        },
    )
    .expect("prepared session should be handled");

    assert_eq!(task.units(), 2);
    assert!(app.trajectory_raw_loading.contains(&(
        "session-1".to_string(),
        "turn-3".to_string(),
        3,
    )));
    assert!(app.trajectory_raw_loading.contains(&(
        "session-1".to_string(),
        "turn-8".to_string(),
        8,
    )));
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
