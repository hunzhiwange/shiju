//! 验证 TUI v2 状态容器的基础行为。
//! 这些用例保护滚动、输入和会话状态的组合更新。

use std::path::PathBuf;

use serde_json::json;
use vw_gateway_client::GatewayChatUsage;
use vw_shared::session::ui_types::{
    ChatMessage, ChatRole, ChatSession, ChatSessionStep, ThinkTiming, TokenUsage,
};

use super::reducer::{
    TuiAction, TuiTerminalUpdate, TuiToolCallUpdate, TuiToolResultUpdate, reduce_tui_state,
};
use super::selectors::{
    TuiAssistantTurnEntry, TuiExploreToolKind, TuiTranscriptItem, select_grouped_transcript,
    select_search_matches, select_status_summary, select_transcript_message_anchors,
    select_visible_grouped_transcript_window, select_visible_message_window,
};
use super::{TuiScrollState, TuiState, apply_runtime_event};
use crate::cli::session::GitWorkspaceStatus;
use crate::cli::tui_v2::model::{
    PromptSubmission, UiMemoryEntry, UiMessage, UiMessageKind, UiQuestionOverlay, UiQuestionPrompt,
    UiSearchOverlay, UiSystemMessageLevel, UiTodoItem, UiTodoOverlay, UiTokenUsage,
    UiToolCallState, UiTurnTerminal,
};
use crate::cli::tui_v2::runtime::stream_adapter::{UiRuntimeEvent, UiRuntimeTerminalEvent};

#[test]
fn chat_session_round_trip_preserves_snapshot_payloads() {
    let snapshot = ChatSession {
        id: "session_roundtrip".to_string(),
        title: "状态层验证".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "把 S2-2 接起来".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "已恢复 snapshot -> state -> snapshot round-trip".to_string(),
                model: None,
                think_timing: vec![ThinkTiming {
                    start_ms: 11,
                    end_ms: Some(13),
                    last_update_ms: 13,
                }],
                processing_timing: None,
            },
        ],
        message_ids: vec![Some("msg-user".to_string()), Some("msg-assistant".to_string())],
        calls: vec![json!({"tool": "search", "state": "complete"})],
        steps: vec![ChatSessionStep {
            index: 1,
            started_ms: 21,
            finished_ms: Some(34),
            start_snapshot_path: Some("snapshots/start.json".to_string()),
            finish_snapshot_path: Some("snapshots/finish.json".to_string()),
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 20,
                cached_tokens: 2,
                reasoning_tokens: 3,
            },
            cost_usd: None,
            finish_reason: Some("stop".to_string()),
            model: Some("gpt-5.4".to_string()),
        }],
        created_ms: 1,
        updated_ms: 2,
    };

    let state = TuiState::from_chat_session(&snapshot);
    let restored = state.to_chat_session();

    assert_eq!(state.session.session_id.as_deref(), Some("session_roundtrip"));
    assert_eq!(state.messages.len(), 3);
    assert_eq!(restored.id, snapshot.id);
    assert_eq!(restored.title, snapshot.title);
    assert_eq!(restored.message_ids, snapshot.message_ids);
    assert_eq!(restored.calls, snapshot.calls);
    assert_eq!(restored.created_ms, snapshot.created_ms);
    assert_eq!(restored.updated_ms, snapshot.updated_ms);
    assert_eq!(restored.messages.len(), 2);
    assert_eq!(restored.messages[1].content, snapshot.messages[1].content);
    assert_eq!(restored.messages[1].think_timing.len(), 1);
    assert_eq!(restored.messages[1].think_timing[0].start_ms, 11);
    assert_eq!(restored.steps.len(), 1);
    assert_eq!(restored.steps[0].index, 1);
    assert_eq!(restored.steps[0].usage.output_tokens, 20);
    assert_eq!(restored.steps[0].finish_reason.as_deref(), Some("stop"));
}

#[test]
fn chat_session_round_trip_preserves_tool_messages() {
    let snapshot = ChatSession {
        id: "session_tool_roundtrip".to_string(),
        title: "Tool Round Trip".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "检查 tool snapshot".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::Tool,
                content: "tool grep\n{\"status\":\"completed\",\"output\":\"2 matches\"}\n"
                    .to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        ],
        message_ids: vec![Some("user-1".to_string()), Some("tool-1".to_string())],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 10,
        updated_ms: 11,
    };

    let state = TuiState::from_chat_session(&snapshot);

    let UiMessage::ToolResult(tool_result) = &state.messages[1] else {
        panic!("tool result should be restored");
    };
    assert_eq!(tool_result.tool_name, "grep");
    assert_eq!(tool_result.content, "2 matches");
    assert!(!tool_result.is_error);

    let restored = state.to_chat_session();
    assert_eq!(restored.messages.len(), 2);
    assert_eq!(restored.messages[1].role, ChatRole::Tool);
    assert_eq!(restored.messages[1].content, snapshot.messages[1].content);
    assert_eq!(restored.message_ids[1].as_deref(), Some("tool-1"));
    assert_eq!(state.status.turn_terminal, UiTurnTerminal::Done { finish_reason: None });
}

