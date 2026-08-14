use std::path::Path;
use std::path::PathBuf;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;

use super::TuiRenderer;
use super::layout::compute_fullscreen_layout;
use super::test_support::{assert_frame_fixture, first_frame_difference, snapshot_buffer};
use crate::cli::tui_v2::model::{
    UiAssistantMessage, UiCompactBoundary, UiErrorMessage, UiMessage, UiMessageBase, UiMessageId,
    UiProgressMessage, UiSystemMessage, UiSystemMessageLevel, UiThinkingBlock, UiThinkingPhase,
    UiThinkingTiming, UiTokenUsage, UiToolCall, UiToolCallState, UiToolResult, UiTurnTerminal,
    UiUserMessage,
};
use crate::cli::tui_v2::state::TuiState;

#[test]
fn tui_parity_golden_harness_detects_frame_change() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
    buffer.cell_mut(Position::new(0, 0)).expect("first cell").set_symbol("A").set_fg(Color::Green);
    let expected = snapshot_buffer(&buffer, Some(Position::new(1, 0)));

    let mut changed = expected.clone();
    changed.cells[0].symbol = "B".to_string();
    changed.cells[0].foreground = "Red".to_string();
    let difference = first_frame_difference(&expected, &changed).expect("cell difference");
    assert!(difference.contains("x=0 y=0"), "{difference}");
    assert!(difference.contains("style"), "{difference}");

    let mut moved_cursor = expected.clone();
    moved_cursor.cursor = Some(Position::new(0, 0));
    let cursor_difference =
        first_frame_difference(&expected, &moved_cursor).expect("cursor difference");
    assert!(cursor_difference.contains("cursor"), "{cursor_difference}");

    let actual = super::test_support::render_snapshot(80, 24, |frame| {
        TuiRenderer::default().render_frame(
            frame,
            &TuiState::default(),
            "VW",
            "gateway://fixture",
            0,
        );
    })
    .expect("render empty frame");
    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tui_parity/empty-80x24.frame");
    assert_frame_fixture(&fixture_path, &actual).expect("stable empty frame fixture");
}

#[test]
fn tui_main_screen_matches_reference_layout_at_supported_sizes() {
    let mut state = TuiState::default();
    state.status.model_name = Some("gpt-5.6".to_string());
    state.status.provider_name = Some("OpenAI".to_string());
    state.project.workspace_root = Some(PathBuf::from("/workspace/vibewindow"));

    for (width, height) in [(80, 24), (100, 30), (120, 40), (160, 50)] {
        let slots = compute_fullscreen_layout(Rect::new(0, 0, width, height), true, false, 0);
        assert_eq!(slots.header.height, 0, "{width}x{height} has no fixed header");
        assert!(slots.project_context.is_none(), "{width}x{height} has no project sidebar");
        assert!(slots.modified_files.is_none(), "{width}x{height} has no files sidebar");
        assert_eq!(slots.scrollable.width, width);
        assert_eq!(slots.bottom.y.saturating_add(slots.bottom.height), height);

        let actual = super::test_support::render_snapshot(width, height, |frame| {
            TuiRenderer::default().render_frame(frame, &state, "VW", "gateway://connected", 0);
        })
        .expect("render welcome frame");
        let text = snapshot_text(&actual);
        let normalized_text = text.replace(' ', "");
        assert!(normalized_text.contains("欢迎使用VibeWindow"), "{width}x{height}: {text}");
        assert!(normalized_text.contains("gpt-5.6"), "{width}x{height}: {text}");
        assert!(normalized_text.contains("/workspace/vibewindow"), "{width}x{height}: {text}");

        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/tui_parity/main-{width}x{height}.frame"));
        assert_frame_fixture(&fixture_path, &actual).expect("stable welcome frame fixture");
    }

    let mut restored = state.clone();
    restored.session.session_id = Some("restored-session".to_string());
    let restored = super::test_support::render_snapshot(80, 24, |frame| {
        TuiRenderer::default().render_frame(frame, &restored, "VW", "gateway://connected", 0);
    })
    .expect("render restored empty session");
    assert!(!snapshot_text(&restored).replace(' ', "").contains("欢迎使用VibeWindow"));
}

