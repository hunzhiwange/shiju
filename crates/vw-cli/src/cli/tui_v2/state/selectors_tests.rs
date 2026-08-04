use super::{
    TuiAssistantTurnEntry, TuiTranscriptItem, derive_search_matches,
    derive_search_matches_with_cache, derive_search_text_cache, select_grouped_transcript,
    select_search_matches, select_status_summary, select_transcript_message_anchors,
    select_visible_grouped_transcript_window, select_visible_message_window,
};
use crate::cli::tui_v2::controller::{TuiController, TuiControllerEvent};
use crate::cli::tui_v2::model::{
    UiAssistantMessage, UiMessage, UiMessageBase, UiMessageId, UiOverlay, UiProgressMessage,
    UiSearchOverlay, UiStep, UiStepState, UiSystemMessage, UiSystemMessageLevel, UiThinkingBlock,
    UiThinkingPhase, UiThinkingTiming, UiTokenUsage, UiToolCall, UiToolCallState, UiToolResult,
    UiTurnTerminal, UiUserMessage,
};
use crate::cli::tui_v2::state::{TuiAction, reduce_tui_state};
use crate::cli::tui_v2::state::{TuiScrollState, TuiState};
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};

fn base(seed: &str) -> UiMessageBase {
    UiMessageBase::new(UiMessageId::local(seed))
}

fn user(seed: &str, text: &str) -> UiMessage {
    UiMessage::User(UiUserMessage { base: base(seed), text: text.to_string() })
}

fn assistant(seed: &str, parent: Option<UiMessageId>, text: &str) -> UiMessage {
    let mut base = base(seed);
    base.parent_id = parent;
    UiMessage::Assistant(UiAssistantMessage {
        base,
        text: text.to_string(),
        usage: UiTokenUsage::default(),
        step_count: 0,
        terminal: UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) },
        model: Some("model-a".to_string()),
    })
}

fn tool_call(seed: &str, name: &str, parent: Option<UiMessageId>) -> UiMessage {
    let mut base = base(seed);
    base.parent_id = parent;
    UiMessage::ToolCall(UiToolCall {
        base,
        call_id: None,
        tool_name: name.to_string(),
        summary: Some(format!("{name} summary")),
        arguments: Some(format!("{name} args")),
        state: UiToolCallState::Complete,
    })
}

fn tool_result(seed: &str, name: &str, parent: Option<UiMessageId>, error: bool) -> UiMessage {
    let mut base = base(seed);
    base.parent_id = parent;
    UiMessage::ToolResult(UiToolResult {
        base,
        call_id: None,
        tool_name: name.to_string(),
        content: format!("{name} result"),
        is_error: error,
    })
}

#[test]
fn tui_transcript_groups_tools_progress_and_hooks_like_reference() {
    let user_id = UiMessageId::local("group-user");
    let read_id = UiMessageId::local("read-call");
    let search_id = UiMessageId::local("search-call");
    let mut state = TuiState::default();
    state.messages = vec![
        UiMessage::User(UiUserMessage {
            base: UiMessageBase::new(user_id.clone()),
            text: "inspect".to_string(),
        }),
        tool_call("read-call", "read", Some(user_id.clone())),
        tool_result("read-result", "read", Some(read_id.clone()), false),
        UiMessage::Progress(UiProgressMessage {
            base: base("read-progress").with_parent_id(read_id.clone()),
            label: "Read 20 lines".to_string(),
            current: Some(20),
            total: Some(20),
            completed: true,
        }),
        UiMessage::System(UiSystemMessage {
            base: base("read-hook").with_parent_id(read_id),
            text: "hook: PostToolUse passed".to_string(),
            level: UiSystemMessageLevel::Info,
        }),
        tool_call("search-call", "grep", Some(user_id.clone())),
        tool_result("search-result", "grep", Some(search_id.clone()), false),
        UiMessage::System(UiSystemMessage {
            base: base("background").with_parent_id(search_id),
            text: "background: search index ready".to_string(),
            level: UiSystemMessageLevel::Info,
        }),
        assistant("group-assistant", Some(user_id), "done"),
    ];
    let original = state.messages.clone();
    state.refresh_transcript_projection();

    assert_eq!(state.messages, original, "projection never reorders persisted messages");
    let transcript = select_grouped_transcript(&state);
    let TuiTranscriptItem::AssistantTurn(turn) = &transcript[1] else {
        panic!("assistant turn");
    };
    assert_eq!(turn.preface.len(), 1);
    let TuiAssistantTurnEntry::CollapsedTools(group) = &turn.preface[0] else {
        panic!("successful read/search run should collapse");
    };
    assert_eq!(group.calls.len(), 2);
    assert_eq!(group.total_results, 2);
    assert_eq!(group.total_progress, 1);
    assert_eq!(group.notice_count, 2);
    assert_eq!(group.calls[0].progress[0].label, "Read 20 lines");
    assert_eq!(group.calls[0].notices[0].text, "hook: PostToolUse passed");
    assert_eq!(group.calls[1].notices[0].text, "background: search index ready");

    let keys = state.transcript.keys();
    assert_eq!(keys.len(), 2);
    assert!(keys[1].contains("local:read-call"));
    assert!(keys[1].contains("local:search-call"));

    let mut running = original;
    let UiMessage::ToolCall(read) = &mut running[1] else {
        panic!("read call");
    };
    read.state = UiToolCallState::Running;
    running.remove(2);
    let running_transcript = super::derive_grouped_transcript(&running);
    let TuiTranscriptItem::AssistantTurn(turn) = &running_transcript[1] else {
        panic!("running assistant turn");
    };
    assert!(
        turn.preface.iter().any(
            |entry| matches!(entry, TuiAssistantTurnEntry::Tool(group) if group.call.state == UiToolCallState::Running)
        ),
        "running tools stay expanded"
    );
}