#[test]
fn reducer_tracks_submission_stream_and_terminal_lifecycle() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_stream".to_string(),
        title: "Streaming".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 5,
        updated_ms: 5,
    });

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            PromptSubmission::new("继续做 reducer")
                .with_stream_id(88)
                .with_session_id("session_stream")
                .with_model("gpt-5.4"),
        ),
    );
    reduce_tui_state(&mut state, TuiAction::AssistantDeltaReceived("状态层已接好".to_string()));
    reduce_tui_state(
        &mut state,
        TuiAction::StepStarted {
            step_index: 1,
            started_ms: 89,
            model: Some("gpt-5.4".to_string()),
        },
    );
    reduce_tui_state(
        &mut state,
        TuiAction::StepFinished {
            step_index: 1,
            finished_ms: 90,
            usage: UiTokenUsage {
                input_tokens: 12,
                output_tokens: 24,
                cached_tokens: 1,
                reasoning_tokens: 4,
            },
            finish_reason: Some("stop".to_string()),
            model: Some("gpt-5.4".to_string()),
        },
    );
    reduce_tui_state(
        &mut state,
        TuiAction::AssistantTerminalUpdated(TuiTerminalUpdate {
            terminal: UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) },
            usage: None,
            message_id: Some("assistant-1".to_string()),
            parent_message_id: Some("user-1".to_string()),
        }),
    );

    assert_eq!(state.messages.len(), 3);
    assert_eq!(state.messages[0].kind(), UiMessageKind::User);
    assert_eq!(state.messages[1].kind(), UiMessageKind::Assistant);
    assert_eq!(state.messages[2].kind(), UiMessageKind::Step);
    assert_eq!(
        state.status.turn_terminal,
        UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) }
    );
    assert_eq!(
        state.prompt.last_submission.as_ref().map(|submission| &submission.status),
        Some(&crate::cli::tui_v2::model::PromptSubmissionStatus::Done {
            finish_reason: Some("stop".to_string())
        })
    );

    let UiMessage::Assistant(assistant) = &state.messages[1] else {
        panic!("assistant message should exist");
    };
    assert_eq!(assistant.text, "状态层已接好");
    assert_eq!(assistant.step_count, 1);
    assert_eq!(assistant.usage.output_tokens, 24);
    assert_eq!(assistant.base.parent_id.as_ref().map(|id| id.as_str()), Some("gateway:user-1"));
    assert_eq!(state.session.persisted_messages[1].raw_message_id.as_deref(), Some("assistant-1"));

    let snapshot = state.to_chat_session();
    assert_eq!(snapshot.message_ids[1].as_deref(), Some("assistant-1"));
    assert_eq!(snapshot.steps.len(), 1);
    assert_eq!(snapshot.steps[0].usage.reasoning_tokens, 4);
}

#[test]
fn replace_from_snapshot_clears_project_memory_evidence() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_before_restore".to_string(),
        title: "Before Restore".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    });
    state.project.workspace_root = Some(PathBuf::from("/tmp/current-workspace"));
    state.project.info = "current project".to_string();
    state.project.git_status = GitWorkspaceStatus::ReadyDirty(vec!["src/main.rs".to_string()]);
    state.project.memory_evidence = Some(UiMemoryEntry {
        scope: "project".to_string(),
        filename: "notes.md".to_string(),
        path: "/tmp/current-workspace/notes.md".to_string(),
        preview_lines: vec!["旧 session 的 memory 证据".to_string()],
        total_lines: 5,
    });

    reduce_tui_state(
        &mut state,
        TuiAction::ReplaceFromSnapshot {
            snapshot: ChatSession {
                id: "session_after_restore".to_string(),
                title: "After Restore".to_string(),
                messages: vec![ChatMessage {
                    role: ChatRole::Assistant,
                    content: "restored content".to_string(),
                    model: None,
                    think_timing: Vec::new(),
                    processing_timing: None,
                }],
                message_ids: vec![Some("assistant-restored".to_string())],
                calls: Vec::new(),
                steps: Vec::new(),
                created_ms: 3,
                updated_ms: 4,
            },
            scope: Some("workspace".to_string()),
            path: Some(PathBuf::from("/tmp/sessions/session_after_restore.json")),
        },
    );

    assert_eq!(state.project.workspace_root, Some(PathBuf::from("/tmp/current-workspace")));
    assert_eq!(state.project.info, "current project");
    assert_eq!(
        state.project.git_status,
        GitWorkspaceStatus::ReadyDirty(vec!["src/main.rs".to_string()])
    );
    assert_eq!(state.project.memory_evidence, None);
    assert_eq!(state.session.session_id.as_deref(), Some("session_after_restore"));
    assert_eq!(state.session.title, "After Restore");
    assert_eq!(state.session.scope.as_deref(), Some("workspace"));
    assert_eq!(state.session.path, Some(PathBuf::from("/tmp/sessions/session_after_restore.json")));
}