#[test]
fn tui_basic_message_rows_match_reference_frame() {
    let base = |id: &str, created_ms| {
        UiMessageBase::new(UiMessageId::gateway(id))
            .with_session_id("basic-message-session")
            .with_created_ms(created_ms)
    };
    let mut state = TuiState::default();
    state.session.session_id = Some("basic-message-session".to_string());
    for message in [
        UiMessage::User(UiUserMessage {
            base: base("user-1", 1_000),
            text: "第一条用户消息".to_string(),
        }),
        UiMessage::User(UiUserMessage {
            base: base("user-2", 2_000),
            text: "follow-up without extra gap".to_string(),
        }),
        UiMessage::Assistant(UiAssistantMessage {
            base: base("assistant-1", 3_000),
            text: "**完成**：支持 Markdown 和宽字符换行。".to_string(),
            usage: UiTokenUsage::default(),
            step_count: 2,
            terminal: UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) },
            model: Some("fixture-model".to_string()),
        }),
        UiMessage::Assistant(UiAssistantMessage {
            base: base("assistant-empty", 4_000),
            text: String::new(),
            usage: UiTokenUsage::default(),
            step_count: 0,
            terminal: UiTurnTerminal::Done { finish_reason: None },
            model: None,
        }),
        UiMessage::System(UiSystemMessage {
            base: base("system-info", 5_000),
            text: "verbose-only diagnostic".to_string(),
            level: UiSystemMessageLevel::Info,
        }),
        UiMessage::System(UiSystemMessage {
            base: base("system-warning", 6_000),
            text: "需要注意的系统警告".to_string(),
            level: UiSystemMessageLevel::Warning,
        }),
        UiMessage::CompactBoundary(UiCompactBoundary {
            base: base("compact", 7_000),
            summary: "Earlier context compacted".to_string(),
            compacted_messages: 12,
        }),
        UiMessage::Error(UiErrorMessage {
            base: base("error", 8_000),
            message: "permission rejected: write denied".to_string(),
            recoverable: true,
        }),
    ] {
        state.append_message(message);
    }
    state.scroll.follow_tail = false;
    state.scroll.top_message = 0;

    for (width, height) in [(80u16, 24u16), (120, 30)] {
        state.scroll.sync_viewport(height.saturating_sub(6), width);
        state.refresh_transcript_layout_for_current_width();
        let actual = super::test_support::render_snapshot(width, height, |frame| {
            TuiRenderer::default().render_frame(frame, &state, "VW", "gateway://fixture", 0);
        })
        .expect("render basic message frame");
        let text = snapshot_text(&actual);
        assert!(text.contains('❯'), "{width}x{height}: {text}");
        assert!(text.contains('●'), "{width}x{height}: {text}");
        assert!(text.contains("✻ Conversation compacted"), "{width}x{height}: {text}");
        assert!(text.contains("permission rejected"), "{width}x{height}: {text}");
        assert!(!text.contains("verbose-only diagnostic"), "{width}x{height}: {text}");
        assert!(!text.contains("助手 "), "{width}x{height}: {text}");

        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/tui_parity/basic-messages-{width}x{height}.frame"));
        assert_frame_fixture(&fixture_path, &actual).expect("stable basic message frame fixture");
    }
}