#[test]
fn visible_message_window_handles_empty_zero_viewport_and_overscan() {
    let mut state = TuiState::default();
    assert!(select_visible_message_window(&state).is_empty());

    state.messages = vec![user("u1", "one"), user("u2", "two"), user("u3", "three")];
    state.scroll = TuiScrollState {
        top_message: 1,
        viewport_messages: 0,
        viewport_height: 0,
        viewport_width: 80,
        overscan: 1,
        follow_tail: false,
        sticky_message: None,
        last_seen_message: None,
    };
    assert!(select_visible_message_window(&state).is_empty());

    state.scroll.viewport_messages = 1;
    assert_eq!(select_visible_message_window(&state).len(), 3);
}

#[test]
fn tui_sticky_prompt_and_unseen_divider_follow_reference_scroll_rules() {
    let first_user = user("sticky-u1", "first prompt");
    let first_user_id = first_user.id().clone();
    let first_assistant = assistant("sticky-a1", Some(first_user_id.clone()), "first answer");
    let mut state = TuiState::default();
    state.session.session_id = Some("sticky-session".to_string());
    state.messages = vec![first_user, first_assistant];
    state.refresh_transcript_projection();
    state.scroll.sync_viewport(6, 40);
    state.clamp_scroll();
    assert!(state.scroll.follow_tail);

    let mut parked = state.scroll.clone();
    parked.follow_tail = false;
    parked.top_message = 1;
    reduce_tui_state(&mut state, TuiAction::ScrollSet(parked));
    let baseline = UiMessageId::local("sticky-a1");
    assert_eq!(state.unseen_baseline_id.as_ref(), Some(&baseline));

    let call_id = UiMessageId::local("sticky-tool");
    state.append_message(tool_call("sticky-tool", "read", Some(first_user_id.clone())));
    state.append_message(tool_result("sticky-result", "read", Some(call_id), false));
    let tool_only = select_visible_grouped_transcript_window(&state)
        .unseen_range()
        .expect("tool-only stream still exposes unread activity");
    assert_eq!(tool_only.unseen_message_count, 1);
    assert_eq!(tool_only.pill_label(), "1 new message");
    assert_eq!(
        select_visible_grouped_transcript_window(&state)
            .sticky_prompt()
            .map(|prompt| prompt.message_index),
        Some(0)
    );

    state.append_message(user("sticky-u2", "second prompt"));
    state.append_message(assistant(
        "sticky-a2",
        Some(UiMessageId::local("sticky-u2")),
        "second answer",
    ));
    let unread = select_visible_grouped_transcript_window(&state)
        .unseen_range()
        .expect("assistant turn increments semantic unread count");
    assert_eq!(unread.unseen_message_count, 2);

    state.prepend_messages_preserving_anchor(vec![user("sticky-old", "older history")]);
    assert_eq!(state.unseen_baseline_id.as_ref(), Some(&baseline));
    assert_eq!(
        select_visible_grouped_transcript_window(&state)
            .unseen_range()
            .map(|range| range.unseen_message_count),
        Some(2),
        "prepend does not become unread activity"
    );

    let mut controller = TuiController::default();
    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))),
    );
    assert!(matches!(command, crate::cli::tui_v2::controller::TuiControllerCommand::Continue));
    assert!(state.scroll.follow_tail);
    assert!(state.unseen_baseline_id.is_none());
    assert!(select_visible_grouped_transcript_window(&state).unseen_range().is_none());

    state.clear_messages();
    assert!(state.unseen_baseline_id.is_none());
    assert!(state.scroll.last_seen_message.is_none());
}