#[test]
fn to_chat_session_reindexes_steps_across_multiple_turns() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_multi_turn_steps".to_string(),
        title: "Step Reindex".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 10,
        updated_ms: 10,
    });

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            PromptSubmission::new("first turn")
                .with_stream_id(11)
                .with_session_id("session_multi_turn_steps")
                .with_model("gpt-5.4"),
        ),
    );
    reduce_tui_state(&mut state, TuiAction::AssistantDeltaReceived("first result".to_string()));
    reduce_tui_state(
        &mut state,
        TuiAction::StepStarted {
            step_index: 1,
            started_ms: 12,
            model: Some("gpt-5.4".to_string()),
        },
    );
    reduce_tui_state(
        &mut state,
        TuiAction::StepFinished {
            step_index: 1,
            finished_ms: 13,
            usage: UiTokenUsage::default(),
            finish_reason: Some("stop".to_string()),
            model: Some("gpt-5.4".to_string()),
        },
    );
    reduce_tui_state(
        &mut state,
        TuiAction::AssistantTerminalUpdated(TuiTerminalUpdate {
            terminal: UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) },
            usage: None,
            message_id: Some("assistant-first".to_string()),
            parent_message_id: Some("user-first".to_string()),
        }),
    );

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            PromptSubmission::new("second turn")
                .with_stream_id(21)
                .with_session_id("session_multi_turn_steps")
                .with_model("gpt-5.4"),
        ),
    );
    reduce_tui_state(&mut state, TuiAction::AssistantDeltaReceived("second result".to_string()));
    reduce_tui_state(
        &mut state,
        TuiAction::StepStarted {
            step_index: 1,
            started_ms: 22,
            model: Some("gpt-5.4".to_string()),
        },
    );
    reduce_tui_state(
        &mut state,
        TuiAction::StepFinished {
            step_index: 1,
            finished_ms: 23,
            usage: UiTokenUsage::default(),
            finish_reason: Some("stop".to_string()),
            model: Some("gpt-5.4".to_string()),
        },
    );
    reduce_tui_state(
        &mut state,
        TuiAction::AssistantTerminalUpdated(TuiTerminalUpdate {
            terminal: UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) },
            usage: None,
            message_id: Some("assistant-second".to_string()),
            parent_message_id: Some("user-second".to_string()),
        }),
    );

    let snapshot = state.to_chat_session();
    assert_eq!(snapshot.steps.len(), 2);
    assert_eq!(snapshot.steps[0].index, 1);
    assert_eq!(snapshot.steps[1].index, 2);
    assert_eq!(snapshot.steps[0].finish_reason.as_deref(), Some("stop"));
    assert_eq!(snapshot.steps[1].finish_reason.as_deref(), Some("stop"));
}

#[test]
fn selectors_derive_visible_window_status_and_search_matches() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_selector".to_string(),
        title: "Selector".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "bridge reducer".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "Bridge search results into overlay state".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::System,
                content: "bridge status summary".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "Visible window should not depend on renderer".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        ],
        message_ids: vec![
            Some("m-1".to_string()),
            Some("m-2".to_string()),
            Some("m-3".to_string()),
            Some("m-4".to_string()),
        ],
        calls: Vec::new(),
        steps: vec![ChatSessionStep {
            index: 1,
            started_ms: 30,
            finished_ms: Some(40),
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage: TokenUsage {
                input_tokens: 3,
                output_tokens: 9,
                cached_tokens: 0,
                reasoning_tokens: 1,
            },
            cost_usd: None,
            finish_reason: Some("stop".to_string()),
            model: Some("gpt-5.4".to_string()),
        }],
        created_ms: 3,
        updated_ms: 4,
    });

    state.scroll = TuiScrollState {
        top_message: 2,
        viewport_messages: 2,
        viewport_height: 2,
        viewport_width: 24,
        overscan: 1,
        follow_tail: false,
        sticky_message: Some(1),
        last_seen_message: Some(2),
    };
    state.tasks.pending_questions = vec![UiQuestionOverlay {
        request_id: "q-1".to_string(),
        session_id: "session_selector".to_string(),
        prompts: vec![UiQuestionPrompt {
            header: "执行模式".to_string(),
            question: "是否继续 bridge?".to_string(),
            options: Vec::new(),
            multiple: false,
            allow_custom_input: true,
        }],
        answers: vec![Vec::new()],
        tool: None,
        selected_index: 0,
    }];
    state.tasks.todo_overlay = Some(UiTodoOverlay {
        session_id: Some("session_selector".to_string()),
        items: vec![UiTodoItem {
            id: "todo-1".to_string(),
            content: "bridge search matches".to_string(),
            status: "in_progress".to_string(),
            priority: "high".to_string(),
        }],
        selected_index: 0,
        dirty: false,
    });
    state.overlays.push(crate::cli::tui_v2::model::UiOverlay::Search(UiSearchOverlay {
        query: "bridge".to_string(),
        ..UiSearchOverlay::default()
    }));

    let visible = select_visible_message_window(&state);
    let status = select_status_summary(&state);
    let matches = select_search_matches(&state);

    assert_eq!(visible.len(), 4);
    assert_eq!(visible.first().map(UiMessage::kind), Some(UiMessageKind::Assistant));
    assert_eq!(status.message_count, 5);
    assert_eq!(status.step_count, 1);
    assert_eq!(status.pending_questions, 1);
    assert_eq!(status.todo_count, 1);
    assert_eq!(status.token_usage.output_tokens, 9);
    assert_eq!(matches.len(), 3);
    assert!(matches.iter().all(|item| item.preview.to_ascii_lowercase().contains("bridge")));
}