#[test]
fn tui_streaming_thinking_matches_reference_frame() {
    let mut state = TuiState::default();
    state.session.session_id = Some("thinking-session".to_string());
    state.append_message(UiMessage::User(UiUserMessage {
        base: UiMessageBase::new(UiMessageId::gateway("thinking-user"))
            .with_session_id("thinking-session")
            .with_created_ms(900),
        text: "Inspect the reducer".to_string(),
    }));
    state.append_message(UiMessage::Thinking(UiThinkingBlock {
        base: UiMessageBase::new(UiMessageId::local("thinking-1"))
            .with_session_id("thinking-session")
            .with_created_ms(1_000),
        summary: Some("Inspect state transitions".to_string()),
        content: "private chain of thought must stay collapsed".to_string(),
        timing: vec![UiThinkingTiming { start_ms: 1_000, end_ms: None, last_update_ms: 1_250 }],
        collapsed: false,
        phase: UiThinkingPhase::Streaming,
    }));
    state.runtime.last_event_time_ms = Some(1_250);
    state.scroll.follow_tail = false;
    state.scroll.top_message = 0;
    state.scroll.sync_viewport(18, 80);
    state.refresh_transcript_layout_for_current_width();

    let actual = super::test_support::render_snapshot(80, 24, |frame| {
        TuiRenderer::default().render_frame(frame, &state, "VW", "gateway://fixture", 2);
    })
    .expect("render streaming thinking frame");
    let text = snapshot_text(&actual);
    assert!(text.contains("∴ Thinking…"), "{text}");
    assert!(text.contains("- Thinking…"), "{text}");
    assert!(!text.contains("private chain of thought"), "{text}");
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tui_parity/thinking-streaming-80x24.frame");
    assert_frame_fixture(&fixture_path, &actual).expect("stable streaming thinking frame fixture");

    let UiMessage::Thinking(thinking) = &mut state.messages[1] else {
        panic!("thinking fixture message");
    };
    thinking.phase = UiThinkingPhase::Closed;
    thinking.collapsed = true;
    thinking.timing[0].end_ms = Some(1_500);
    thinking.timing[0].last_update_ms = 1_500;
    state.runtime.last_event_time_ms = Some(1_500);
    let collapsed = super::test_support::render_snapshot(80, 24, |frame| {
        TuiRenderer::default().render_frame(frame, &state, "VW", "gateway://fixture", 3);
    })
    .expect("render closed thinking frame");
    let collapsed_text = snapshot_text(&collapsed);
    assert!(collapsed_text.contains("Ctrl+O to expand"), "{collapsed_text}");
    assert!(collapsed_text.contains("Thought for 1s"), "{collapsed_text}");
    assert!(!collapsed_text.contains("private chain of thought"), "{collapsed_text}");

    state.status.verbose = true;
    let expanded = super::test_support::render_snapshot(80, 24, |frame| {
        TuiRenderer::default().render_frame(frame, &state, "VW", "gateway://fixture", 0);
    })
    .expect("render expanded historical thinking");
    assert!(snapshot_text(&expanded).contains("private chain of thought"));

    let UiMessage::Thinking(thinking) = &mut state.messages[1] else {
        panic!("thinking fixture message");
    };
    thinking.phase = UiThinkingPhase::HiddenHistory;
    let hidden = super::test_support::render_snapshot(80, 24, |frame| {
        TuiRenderer::default().render_frame(frame, &state, "VW", "gateway://fixture", 0);
    })
    .expect("render hidden historical thinking");
    assert!(!snapshot_text(&hidden).contains("private chain of thought"));
}