#[test]
fn grouped_transcript_attaches_preface_and_collapses_explore_tools() {
    let user = user("u1", "prompt");
    let assistant = assistant("a1", Some(user.id().clone()), "answer");
    let read = tool_call("read-call", "read", Some(assistant.id().clone()));
    let read_result = tool_result("read-result", "read", Some(read.id().clone()), false);
    let grep = tool_call("grep-call", "grep", Some(assistant.id().clone()));
    let grep_result = tool_result("grep-result", "grep", Some(grep.id().clone()), false);
    let messages = vec![user, assistant, read, read_result, grep, grep_result];

    let mut state = TuiState::default();
    state.messages = messages;
    state.refresh_transcript_projection();
    let grouped = select_grouped_transcript(&state);

    assert_eq!(select_transcript_message_anchors(&state), vec![0, 1]);
    let TuiTranscriptItem::AssistantTurn(turn) = &grouped[1] else {
        panic!("assistant turn expected");
    };
    assert_eq!(turn.children.len(), 1);
    let TuiAssistantTurnEntry::CollapsedTools(batch) = &turn.children[0] else {
        panic!("explore tools should collapse");
    };
    assert_eq!(batch.total_results, 2);
    assert_eq!(batch.summary, "explore results: read x1, grep x1");
}

#[test]
fn failed_single_explore_tool_is_not_collapsed() {
    let user = user("u1", "prompt");
    let assistant = assistant("a1", Some(user.id().clone()), "answer");
    let read = tool_call("read-call", "read", Some(assistant.id().clone()));
    let read_result = tool_result("read-result", "read", Some(read.id().clone()), true);
    let mut state = TuiState::default();
    state.messages = vec![user, assistant, read, read_result];
    state.refresh_transcript_projection();

    let grouped = select_grouped_transcript(&state);
    let TuiTranscriptItem::AssistantTurn(turn) = &grouped[1] else {
        panic!("assistant turn expected");
    };
    assert!(matches!(turn.children[0], TuiAssistantTurnEntry::Tool(_)));
}

#[test]
fn status_summary_prefers_step_usage_over_assistant_usage() {
    let mut state = TuiState::default();
    state.session.session_id = Some("s1".to_string());
    state.status.session_title = "Title".to_string();
    let mut assistant = assistant("a1", None, "answer");
    if let UiMessage::Assistant(message) = &mut assistant {
        message.usage.output_tokens = 99;
    }
    state.messages = vec![
        assistant,
        UiMessage::Step(UiStep {
            base: base("step1"),
            step_index: 1,
            started_ms: 10,
            finished_ms: Some(20),
            usage: UiTokenUsage {
                input_tokens: 1,
                output_tokens: 2,
                cached_tokens: 3,
                reasoning_tokens: 4,
            },
            finish_reason: Some("stop".to_string()),
            model: Some("model-a".to_string()),
            state: UiStepState::Complete,
        }),
    ];

    let summary = select_status_summary(&state);

    assert_eq!(summary.session_id.as_deref(), Some("s1"));
    assert_eq!(summary.title, "Title");
    assert_eq!(summary.assistant_message_count, 1);
    assert_eq!(summary.step_count, 1);
    assert_eq!(summary.token_usage.output_tokens, 2);
}