#[test]
fn searchable_text_cache_tracks_incremental_assistant_updates() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_search_cache".to_string(),
        title: "Search Cache".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 12,
        updated_ms: 12,
    });

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            PromptSubmission::new("继续补 searchable cache")
                .with_stream_id(201)
                .with_session_id("session_search_cache")
                .with_model("gpt-5.4"),
        ),
    );
    reduce_tui_state(&mut state, TuiAction::AssistantDeltaReceived("search cache".to_string()));
    reduce_tui_state(&mut state, TuiAction::SearchQuerySet("cache".to_string()));

    let first_matches = select_search_matches(&state);
    assert_eq!(first_matches.len(), 2);

    reduce_tui_state(&mut state, TuiAction::AssistantDeltaReceived(" tail".to_string()));
    reduce_tui_state(&mut state, TuiAction::SearchQuerySet("tail".to_string()));

    let second_matches = select_search_matches(&state);
    assert_eq!(second_matches.len(), 1);
    assert!(second_matches[0].preview.contains("tail"));
}

#[test]
fn transcript_projection_cache_refreshes_after_message_append() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_projection_cache".to_string(),
        title: "Projection Cache".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "先建 grouped projection".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "assistant body".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        ],
        message_ids: vec![Some("msg-user".to_string()), Some("msg-assistant".to_string())],
        calls: Vec::new(),
        steps: vec![ChatSessionStep {
            index: 1,
            started_ms: 30,
            finished_ms: Some(31),
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 2,
                cached_tokens: 0,
                reasoning_tokens: 0,
            },
            cost_usd: None,
            finish_reason: Some("stop".to_string()),
            model: Some("gpt-5.4".to_string()),
        }],
        created_ms: 20,
        updated_ms: 21,
    });

    assert_eq!(select_transcript_message_anchors(&state), vec![0, 1]);

    state.scroll.follow_tail = false;
    state.scroll.top_message = 2;
    state.clamp_scroll();
    assert_eq!(state.scroll.top_message, 1);

    reduce_tui_state(
        &mut state,
        TuiAction::MessagePushed(UiMessage::System(crate::cli::tui_v2::model::UiSystemMessage {
            base: crate::cli::tui_v2::model::UiMessageBase::new(
                crate::cli::tui_v2::model::UiMessageId::local("sys-cache-refresh"),
            ),
            text: "system boundary".to_string(),
            level: UiSystemMessageLevel::Info,
        })),
    );

    assert_eq!(select_transcript_message_anchors(&state), vec![0, 1, 3]);

    state.scroll.top_message = 2;
    state.clamp_scroll();
    assert_eq!(state.scroll.top_message, 1);

    state.scroll.top_message = 3;
    state.clamp_scroll();
    assert_eq!(state.scroll.top_message, 3);
}

#[test]
fn runtime_pipeline_extracts_thinking_blocks_from_delta_stream() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_thinking".to_string(),
        title: "Thinking Runtime".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 10,
        updated_ms: 10,
    });

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            PromptSubmission::new("拆 thinking")
                .with_stream_id(101)
                .with_session_id("session_thinking")
                .with_model("gpt-5.4"),
        ),
    );
    apply_runtime_event(&mut state, UiRuntimeEvent::Delta("<think>先读 controller".to_string()));
    apply_runtime_event(&mut state, UiRuntimeEvent::Delta("，再接 reducer".to_string()));
    apply_runtime_event(&mut state, UiRuntimeEvent::Delta("</think>\n\n最终回答".to_string()));

    assert!(!state.runtime.thinking_open);
    assert_eq!(state.messages.len(), 3);

    let UiMessage::Thinking(thinking) = &state.messages[1] else {
        panic!("thinking message should exist");
    };
    assert_eq!(thinking.content, "先读 controller，再接 reducer");
    assert_eq!(thinking.summary.as_deref(), Some("先读 controller，再接 reducer"));
    assert_eq!(thinking.timing.len(), 1);
    assert!(thinking.timing[0].end_ms.is_some());

    let UiMessage::Assistant(assistant) = &state.messages[2] else {
        panic!("assistant message should exist");
    };
    assert_eq!(assistant.text, "最终回答");

    let snapshot = state.to_chat_session();
    assert_eq!(snapshot.messages.len(), 2);
    assert!(snapshot.messages.iter().all(|message| !message.content.contains("<think>")));
}