#[test]
fn tui_grouped_progress_and_hooks_match_reference_frame() {
    let user_id = UiMessageId::local("group-user");
    let read_id = UiMessageId::local("read-call");
    let search_id = UiMessageId::local("search-call");
    let mut state = TuiState::default();
    state.messages = vec![
        UiMessage::User(UiUserMessage {
            base: UiMessageBase::new(user_id.clone()),
            text: "inspect project".to_string(),
        }),
        UiMessage::ToolCall(UiToolCall {
            base: UiMessageBase::new(read_id.clone()).with_parent_id(user_id.clone()),
            call_id: Some("read".to_string()),
            tool_name: "read".to_string(),
            summary: Some("Read config".to_string()),
            arguments: Some(r#"{"path":"Cargo.toml"}"#.to_string()),
            state: UiToolCallState::Complete,
        }),
        UiMessage::ToolResult(UiToolResult {
            base: UiMessageBase::new(UiMessageId::local("read-result"))
                .with_parent_id(read_id.clone()),
            call_id: Some("read".to_string()),
            tool_name: "read".to_string(),
            content: "ok".to_string(),
            is_error: false,
        }),
        UiMessage::Progress(UiProgressMessage {
            base: UiMessageBase::new(UiMessageId::local("read-progress"))
                .with_parent_id(read_id.clone()),
            label: "Read lines".to_string(),
            current: Some(20),
            total: Some(20),
            completed: true,
        }),
        UiMessage::System(UiSystemMessage {
            base: UiMessageBase::new(UiMessageId::local("hook")).with_parent_id(read_id),
            text: "hook: PostToolUse passed".to_string(),
            level: UiSystemMessageLevel::Info,
        }),
        UiMessage::ToolCall(UiToolCall {
            base: UiMessageBase::new(search_id.clone()).with_parent_id(user_id.clone()),
            call_id: Some("search".to_string()),
            tool_name: "grep".to_string(),
            summary: Some("Search source".to_string()),
            arguments: Some(r#"{"pattern":"needle"}"#.to_string()),
            state: UiToolCallState::Complete,
        }),
        UiMessage::ToolResult(UiToolResult {
            base: UiMessageBase::new(UiMessageId::local("search-result"))
                .with_parent_id(search_id.clone()),
            call_id: Some("search".to_string()),
            tool_name: "grep".to_string(),
            content: "2 matches".to_string(),
            is_error: false,
        }),
        UiMessage::System(UiSystemMessage {
            base: UiMessageBase::new(UiMessageId::local("background")).with_parent_id(search_id),
            text: "background: index ready".to_string(),
            level: UiSystemMessageLevel::Info,
        }),
        UiMessage::Assistant(UiAssistantMessage {
            base: UiMessageBase::new(UiMessageId::local("group-assistant")).with_parent_id(user_id),
            text: "done".to_string(),
            usage: UiTokenUsage::default(),
            step_count: 0,
            terminal: UiTurnTerminal::Done { finish_reason: None },
            model: None,
        }),
    ];
    state.refresh_transcript_projection();
    state.scroll.follow_tail = false;
    state.scroll.top_message = 0;
    state.scroll.sync_viewport(18, 80);
    state.refresh_transcript_layout_for_current_width();

    let collapsed = super::test_support::render_snapshot(80, 24, |frame| {
        TuiRenderer::default().render_frame(frame, &state, "VW", "gateway://fixture", 0);
    })
    .expect("render grouped tools");
    let text = snapshot_text(&collapsed);
    let normalized = text.replace(' ', "");
    assert!(text.contains("explore results: read x1, grep x1"), "{text}");
    assert!(normalized.contains("1条进度"), "{text}");
    assert!(normalized.contains("2条通知"), "{text}");
    assert!(!text.contains("PostToolUse passed"), "{text}");
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tui_parity/grouped-tools-80x24.frame");
    assert_frame_fixture(&fixture_path, &collapsed).expect("stable grouped tools frame");

    state.status.verbose = true;
    let expanded = super::test_support::render_snapshot(80, 24, |frame| {
        TuiRenderer::default().render_frame(frame, &state, "VW", "gateway://fixture", 0);
    })
    .expect("render expanded tools");
    let expanded_text = snapshot_text(&expanded);
    assert!(expanded_text.contains("PostToolUse passed"), "{expanded_text}");
    assert!(expanded_text.contains("background: index ready"), "{expanded_text}");
}

#[test]
fn tui_virtual_scroll_uses_measured_heights_and_preserves_anchor() {
    let mut state = TuiState::default();
    state.session.session_id = Some("virtual-scroll".to_string());
    state.messages = (0..24)
        .map(|index| {
            UiMessage::User(UiUserMessage {
                base: UiMessageBase::new(UiMessageId::local(format!("virtual-{index}"))),
                text: if index % 3 == 0 {
                    format!("message {index} {}", "wrapped content ".repeat(12))
                } else {
                    format!("message {index}")
                },
            })
        })
        .collect();
    state.refresh_transcript_projection();
    state.scroll.follow_tail = false;
    state.scroll.top_message = 9;
    state.scroll.overscan = 1;
    state.scroll.sync_viewport(8, 38);
    state.refresh_transcript_layout_for_current_width();

    let mut feedback = super::TuiRenderFeedback::default();
    super::test_support::render_snapshot(40, 14, |frame| {
        feedback = TuiRenderer::default().render_frame(frame, &state, "VW", "gateway://fixture", 0);
    })
    .expect("render measured virtual window");
    assert!(
        !feedback.transcript_measurements.items.is_empty(),
        "real renderer reports visible item heights"
    );
    assert!(
        feedback.transcript_measurements.items.len() < state.messages.len(),
        "renderer only measures the bounded virtual window"
    );
    state.apply_transcript_measurements(&feedback.transcript_measurements);

    let window = crate::cli::tui_v2::state::select_visible_grouped_transcript_window(&state);
    assert!(window.top_spacer_rows > 0);
    assert!(window.bottom_spacer_rows > 0);
    assert!(window.len() <= 6, "overscan stays bounded: {}", window.len());
    let anchor_id = state.messages[state.scroll.top_message].base().id.clone();
    let stable_measurements = state.transcript_layout.measured_item_count(38, false);

    state.prepend_messages_preserving_anchor(vec![
        UiMessage::User(UiUserMessage {
            base: UiMessageBase::new(UiMessageId::local("prepended-a")),
            text: "older a".to_string(),
        }),
        UiMessage::User(UiUserMessage {
            base: UiMessageBase::new(UiMessageId::local("prepended-b")),
            text: "older b".to_string(),
        }),
    ]);
    assert_eq!(state.messages[state.scroll.top_message].base().id, anchor_id);
    assert_eq!(
        state.transcript_layout.measured_item_count(38, false),
        stable_measurements,
        "prepend keeps stable ID measurements"
    );

    state.scroll.sync_viewport(8, 18);
    state.refresh_transcript_layout_for_current_width();
    assert!(state.transcript_layout.bucket_count() >= 2, "width versions coexist");

    state.status.verbose = true;
    let mut expanded_feedback = super::TuiRenderFeedback::default();
    super::test_support::render_snapshot(20, 14, |frame| {
        expanded_feedback =
            TuiRenderer::default().render_frame(frame, &state, "VW", "gateway://fixture", 0);
    })
    .expect("render expanded virtual window");
    state.apply_transcript_measurements(&expanded_feedback.transcript_measurements);
    assert!(
        state.transcript_layout.measured_item_count(19, true) > 0,
        "expanded cards have an independent height version"
    );

    let manual_anchor = state.messages[state.scroll.top_message].base().id.clone();
    state.append_message(UiMessage::User(UiUserMessage {
        base: UiMessageBase::new(UiMessageId::local("manual-tail")),
        text: "manual tail".to_string(),
    }));
    assert_eq!(state.messages[state.scroll.top_message].base().id, manual_anchor);

    state.scroll.follow_tail = true;
    state.append_message(UiMessage::User(UiUserMessage {
        base: UiMessageBase::new(UiMessageId::local("followed-tail")),
        text: "followed tail".to_string(),
    }));
    assert_eq!(state.scroll.top_message, state.messages.len().saturating_sub(1));
}

#[test]
fn tui_sticky_unread_chrome_matches_reference_frame() {
    let user_id = UiMessageId::local("sticky-frame-user");
    let mut state = TuiState::default();
    state.session.session_id = Some("sticky-frame".to_string());
    state.messages = vec![
        UiMessage::User(UiUserMessage {
            base: UiMessageBase::new(user_id.clone()),
            text: "Inspect the scroll contract".to_string(),
        }),
        UiMessage::Assistant(UiAssistantMessage {
            base: UiMessageBase::new(UiMessageId::local("sticky-frame-answer"))
                .with_parent_id(user_id.clone()),
            text: "Initial answer".to_string(),
            usage: UiTokenUsage::default(),
            step_count: 0,
            terminal: UiTurnTerminal::Done { finish_reason: None },
            model: None,
        }),
    ];
    state.refresh_transcript_projection();
    state.scroll.sync_viewport(12, 78);
    state.clamp_scroll();
    let mut parked = state.scroll.clone();
    parked.follow_tail = false;
    parked.top_message = 1;
    crate::cli::tui_v2::state::reduce_tui_state(
        &mut state,
        crate::cli::tui_v2::state::TuiAction::ScrollSet(parked),
    );
    state.append_message(UiMessage::ToolCall(UiToolCall {
        base: UiMessageBase::new(UiMessageId::local("sticky-frame-tool")).with_parent_id(user_id),
        call_id: Some("sticky-frame-tool".to_string()),
        tool_name: "read".to_string(),
        summary: Some("Read scroll state".to_string()),
        arguments: None,
        state: UiToolCallState::Running,
    }));

    let actual = super::test_support::render_snapshot(80, 24, |frame| {
        TuiRenderer::default().render_frame(frame, &state, "VW", "gateway://fixture", 0);
    })
    .expect("render sticky unread frame");
    let text = snapshot_text(&actual).replace(' ', "");
    assert!(text.contains("上一条输入"), "{text}");
    assert!(text.contains("1条新消息"), "{text}");
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tui_parity/sticky-unread-80x24.frame");
    assert_frame_fixture(&fixture_path, &actual).expect("stable sticky unread frame");
}

#[test]
fn tui_transcript_reading_mode_matches_reference_frame() {
    let mut state = TuiState::default();
    state.transcript_mode = crate::cli::tui_v2::model::TranscriptModeState {
        active: true,
        lines: vec![
            "❯ Inspect the transcript contract".to_string(),
            "TOOL read visible summary".to_string(),
            "RESULT read 24 lines".to_string(),
            "● The reading mode freezes visible output.".to_string(),
        ],
        viewport_rows: 18,
        search_query: "read".to_string(),
        search_matches: vec![1, 2],
        selected_match: Some(0),
        ..Default::default()
    };

    let actual = super::test_support::render_snapshot(80, 24, |frame| {
        TuiRenderer::default().render_frame(frame, &state, "VW", "gateway://fixture", 0);
    })
    .expect("render transcript mode");
    let text = snapshot_text(&actual);
    let normalized = text.replace(' ', "");
    assert!(normalized.contains("Transcript阅读模式"), "{text}");
    assert!(normalized.contains("匹配1/2"), "{text}");
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tui_parity/transcript-mode-80x24.frame");
    assert_frame_fixture(&fixture_path, &actual).expect("stable transcript mode frame");
}

#[test]
fn tui_prompt_footer_states_match_reference_frames() {
    let mut empty = TuiState::default();
    empty.session.session_id = Some("prompt-frame".to_string());
    empty.status.model_name = Some("gpt-5.6".to_string());
    empty.project.workspace_root = Some(PathBuf::from("/workspace/vibewindow"));

    let mut multiline = empty.clone();
    multiline.prompt.set_value("first line\n第二行");

    let mut busy = empty.clone();
    busy.prompt
        .start_submission(crate::cli::tui_v2::model::PromptSubmission::new("running request"));

    let mut search = empty.clone();
    search.prompt.history.entries =
        vec!["cargo clippy".to_string(), "cargo test focused".to_string()];
    search.history_search.active = true;
    search.history_search.query = "cargo".to_string();
    search.history_search.matches = vec![0, 1];
    search.history_search.selected_index = Some(1);

    for (name, state) in [
        ("prompt-empty-60x16.frame", empty),
        ("prompt-multiline-60x16.frame", multiline),
        ("prompt-busy-60x16.frame", busy),
        ("prompt-history-search-60x16.frame", search),
    ] {
        let actual = super::test_support::render_snapshot(60, 16, |frame| {
            TuiRenderer::default().render_frame(frame, &state, "VW", "gateway://fixture", 0);
        })
        .expect("render prompt state");
        let text = snapshot_text(&actual).replace(' ', "");
        assert!(text.contains('❯'), "{name}: {text}");
        assert!(text.contains("模式"), "{name}: {text}");
        let fixture_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tui_parity").join(name);
        assert_frame_fixture(&fixture_path, &actual).expect("stable prompt state frame");
    }
}

#[test]
fn tui_claude_code_2_1_88_parity_acceptance() {
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let contract_path = repository_root.join("docs/tui-parity/claude-code-2.1.88-contract.md");
    let contract = std::fs::read_to_string(&contract_path).expect("read frozen TUI contract");
    let rows = contract
        .lines()
        .filter(|line| {
            line.strip_prefix("| ").and_then(|line| line.split_once(' ')).is_some_and(|(id, _)| {
                id.len() == 3
                    && id.starts_with(|character: char| character.is_ascii_uppercase())
                    && id[1..].chars().all(|character| character.is_ascii_digit())
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 56, "all frozen contract rows remain auditable");

    let blocked =
        rows.iter().filter(|row| row.contains("| 后端阻塞 |")).copied().collect::<Vec<_>>();
    assert!(blocked.is_empty(), "backend-blocked contract rows remain: {blocked:#?}");

    let unresolved = rows
        .iter()
        .filter(|row| {
            ["缺少", "缺 ", "尚无", "未实现", "待对齐", "待校准"]
                .iter()
                .any(|marker| row.contains(marker))
        })
        .copied()
        .collect::<Vec<_>>();
    assert!(unresolved.is_empty(), "unresolved contract wording remains: {unresolved:#?}");

    let report_path = repository_root.join("docs/tui-parity/acceptance-report.md");
    let report = std::fs::read_to_string(&report_path).expect("read current acceptance report");
    for row in &rows {
        let id = row.trim_start_matches("| ").split('|').next().expect("contract ID").trim();
        assert!(report.contains(&format!("`{id}`")), "acceptance report omits {id}");
    }

    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tui_parity");
    let fixture_count = std::fs::read_dir(&fixture_root)
        .expect("read parity fixtures")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|extension| extension == "frame"))
        .count();
    assert_eq!(fixture_count, 15, "the current golden-frame set is complete");
    for size in ["80x24", "100x30", "120x40", "160x50"] {
        assert!(
            fixture_root.join(format!("main-{size}.frame")).is_file(),
            "missing four-size golden frame {size}"
        );
    }
}

fn snapshot_text(snapshot: &super::test_support::FrameSnapshot) -> String {
    snapshot
        .cells
        .chunks(usize::from(snapshot.width))
        .map(|row| row.iter().map(|cell| cell.symbol.as_str()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