#[test]
fn search_matches_use_cache_fallback_case_modes_and_overlay() {
    let messages = vec![
        user("u1", "Alpha beta alpha"),
        UiMessage::System(UiSystemMessage {
            base: base("sys"),
            text: "系统消息 Alpha".to_string(),
            level: UiSystemMessageLevel::Info,
        }),
    ];
    let cache = derive_search_text_cache(&messages);

    assert_eq!(derive_search_matches_with_cache(&messages, &cache, "alpha", false).len(), 3);
    assert_eq!(derive_search_matches(&messages, "Alpha", true).len(), 2);
    assert!(derive_search_matches(&messages, "  ", false).is_empty());

    let mut state = TuiState::default();
    state.messages = messages;
    state.refresh_search_index();
    assert!(select_search_matches(&state).is_empty());
    state.overlays.push(UiOverlay::Search(UiSearchOverlay {
        query: "beta".to_string(),
        ..UiSearchOverlay::default()
    }));
    assert_eq!(select_search_matches(&state).len(), 1);
}

#[test]
fn visible_grouped_window_reports_summaries_sticky_prompt_and_unseen_range() {
    let first_user = user("u1", "first prompt line that is deliberately long enough to preview");
    let assistant = assistant("a1", Some(first_user.id().clone()), "answer");
    let mut state = TuiState::default();
    state.messages = vec![
        first_user,
        UiMessage::Thinking(UiThinkingBlock {
            base: base("t1"),
            summary: Some("thinking summary".to_string()),
            content: "thought".to_string(),
            timing: vec![UiThinkingTiming { start_ms: 1, end_ms: Some(2), last_update_ms: 2 }],
            collapsed: false,
            phase: UiThinkingPhase::default(),
        }),
        assistant,
        user("u2", "new prompt"),
    ];
    state.refresh_transcript_projection();
    state.scroll = TuiScrollState {
        top_message: 2,
        viewport_messages: 1,
        viewport_height: 1,
        viewport_width: 12,
        overscan: 0,
        follow_tail: false,
        sticky_message: Some(0),
        last_seen_message: Some(0),
    };
    state.refresh_transcript_layout_for_current_width();

    let window = select_visible_grouped_transcript_window(&state);
    let viewport = window.viewport_summary();
    let summary = window.window_summary();

    assert!(!window.is_empty());
    assert_eq!(viewport.label(), "1rows/1messages");
    assert!(summary.has_sticky_anchor());
    assert!(!summary.follows_tail());
    assert_eq!(summary.sticky_label(), "sticky m0");
    assert!(summary.coverage_label().contains("items"));
    assert_eq!(window.sticky_prompt().map(|prompt| prompt.label()), Some("prompt m0".to_string()));
    assert_eq!(
        window.unseen_range().map(|range| range.pill_label()),
        Some("3 new messages".to_string())
    );
}

#[test]
fn visible_items_exclude_leading_overscan_before_viewport_top() {
    let mut state = TuiState::default();
    state.messages = vec![user("u1", "one"), user("u2", "two"), user("u3", "three")];
    state.refresh_transcript_projection();
    state.scroll = TuiScrollState {
        top_message: 1,
        viewport_messages: 1,
        viewport_height: 1,
        viewport_width: 80,
        overscan: 1,
        follow_tail: false,
        sticky_message: None,
        last_seen_message: None,
    };

    let window = select_visible_grouped_transcript_window(&state);
    let visible = window.visible_items();

    assert_eq!(window.start_item_index, 0);
    assert_eq!(window.top_item_index, 1);
    assert!(matches!(
        visible.first(),
        Some(TuiTranscriptItem::Standalone(UiMessage::User(message))) if message.text == "two"
    ));
}

#[test]
fn follow_tail_backfills_the_viewport_instead_of_hiding_previous_rows() {
    let mut state = TuiState::default();
    state.messages = vec![user("u1", "one"), user("u2", "two"), user("u3", "three")];
    state.refresh_transcript_projection();
    state.scroll.sync_viewport(3, 80);
    state.refresh_transcript_layout_for_current_width();
    state.clamp_scroll();

    let window = select_visible_grouped_transcript_window(&state);

    assert!(window.follow_tail);
    assert!(
        window.top_item_index < 2,
        "tail view must include the rows immediately before the final message"
    );
}

#[test]
fn search_excerpt_preserves_unicode_boundaries_around_long_context() {
    let text = format!("{}🙂{}目标a{}", "x".repeat(30), "界".repeat(7), "界".repeat(20));
    let messages = vec![user("u1", &text)];

    let matches = derive_search_matches(&messages, "目标", true);

    assert_eq!(matches.len(), 1);
    assert!(matches[0].preview.starts_with("..."));
    assert!(matches[0].preview.contains("[目标]"));
    assert!(matches[0].preview.ends_with("..."));
}