#[test]
fn runtime_pipeline_extracts_tool_messages_from_structured_delta() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_tool".to_string(),
        title: "Tool Runtime".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 12,
        updated_ms: 12,
    });

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            PromptSubmission::new("拆 tool")
                .with_stream_id(102)
                .with_session_id("session_tool")
                .with_model("gpt-5.4"),
        ),
    );
    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Delta(
            "tool grep\n{\"status\":\"running\",\"input\":\"bridge reducer\",\"title\":\"grep\",\"metadata\":{\"truncated\":false},\"output\":\"\"}\n"
                .to_string(),
        ),
    );
    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Delta(
            "tool grep\n{\"status\":\"completed\",\"input\":\"bridge reducer\",\"title\":\"grep\",\"metadata\":{\"truncated\":false},\"output\":\"2 matches\"}\n"
                .to_string(),
        ),
    );

    assert_eq!(state.messages.len(), 3);

    let UiMessage::ToolCall(tool_call) = &state.messages[1] else {
        panic!("tool call message should exist");
    };
    assert_eq!(tool_call.tool_name, "grep");
    assert_eq!(tool_call.arguments.as_deref(), Some("bridge reducer"));
    assert_eq!(tool_call.state, UiToolCallState::Complete);

    let UiMessage::ToolResult(tool_result) = &state.messages[2] else {
        panic!("tool result message should exist");
    };
    assert_eq!(tool_result.tool_name, "grep");
    assert_eq!(tool_result.content, "2 matches");
    assert!(!tool_result.is_error);
    assert_eq!(
        tool_result.base.parent_id.as_ref().map(|id| id.as_str()),
        Some(tool_call.base.id.as_str())
    );

    let snapshot = state.to_chat_session();
    assert_eq!(snapshot.messages.len(), 2);
    assert_eq!(snapshot.messages[1].role, ChatRole::Tool);
    assert!(snapshot.messages[1].content.contains("\"output\":\"2 matches\""));
}

#[test]
fn runtime_pipeline_applies_terminal_usage_and_ignores_ui_warning_in_snapshot() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_runtime".to_string(),
        title: "Runtime Pipeline".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 8,
        updated_ms: 8,
    });

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            PromptSubmission::new("接入真实网关")
                .with_stream_id(99)
                .with_session_id("session_runtime")
                .with_model("gpt-5.4"),
        ),
    );
    apply_runtime_event(&mut state, UiRuntimeEvent::Delta("已切到消息管线".to_string()));
    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Done {
            finish_reason: Some("stop".to_string()),
            usage: Some(GatewayChatUsage {
                input_tokens: 5,
                output_tokens: 13,
                cached_tokens: 2,
                reasoning_tokens: 3,
            }),
            message_id: Some("assistant-final".to_string()),
            parent_message_id: Some("user-final".to_string()),
        }),
    );
    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Unknown { event_type: Some("chat.tool_delta".to_string()) },
    );

    let UiMessage::Assistant(assistant) = &state.messages[1] else {
        panic!("assistant message should exist");
    };
    assert_eq!(assistant.text, "已切到消息管线");
    assert_eq!(assistant.usage.output_tokens, 13);
    assert_eq!(assistant.base.id.as_str(), "gateway:assistant-final");
    assert_eq!(assistant.base.parent_id.as_ref().map(|id| id.as_str()), Some("gateway:user-final"));

    let UiMessage::System(warning) = state.messages.last().expect("warning message should exist")
    else {
        panic!("unknown runtime event should become a warning system message");
    };
    assert_eq!(warning.level, UiSystemMessageLevel::Warning);
    assert!(warning.text.contains("chat.tool_delta"));
    assert!(warning.base.id.as_str().starts_with("local:ui-runtime-warning-"));

    let snapshot = state.to_chat_session();
    assert_eq!(snapshot.messages.len(), 2);
    assert!(
        snapshot
            .messages
            .iter()
            .all(|message| !message.content.contains("Unsupported gateway event"))
    );
}

#[test]
fn grouped_transcript_associates_preface_and_child_messages_with_assistant_turn() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_grouped".to_string(),
        title: "Grouped Transcript".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 20,
        updated_ms: 20,
    });

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            PromptSubmission::new("开始做 grouped view")
                .with_stream_id(200)
                .with_session_id("session_grouped")
                .with_model("gpt-5.4"),
        ),
    );
    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Delta("<think>先看 selectors</think>整理 grouped view".to_string()),
    );
    reduce_tui_state(
        &mut state,
        TuiAction::StepStarted {
            step_index: 1,
            started_ms: 201,
            model: Some("gpt-5.4".to_string()),
        },
    );
    reduce_tui_state(
        &mut state,
        TuiAction::StepFinished {
            step_index: 1,
            finished_ms: 202,
            usage: UiTokenUsage {
                input_tokens: 5,
                output_tokens: 7,
                cached_tokens: 0,
                reasoning_tokens: 2,
            },
            finish_reason: Some("stop".to_string()),
            model: Some("gpt-5.4".to_string()),
        },
    );
    reduce_tui_state(
        &mut state,
        TuiAction::ToolCallUpdated(TuiToolCallUpdate {
            tool_name: "grep".to_string(),
            summary: None,
            arguments: Some("selectors".to_string()),
            append_arguments: false,
            state: UiToolCallState::Complete,
            result: Some(TuiToolResultUpdate { content: "2 matches".to_string(), is_error: false }),
        }),
    );
    reduce_tui_state(
        &mut state,
        TuiAction::AssistantTerminalUpdated(TuiTerminalUpdate {
            terminal: UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) },
            usage: None,
            message_id: Some("assistant-turn-1".to_string()),
            parent_message_id: Some("user-turn-1".to_string()),
        }),
    );

    let transcript = select_grouped_transcript(&state);

    assert_eq!(transcript.len(), 2);
    assert!(matches!(transcript.first(), Some(TuiTranscriptItem::Standalone(UiMessage::User(_)))));

    let Some(TuiTranscriptItem::AssistantTurn(turn)) = transcript.get(1) else {
        panic!("assistant turn should exist");
    };
    assert_eq!(turn.assistant.text, "整理 grouped view");
    assert_eq!(turn.assistant.base.id.as_str(), "gateway:assistant-turn-1");
    assert_eq!(turn.preface.len(), 1);
    assert_eq!(turn.children.len(), 2);

    let TuiAssistantTurnEntry::Thinking(thinking) = &turn.preface[0] else {
        panic!("thinking preface should exist");
    };
    assert_eq!(thinking.content, "先看 selectors");

    let TuiAssistantTurnEntry::Step(step) = &turn.children[0] else {
        panic!("step child should exist");
    };
    assert_eq!(step.step_index, 1);
    assert_eq!(step.usage.output_tokens, 7);

    let TuiAssistantTurnEntry::Tool(tool_group) = &turn.children[1] else {
        panic!("tool child should exist");
    };
    assert_eq!(tool_group.call.tool_name, "grep");
    assert_eq!(tool_group.call.arguments.as_deref(), Some("selectors"));
    assert_eq!(tool_group.results.len(), 1);
    assert_eq!(tool_group.results[0].content, "2 matches");

    let snapshot = state.to_chat_session();
    assert_eq!(snapshot.messages.len(), 3);
    assert_eq!(snapshot.message_ids[1].as_deref(), Some("assistant-turn-1"));
    assert_eq!(snapshot.messages[2].role, ChatRole::Tool);
    assert!(snapshot.messages.iter().all(|message| !message.content.contains("先看 selectors")));
    assert!(snapshot.messages[2].content.contains("\"output\":\"2 matches\""));
}

#[test]
fn grouped_transcript_collapses_only_contiguous_explore_tool_results() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_collapsed".to_string(),
        title: "Collapsed Transcript".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 30,
        updated_ms: 30,
    });

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            PromptSubmission::new("继续做 collapsed view")
                .with_stream_id(300)
                .with_session_id("session_collapsed")
                .with_model("gpt-5.4"),
        ),
    );
    reduce_tui_state(&mut state, TuiAction::AssistantDeltaReceived("工具结果已收口".to_string()));

    for (tool_name, arguments, content) in [
        ("grep", "selectors", "2 matches"),
        ("read_file", "state/selectors.rs", "pub(crate) fn ..."),
        ("apply_patch", "selectors.rs", "updated 1 file"),
        ("semantic_search", "assistant turn", "1 snippet"),
    ] {
        reduce_tui_state(
            &mut state,
            TuiAction::ToolCallUpdated(TuiToolCallUpdate {
                tool_name: tool_name.to_string(),
                summary: None,
                arguments: Some(arguments.to_string()),
                append_arguments: false,
                state: UiToolCallState::Complete,
                result: Some(TuiToolResultUpdate { content: content.to_string(), is_error: false }),
            }),
        );
    }

    let transcript = select_grouped_transcript(&state);
    let Some(TuiTranscriptItem::AssistantTurn(turn)) = transcript.get(1) else {
        panic!("assistant turn should exist");
    };

    assert_eq!(turn.children.len(), 3);

    let TuiAssistantTurnEntry::CollapsedTools(batch) = &turn.children[0] else {
        panic!("contiguous explore tools should collapse");
    };
    assert_eq!(batch.calls.len(), 2);
    assert_eq!(batch.total_results, 2);
    assert_eq!(batch.tool_counts.len(), 2);
    assert_eq!(batch.tool_counts[0].kind, TuiExploreToolKind::Grep);
    assert_eq!(batch.tool_counts[0].count, 1);
    assert_eq!(batch.tool_counts[1].kind, TuiExploreToolKind::Read);
    assert_eq!(batch.tool_counts[1].count, 1);
    assert!(batch.summary.contains("grep x1"));
    assert!(batch.summary.contains("read x1"));

    let TuiAssistantTurnEntry::Tool(tool_group) = &turn.children[1] else {
        panic!("non-explore tool should remain expanded");
    };
    assert_eq!(tool_group.call.tool_name, "apply_patch");
    assert_eq!(tool_group.results.len(), 1);

    let TuiAssistantTurnEntry::Tool(tool_group) = &turn.children[2] else {
        panic!("single explore tool after a boundary should remain expanded");
    };
    assert_eq!(tool_group.call.tool_name, "semantic_search");
    assert_eq!(tool_group.results.len(), 1);
}

#[test]
fn visible_grouped_window_keeps_assistant_turn_intact_across_message_boundary() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_visible_grouped".to_string(),
        title: "Visible Grouped Window".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 40,
        updated_ms: 40,
    });

    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            PromptSubmission::new("继续做 grouped window")
                .with_stream_id(400)
                .with_session_id("session_visible_grouped")
                .with_model("gpt-5.4"),
        ),
    );
    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Delta("<think>先收 transcript window</think>再补 step".to_string()),
    );
    reduce_tui_state(
        &mut state,
        TuiAction::StepStarted {
            step_index: 1,
            started_ms: 401,
            model: Some("gpt-5.4".to_string()),
        },
    );
    reduce_tui_state(
        &mut state,
        TuiAction::StepFinished {
            step_index: 1,
            finished_ms: 402,
            usage: UiTokenUsage {
                input_tokens: 1,
                output_tokens: 2,
                cached_tokens: 0,
                reasoning_tokens: 1,
            },
            finish_reason: Some("stop".to_string()),
            model: Some("gpt-5.4".to_string()),
        },
    );
    reduce_tui_state(
        &mut state,
        TuiAction::ToolCallUpdated(TuiToolCallUpdate {
            tool_name: "grep".to_string(),
            summary: None,
            arguments: Some("window".to_string()),
            append_arguments: false,
            state: UiToolCallState::Complete,
            result: Some(TuiToolResultUpdate { content: "1 match".to_string(), is_error: false }),
        }),
    );

    assert_eq!(state.messages.len(), 6);

    state.scroll.top_message = 3;
    state.scroll.viewport_messages = 1;
    state.scroll.viewport_height = 1;
    state.scroll.viewport_width = 18;
    state.scroll.overscan = 0;
    state.scroll.follow_tail = false;
    state.scroll.sticky_message = Some(2);
    state.scroll.last_seen_message = Some(2);
    state.refresh_transcript_layout_for_current_width();

    let visible = select_visible_grouped_transcript_window(&state);
    let viewport_summary = visible.viewport_summary();
    let window_summary = visible.window_summary();

    assert_eq!(visible.total_items, 2);
    assert_eq!(visible.top_item_index, 1);
    assert_eq!(visible.start_item_index, 1);
    assert_eq!(visible.end_item_index, 2);
    assert_eq!(visible.covered_message_start, 1);
    assert_eq!(visible.covered_message_end, 6);
    assert_eq!(visible.len(), 1);
    assert_eq!(viewport_summary.rows, 1);
    assert_eq!(viewport_summary.message_capacity, 1);
    assert_eq!(viewport_summary.label(), "1rows/1messages");
    assert_eq!(window_summary.top_message, 3);
    assert_eq!(window_summary.coverage_label(), "items 2..2/2 msg 1..5");
    assert_eq!(window_summary.sticky_label(), "sticky m2");
    assert!(window_summary.has_sticky_anchor());
    assert!(!window_summary.follows_tail());
    assert_eq!(
        window_summary.sticky_notice().as_deref(),
        Some("message 2 is parked above the viewport host.")
    );
    let sticky_prompt =
        visible.sticky_prompt().expect("assistant turn should expose sticky prompt summary");
    assert_eq!(sticky_prompt.message_index, 0);
    assert_eq!(sticky_prompt.label(), "prompt m0");
    assert_eq!(sticky_prompt.preview, "继续做 grouped window");
    let unseen_range =
        visible.unseen_range().expect("off-tail new activity should expose unseen range");
    assert_eq!(unseen_range.first_unseen_message, 3);
    assert_eq!(unseen_range.first_unseen_item_index, 1);
    assert_eq!(unseen_range.unseen_message_count, 3);
    assert_eq!(unseen_range.unseen_item_count, 1);
    assert!(unseen_range.boundary_in_window);
    assert_eq!(unseen_range.pill_label(), "3 new messages");
    assert_eq!(unseen_range.divider_label(), "3 new messages below");
    let Some(TuiTranscriptItem::AssistantTurn(turn)) = visible.items.first() else {
        panic!("assistant turn should remain visible as one transcript item");
    };
    assert_eq!(turn.assistant.text, "再补 step");
    assert_eq!(turn.preface.len(), 1);
    assert_eq!(turn.children.len(), 2);

    let TuiAssistantTurnEntry::Thinking(thinking) = &turn.preface[0] else {
        panic!("thinking preface should stay attached");
    };
    assert_eq!(thinking.content, "先收 transcript window");

    let TuiAssistantTurnEntry::Step(step) = &turn.children[0] else {
        panic!("step child should stay attached");
    };
    assert_eq!(step.step_index, 1);

    let TuiAssistantTurnEntry::Tool(tool_group) = &turn.children[1] else {
        panic!("tool child should stay attached");
    };
    assert_eq!(tool_group.call.tool_name, "grep");
    assert_eq!(tool_group.results.len(), 1);
}

#[test]
fn visible_grouped_window_uses_height_cache_row_budget() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_height_bucket".to_string(),
        title: "Height Bucket".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "abcdefghijklmno".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::System,
                content: "tail".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        ],
        message_ids: vec![Some("msg-u".to_string()), Some("msg-s".to_string())],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 90,
        updated_ms: 91,
    });

    state.scroll = TuiScrollState {
        top_message: 0,
        viewport_messages: 2,
        viewport_height: 2,
        viewport_width: 8,
        overscan: 0,
        follow_tail: false,
        sticky_message: None,
        last_seen_message: Some(1),
    };
    state.refresh_transcript_layout_for_current_width();

    let visible = select_visible_grouped_transcript_window(&state);

    assert_eq!(visible.total_items, 2);
    assert_eq!(visible.start_item_index, 0);
    assert_eq!(visible.end_item_index, 1);
    assert_eq!(visible.covered_message_start, 0);
    assert_eq!(visible.covered_message_end, 1);
    assert_eq!(visible.len(), 1);
    let Some(TuiTranscriptItem::Standalone(UiMessage::User(message))) = visible.items.first()
    else {
        panic!("row budget should keep only the first wrapped item visible");
    };
    assert_eq!(message.text, "abcdefghijklmno");
}

#[test]
fn visible_grouped_window_falls_back_to_height_estimation_without_cache_bucket() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_height_fallback".to_string(),
        title: "Height Fallback".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "abcdefghijklmno".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::System,
                content: "tail".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        ],
        message_ids: vec![Some("msg-u".to_string()), Some("msg-s".to_string())],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 92,
        updated_ms: 93,
    });

    state.scroll = TuiScrollState {
        top_message: 0,
        viewport_messages: 99,
        viewport_height: 2,
        viewport_width: 8,
        overscan: 0,
        follow_tail: false,
        sticky_message: None,
        last_seen_message: Some(1),
    };
    state.transcript_layout.clear();

    let visible = select_visible_grouped_transcript_window(&state);

    assert_eq!(visible.total_items, 2);
    assert_eq!(visible.start_item_index, 0);
    assert_eq!(visible.end_item_index, 1);
    assert_eq!(visible.covered_message_start, 0);
    assert_eq!(visible.covered_message_end, 1);
    assert_eq!(visible.len(), 1);
    assert_eq!(visible.viewport_summary().message_capacity, 1);
}

#[test]
fn transcript_layout_cache_rebuilds_after_resize_width_change() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "session_resize_bucket".to_string(),
        title: "Resize Bucket".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "abcdefghijklmno".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::System,
                content: "tail".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        ],
        message_ids: vec![Some("msg-u".to_string()), Some("msg-s".to_string())],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 94,
        updated_ms: 95,
    });

    state.scroll = TuiScrollState {
        top_message: 0,
        viewport_messages: 4,
        viewport_height: 4,
        viewport_width: 40,
        overscan: 0,
        follow_tail: false,
        sticky_message: None,
        last_seen_message: Some(1),
    };
    state.refresh_transcript_layout_for_current_width();
    let wide = {
        let window = select_visible_grouped_transcript_window(&state);
        (window.end_item_index, window.viewport_summary().rows)
    };

    state.scroll.viewport_width = 8;
    state.refresh_transcript_layout_for_current_width();
    let narrow = {
        let window = select_visible_grouped_transcript_window(&state);
        (window.end_item_index, window.viewport_summary().rows)
    };

    assert_eq!(wide.0, 2);
    assert_eq!(narrow.0, 1);
    assert_eq!(wide.1, 4);
    assert_eq!(narrow.1, 4);
}
