use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use vw_config_types::skills::SkillsDirectoryProvider;

use super::{
    TuiPill, TuiRenderer, TuiTone, build_modal_host, build_modified_files_host,
    build_project_context_host, build_prompt_host, build_status_footer, build_status_header,
    clip_sidebar_body_lines, compact_path_label, last_submission_pill, overlay_kind_label,
    prompt_cursor_line_column, prompt_line_prefix, prompt_mode_label, prompt_shortcut_hint,
    queue_command_pill, render_prompt_value_lines, sidebar_lines, step_state_label,
    step_timing_line, terminal_label, terminal_tone, tone_color, truncate_label, truncate_start,
    usize_to_u16, visible_prompt_suggestion_window, zh_sticky_notice, zh_unseen_divider_label,
    zh_unseen_pill_label, zh_window_sticky_label,
};
use crate::cli::session::GitWorkspaceStatus;
use crate::cli::tui_v2::completion::{CompletionCatalogItem, CompletionKind};
use crate::cli::tui_v2::editor::EditorSelection;
use crate::cli::tui_v2::model::integration::{IntegrationItem, IntegrationKind, IntegrationPanel};
use crate::cli::tui_v2::model::overlay::{UiCommandPaletteOverlay, UiPaletteItem};
use crate::cli::tui_v2::model::session_mode::SessionExecutionSnapshot;
use crate::cli::tui_v2::model::{
    McpServerTransport, PromptMode, PromptSubmission, PromptSubmissionStatus, QueuedPromptCommand,
    QueuedPromptCommandKind, UiConfirmIntent, UiConfirmOverlay, UiErrorOverlay, UiMcpOverlay,
    UiMcpServerInfo, UiMemoryEntry, UiMemoryOverlay, UiMessage, UiMessageBase, UiMessageId,
    UiOverlay, UiOverlayKind, UiQuestionOption, UiQuestionOverlay, UiQuestionPrompt,
    UiQuestionToolContext, UiSearchMatch, UiSearchOverlay, UiStepState, UiTaskOverlay,
    UiTaskStepItem, UiTodoItem, UiTodoOverlay, UiTokenUsage, UiTurnTerminal, UiUserMessage,
};
use crate::cli::tui_v2::render::test_support::render_tui_snapshot_with_theme;
use crate::cli::tui_v2::state::{
    TuiAction, TuiState, TuiStatusSummary, TuiUnseenRangeSummary, TuiWindowSummary,
    reduce_tui_state, select_status_summary, select_visible_grouped_transcript_window,
};
use crate::cli::tui_v2::terminal_capabilities::TerminalCapabilities;

#[test]
fn prompt_footer_renders_below_border_without_background_fills() {
    let mut state = TuiState::default();
    state.session.session_id = Some("render-test".to_string());
    state.status.model_name = Some("gpt-test".to_string());
    state.session_options.acp_backend = Some("codex".to_string());
    state.completion_catalog.items = vec![CompletionCatalogItem {
        replacement: "@src/".to_string(),
        label: "@src/".to_string(),
        detail: Some("/workspace/project".to_string()),
        kind: CompletionKind::Directory,
        score: 100,
    }];
    state.prompt.set_value("@");
    state.prompt.set_selected_suggestion_index(Some(0));

    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut feedback = None;
    terminal
        .draw(|frame| {
            feedback = Some(TuiRenderer::default().render_frame(frame, &state, "TUI", "local", 0));
        })
        .expect("render prompt footer");
    let feedback = feedback.expect("render feedback");
    let buffer = terminal.backend().buffer();
    let row_text = |y| {
        (buffer.area.left()..buffer.area.right())
            .filter_map(|x| buffer.cell((x, y)))
            .map(|cell| cell.symbol())
            .collect::<String>()
    };
    let rows = (buffer.area.top()..buffer.area.bottom()).map(row_text).collect::<Vec<_>>();
    let input_border_y = rows
        .iter()
        .rposition(|row| row.contains('└'))
        .map(|y| buffer.area.y.saturating_add(u16::try_from(y).unwrap_or(u16::MAX)))
        .expect("prompt input bottom border");
    let status_y = rows
        .iter()
        .position(|row| row.replace(' ', "").contains("完全访问已启用"))
        .map(|y| buffer.area.y.saturating_add(u16::try_from(y).unwrap_or(u16::MAX)))
        .unwrap_or_else(|| panic!("full-access status row:\n{}", rows.join("\n")));
    let shortcut_y = rows
        .iter()
        .position(|row| {
            ["Shift+Enter", "/model", "/acp", "$"].iter().all(|hint| row.contains(hint))
        })
        .map(|y| buffer.area.y.saturating_add(u16::try_from(y).unwrap_or(u16::MAX)))
        .expect("shortcut row");

    assert!(input_border_y >= feedback.layout.bottom.y);
    assert!(status_y > input_border_y, "status must render after the input border");
    assert!(shortcut_y > status_y, "shortcut hint must render after the status row");
    assert!(
        buffer.content().iter().all(|cell| cell.bg == Color::Reset),
        "root, panels, pills, and selected completions must not fill backgrounds"
    );
    assert!(
        rows.iter().any(|row| row.contains("> [D] src/")),
        "selected completion keeps an explicit marker without relying on a background"
    );
}

#[test]
fn transcript_scrollbar_is_visible_and_tracks_viewport() {
    let mut state = TuiState::default();
    state.transcript_mode.active = true;
    state.transcript_mode.lines = (1..=40).map(|line| format!("line {line}")).collect();
    state.transcript_mode.viewport_rows = 8;

    let top = rendered_scrollbar_thumb_rows(&state, 48, 18);
    assert!(top.len() >= 2, "overflowing transcript needs a visible solid thumb");
    state.transcript_mode.top_line = 16;
    let middle = rendered_scrollbar_thumb_rows(&state, 48, 18);
    state.transcript_mode.top_line = state.transcript_mode.max_top_line();
    let tail = rendered_scrollbar_thumb_rows(&state, 48, 18);
    assert!(
        top.first() < middle.first() && middle.first() < tail.first(),
        "thumb must move from top through middle to tail"
    );

    state.transcript_mode.lines.truncate(6);
    state.transcript_mode.top_line = 0;
    assert!(
        rendered_scrollbar_thumb_rows(&state, 48, 18).is_empty(),
        "non-overflowing transcript must not draw a fake thumb"
    );
    assert!(
        rendered_scrollbar_thumb_rows(&state, 3, 3).is_empty(),
        "tiny terminals must hide the scrollbar safely"
    );

    let mut main_state = TuiState::default();
    for index in 0..30 {
        reduce_tui_state(
            &mut main_state,
            TuiAction::MessagePushed(UiMessage::User(UiUserMessage {
                base: UiMessageBase::new(UiMessageId::local(format!("scroll-{index}"))),
                text: format!("message {index}"),
            })),
        );
    }
    main_state.scroll.sync_viewport(8, 47);
    main_state.scroll.follow_tail = false;
    main_state.scroll.top_message = 10;
    assert!(
        rendered_scrollbar_thumb_rows(&main_state, 48, 18).len() >= 2,
        "the main message viewport uses the same visible scrollbar"
    );
}

fn rendered_scrollbar_thumb_rows(state: &TuiState, width: u16, height: u16) -> Vec<u16> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let renderer = super::TuiRenderer::for_capabilities(TerminalCapabilities::default(), false);
    terminal
        .draw(|frame| {
            renderer.render_frame(frame, state, "TUI", "local", 0);
        })
        .expect("render scrollbar fixture");
    let buffer = terminal.backend().buffer();
    let area = buffer.area;
    (area.top()..area.bottom())
        .filter(|y| {
            (area.left()..area.right())
                .any(|x| buffer.cell((x, *y)).is_some_and(|cell| cell.symbol() == "█"))
        })
        .collect()
}

#[test]
fn skill_overlay_renders_current_platform_and_platform_picker() {
    let mut state = TuiState::default();
    let mut panel = IntegrationPanel::new(IntegrationKind::Skill);
    panel.finish_refresh(Ok(vec![IntegrationItem {
        id: "review".to_string(),
        title: "Review".to_string(),
        detail: "Installed skill".to_string(),
        enabled: true,
        actions: Vec::new(),
    }]));
    state.overlays.push(UiOverlay::Integration(panel));

    let host = build_modal_host(&state).expect("skill overlay");
    assert!(host.body_lines.iter().any(|line| line.contains("平台 · VibeWindow")));
    assert!(
        host.body_lines
            .iter()
            .any(|line| line.contains("输入 `$` 选择技能；`/skills` 管理平台与启停"))
    );

    let Some(UiOverlay::Integration(panel)) = state.overlays.stack.last_mut() else {
        panic!("skill overlay remains active");
    };
    assert!(panel.open_skill_provider_picker());
    let picker = build_modal_host(&state).expect("skill provider picker");
    assert_eq!(picker.title, "技能平台");
    for provider in SkillsDirectoryProvider::ALL {
        assert!(picker.body_lines.iter().any(|line| line.contains(provider.label())));
    }

    let mut external_state = TuiState::default();
    let mut external_panel = IntegrationPanel::new(IntegrationKind::Skill);
    external_panel.set_skill_provider(SkillsDirectoryProvider::Claude);
    external_panel.finish_refresh(Ok(vec![IntegrationItem {
        id: "external".to_string(),
        title: "External".to_string(),
        detail: "Installed".to_string(),
        enabled: true,
        actions: Vec::new(),
    }]));
    external_state.overlays.push(UiOverlay::Integration(external_panel));
    let external = build_modal_host(&external_state).expect("external skill overlay");
    assert!(external.body_lines.iter().any(|line| line.contains("平台 · Claude")));
    assert!(external.body_lines.iter().any(|line| line.contains("当前平台只读")));
    assert!(!external.body_lines.iter().any(|line| line.contains("启停")));
}

#[test]
fn tui_pill_new_preserves_label_and_tone() {
    let pill = TuiPill::new("Ready", TuiTone::Success);
    assert_eq!(pill.label, "Ready");
    assert_eq!(pill.tone, TuiTone::Success);
}

#[test]
fn status_header_uses_fallbacks_truncation_and_spinner_modulo() {
    let mut state = TuiState::default();
    state.session.scope = Some("workspace".to_string());
    state.project.workspace_root = Some(PathBuf::from("/tmp/example/workspace"));
    let status = TuiStatusSummary {
        session_id: Some("session-123".to_string()),
        title: "   ".to_string(),
        provider_name: None,
        model_name: Some("gpt".to_string()),
        message_count: 0,
        assistant_message_count: 0,
        step_count: 0,
        pending_questions: 0,
        todo_count: 0,
        overlay_depth: 0,
        prompt_busy: false,
        turn_terminal: UiTurnTerminal::Streaming,
        token_usage: UiTokenUsage::default(),
    };

    let header = build_status_header(&state, &status, "VW", "http://localhost:1234567890/path", 5);
    assert_eq!(header.title, "新会话");
    assert_eq!(header.terminal, "输出中");
    assert_eq!(header.terminal_tone, TuiTone::Accent);
    assert_eq!(header.scope, "workspace");
    assert_eq!(header.spinner, "/");
}

#[test]
fn status_footer_reflects_errors_tokens_queue_questions_and_todos() {
    let mut state = TuiState::default();
    state.status.last_error = Some("network is unavailable for a long time".to_string());
    state.prompt.queue_command(QueuedPromptCommand {
        raw: "queued".to_string(),
        kind: QueuedPromptCommandKind::Submit,
        enqueued_ms: None,
        ..QueuedPromptCommand::default()
    });
    reduce_tui_state(
        &mut state,
        TuiAction::MessagePushed(UiMessage::User(UiUserMessage {
            base: UiMessageBase::new(UiMessageId::new("u1")),
            text: "hello".to_string(),
        })),
    );
    state.tasks.pending_questions.push(crate::cli::tui_v2::model::UiQuestionOverlay {
        request_id: "req".to_string(),
        session_id: "session".to_string(),
        prompts: Vec::new(),
        answers: Vec::new(),
        tool: None,
        selected_index: 0,
    });
    state.tasks.todo_overlay = Some(crate::cli::tui_v2::model::UiTodoOverlay {
        session_id: None,
        items: vec![crate::cli::tui_v2::model::UiTodoItem {
            id: "1".to_string(),
            content: "todo".to_string(),
            status: "pending".to_string(),
            priority: "medium".to_string(),
        }],
        selected_index: 0,
        dirty: false,
    });

    let mut status = select_status_summary(&state);
    status.token_usage.input_tokens = 3;
    status.token_usage.output_tokens = 4;
    let window = select_visible_grouped_transcript_window(&state);
    let footer = build_status_footer(&state, &status, &window);

    assert!(footer.pills.iter().any(|pill| pill.label.starts_with("错误 ")));
    assert!(footer.pills.iter().any(|pill| pill.label == "令牌 3/4"));
    assert!(!footer.pills.iter().any(|pill| pill.label.starts_with("队列 ")));
    assert!(footer.pills.iter().any(|pill| pill.label == "问题 1"));
    assert!(footer.pills.iter().any(|pill| pill.label == "待办 1"));
}

#[test]
fn prompt_host_reports_suggestions_queue_footer_and_last_submission() {
    let mut state = TuiState::default();
    state.project.workspace_root = Some(PathBuf::from("/tmp/must-not-render-in-prompt-bottom"));
    reduce_tui_state(&mut state, TuiAction::StatusModelSet(Some("gpt-5.4".to_string())));
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("/".to_string()));
    state.prompt.set_selected_suggestion_index(Some(2));
    for index in 0..5 {
        state.prompt.queue_command(QueuedPromptCommand {
            raw: format!("cmd-{index}"),
            kind: QueuedPromptCommandKind::Submit,
            enqueued_ms: Some(index),
            ..QueuedPromptCommand::default()
        });
    }
    state.prompt.last_submission = Some(PromptSubmission {
        stream_id: None,
        session_id: None,
        text: "old".to_string(),
        root: None,
        model: None,
        execution: Box::new(SessionExecutionSnapshot::default()),
        history_len: 0,
        status: PromptSubmissionStatus::Error { message: "failed".to_string() },
    });

    let host = build_prompt_host(&state);
    assert_eq!(host.placeholder, "输入 / 命令…");
    assert_eq!(host.suggestion_rows.len(), 3);
    assert!(host.suggestion_rows.iter().any(|row| row.selected));
    assert_eq!(host.queued_commands.last().map(|pill| pill.label.as_str()), Some("+2 more"));
    assert!(!host.footer_pills.is_empty());
    assert!(host.footer_pills.iter().any(|pill| pill.label == "模型 gpt-5.4"));
    assert!(
        host.footer_pills.iter().all(|pill| !pill.label.starts_with("目录 ")),
        "输入区底部不应重复显示工作目录"
    );
}

#[test]
fn prompt_queue_row_omits_workspace_directory() {
    let mut state = TuiState::default();
    state.project.workspace_root = Some(PathBuf::from("/tmp/must-not-render-in-queue"));
    state.prompt.queue_command(QueuedPromptCommand {
        raw: "queued".to_string(),
        kind: QueuedPromptCommandKind::Submit,
        ..QueuedPromptCommand::default()
    });
    let host = build_prompt_host(&state);
    let backend = TestBackend::new(100, 8);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| {
            let area = frame.area();
            TuiRenderer::default().render_prompt_host(frame, area, &host, true);
        })
        .expect("render prompt host");

    let rendered =
        terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
    assert!(!rendered.contains("目录"));
    assert!(!rendered.contains("must-not-render-in-queue"));
}

#[test]
fn file_picker_rows_use_compact_ascii_labels_and_explicit_selection_marker() {
    let mut state = TuiState::default();
    state.completion_catalog.items = (0..9)
        .map(|index| CompletionCatalogItem {
            replacement: format!("@src/file-{index}.rs"),
            label: format!("@src/file-{index}.rs"),
            detail: Some("/workspace/project".to_string()),
            kind: CompletionKind::File,
            score: 100 - index,
        })
        .chain([CompletionCatalogItem {
            replacement: "@\"docs space\"/".to_string(),
            label: "@\"docs space\"/".to_string(),
            detail: Some("/workspace/project".to_string()),
            kind: CompletionKind::Directory,
            score: 200,
        }])
        .collect();
    state.prompt.set_value("@");
    state.prompt.set_selected_suggestion_index(Some(1));

    let host = build_prompt_host(&state);
    assert_eq!(host.suggestion_rows.len(), 8);
    assert!(host.suggestion_rows.iter().all(|row| row.detail.is_none()));
    assert!(host.suggestion_rows.iter().any(|row| row.label == "docs space/"));
    assert!(host.suggestion_rows.iter().all(|row| !row.label.starts_with('@')));

    let renderer = super::TuiRenderer::for_capabilities(TerminalCapabilities::default(), false);
    let lines = renderer.render_prompt_suggestion_rows(&host.suggestion_rows);
    let labels = lines
        .iter()
        .map(|line| line.spans.iter().map(|span| span.content.as_ref()).collect::<String>())
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label.contains("[D] docs space/")));
    assert!(labels.iter().any(|label| label.contains("[F] src/file-0.rs")));
    assert!(labels.iter().all(|label| !label.contains("/workspace/project")));

    let selected_line = lines
        .iter()
        .find(|line| line.spans.iter().any(|span| span.content.contains("src/file-0.rs")))
        .expect("selected file row");
    assert_eq!(selected_line.style.bg, None);
    assert!(!selected_line.style.add_modifier.contains(ratatui::style::Modifier::REVERSED));
    assert!(
        selected_line.spans.first().is_some_and(|span| span.content.starts_with("> [F]")),
        "selected file row keeps a visible marker"
    );
}

#[test]
fn project_modified_modal_and_prompt_viewport_helpers_are_stable() {
    let mut state = TuiState::default();
    state.session.session_id = Some("session-1".to_string());
    state.session.scope = Some("project".to_string());
    state.project.info = "Demo project".to_string();
    state.project.workspace_root = Some(PathBuf::from("/tmp/workspace"));
    state.project.git_status = GitWorkspaceStatus::ReadyDirty(vec!["src/lib.rs".to_string()]);
    let status = select_status_summary(&state);

    let project = build_project_context_host(&state, &status);
    assert!(project.pills.iter().any(|pill| pill.label == "范围 project"));
    assert!(project.body_lines.iter().any(|line| line.contains("Demo project")));
    let files = build_modified_files_host(&state);
    assert_eq!(files.pills[0].label, "数量 1");

    state.overlays.push(UiOverlay::Confirm(UiConfirmOverlay {
        title: String::new(),
        body: "Clear everything?".to_string(),
        confirm_label: "Clear".to_string(),
        cancel_label: "Cancel".to_string(),
        destructive: true,
        intent: UiConfirmIntent::Generic,
    }));
    let modal = build_modal_host(&state).expect("modal host");
    assert_eq!(modal.title, "确认");
    assert!(modal.body_lines.iter().any(|line| line.contains("危险操作=true")));

    let mut prompt = build_prompt_host(&TuiState::default());
    prompt.value = "first\n第二\nthird".to_string();
    prompt.cursor_char_index = prompt.value.chars().count();
    let viewport = render_prompt_value_lines(&prompt, super::TuiTheme::default(), 2);
    assert_eq!(viewport.hidden_above, 1);
    assert_eq!(viewport.cursor.y, 1);
}

#[test]
fn small_helpers_cover_labels_truncation_windows_and_sidebar_clipping() {
    assert_eq!(visible_prompt_suggestion_window(2, 1, 3), (0, 2));
    assert_eq!(visible_prompt_suggestion_window(8, 7, 3), (5, 8));
    assert_eq!(truncate_label("abcdef", 4), "a...");
    assert_eq!(truncate_start("/very/long/path", 8), ".../path");
    assert_eq!(compact_path_label(None), "-");
    assert_eq!(prompt_mode_label(&PromptMode::Search), "搜索");
    assert_eq!(step_state_label(&UiStepState::Failed), "失败");
    assert_eq!(overlay_kind_label(crate::cli::tui_v2::model::UiOverlayKind::Mcp), "MCP");
    assert_eq!(
        terminal_tone(&UiTurnTerminal::TimedOut { message: "slow".into() }),
        TuiTone::Danger
    );

    let queued = queue_command_pill(&QueuedPromptCommand {
        raw: "/really-long-command-name".to_string(),
        kind: QueuedPromptCommandKind::SlashCommand,
        enqueued_ms: None,
        ..QueuedPromptCommand::default()
    });
    assert!(queued.label.starts_with("命令 "));
    assert_eq!(queued.tone, TuiTone::Warning);

    let lines = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()];
    assert_eq!(clip_sidebar_body_lines(&lines, 0), Vec::<String>::new());
    assert_eq!(clip_sidebar_body_lines(&lines, 1), vec!["... +4 more".to_string()]);
    assert_eq!(clip_sidebar_body_lines(&lines, 3), vec!["a", "b", "... +2 more"]);
    assert_eq!(sidebar_lines(ratatui::text::Line::raw("head"), &lines, 0).len(), 0);
    assert_eq!(sidebar_lines(ratatui::text::Line::raw("head"), &lines, 1).len(), 1);
}

#[test]
fn last_submission_and_step_timing_helpers_cover_terminal_variants() {
    let mut state = TuiState::default();
    assert_eq!(last_submission_pill(&state).label, "上次空闲");
    state.prompt.last_submission = Some(PromptSubmission {
        stream_id: None,
        session_id: None,
        text: "text".to_string(),
        root: None,
        model: None,
        execution: Box::new(SessionExecutionSnapshot::default()),
        history_len: 0,
        status: PromptSubmissionStatus::TimedOut { message: "slow".to_string() },
    });
    assert_eq!(last_submission_pill(&state).label, "上次失败");

    let running = UiTaskStepItem {
        message_id: UiMessageId::new("step"),
        step_index: 1,
        state: UiStepState::Running,
        started_ms: 10,
        finished_ms: None,
        model: None,
        finish_reason: None,
        usage: UiTokenUsage::default(),
    };
    assert_eq!(step_timing_line(&running), "耗时: 开始=10 结束=进行中");
    let finished = UiTaskStepItem { finished_ms: Some(25), ..running };
    assert_eq!(step_timing_line(&finished), "耗时: 开始=10 结束=25 总计=15ms");
}

#[test]
fn terminal_label_and_tone_color_covers_all_variants() {
    assert_eq!(terminal_label(&UiTurnTerminal::Pending), "待命");
    assert_eq!(terminal_label(&UiTurnTerminal::Streaming), "输出中");
    assert_eq!(terminal_label(&UiTurnTerminal::Done { finish_reason: None }), "完成");
    assert_eq!(terminal_label(&UiTurnTerminal::Cancelled { reason: None }), "已取消");
    assert_eq!(terminal_label(&UiTurnTerminal::TimedOut { message: "slow".into() }), "超时");
    assert_eq!(terminal_label(&UiTurnTerminal::Error { message: "err".into() }), "错误");

    let theme = super::TuiTheme::default();
    assert_eq!(tone_color(TuiTone::Accent, theme), theme.accent);
    assert_eq!(tone_color(TuiTone::Muted, theme), theme.border);
    assert_eq!(tone_color(TuiTone::Success, theme), theme.success);
    assert_eq!(tone_color(TuiTone::Warning, theme), theme.warning);
    assert_eq!(tone_color(TuiTone::Danger, theme), theme.danger);
}

#[test]
fn prompt_shortcut_hint_covers_all_modes() {
    let mut state = TuiState::default();
    assert!(prompt_shortcut_hint(&state).contains("Enter 发送"));

    state.prompt.mode = PromptMode::SlashCommand;
    assert_eq!(prompt_shortcut_hint(&state), "Up/Down 切换 · Tab/Enter 接受 · Enter 执行");

    state.prompt.mode = PromptMode::Busy;
    assert_eq!(prompt_shortcut_hint(&state), "当前轮次仍在输出，新输入会暂存到队列");
}

#[test]
fn prompt_line_prefix_and_cursor_column_edge_cases() {
    assert_eq!(prompt_line_prefix(0), "");
    assert_eq!(prompt_line_prefix(1), "· ");
    assert_eq!(prompt_line_prefix(5), "· ");

    assert_eq!(prompt_cursor_line_column("", 0), (0, 0));
    assert_eq!(prompt_cursor_line_column("hello", 0), (0, 0));
    assert_eq!(prompt_cursor_line_column("hello", 3), (0, 3));
    assert_eq!(prompt_cursor_line_column("hello", 10), (0, 5));
    assert_eq!(prompt_cursor_line_column("a\nbc\ndef", 2), (1, 0));
    assert_eq!(prompt_cursor_line_column("a\nbc\ndef", 4), (1, 2));
    assert_eq!(prompt_cursor_line_column("a\nbc\ndef", 6), (2, 1));
}

#[test]
fn compact_path_label_and_queue_pill_extra_cases() {
    assert_eq!(compact_path_label(Some(std::path::Path::new("/tmp/test/path"))), "/tmp/test/path");

    let submit_pill = queue_command_pill(&QueuedPromptCommand {
        raw: "hello".to_string(),
        kind: QueuedPromptCommandKind::Submit,
        enqueued_ms: None,
        ..QueuedPromptCommand::default()
    });
    assert!(submit_pill.label.starts_with("发送 "));
    assert_eq!(submit_pill.tone, TuiTone::Warning);
}

#[test]
fn last_submission_pill_covers_all_statuses() {
    let mut state = TuiState::default();
    assert_eq!(last_submission_pill(&state).label, "上次空闲");

    let make_submission = |status: PromptSubmissionStatus| -> PromptSubmission {
        PromptSubmission {
            stream_id: None,
            session_id: None,
            text: "x".to_string(),
            root: None,
            model: None,
            execution: Box::new(SessionExecutionSnapshot::default()),
            history_len: 0,
            status,
        }
    };

    state.prompt.last_submission = Some(make_submission(PromptSubmissionStatus::Pending));
    assert_eq!(last_submission_pill(&state).label, "上次等待");

    state.prompt.last_submission = Some(make_submission(PromptSubmissionStatus::Streaming));
    assert_eq!(last_submission_pill(&state).label, "上次输出中");

    state.prompt.last_submission =
        Some(make_submission(PromptSubmissionStatus::Done { finish_reason: Some("stop".into()) }));
    assert_eq!(last_submission_pill(&state).label, "上次完成");

    state.prompt.last_submission =
        Some(make_submission(PromptSubmissionStatus::Cancelled { reason: Some("user".into()) }));
    assert_eq!(last_submission_pill(&state).label, "上次取消");

    state.prompt.last_submission =
        Some(make_submission(PromptSubmissionStatus::Error { message: "fail".into() }));
    assert_eq!(last_submission_pill(&state).label, "上次失败");
}

#[test]
fn status_header_and_footer_edge_cases() {
    let mut state = TuiState::default();
    state.session.scope = Some("project".to_string());
    state.project.workspace_root = Some(std::path::PathBuf::from("/tmp/workspace"));

    let status = TuiStatusSummary {
        session_id: Some("s1".to_string()),
        title: "  ".to_string(),
        provider_name: None,
        model_name: Some("gpt".to_string()),
        message_count: 0,
        assistant_message_count: 0,
        step_count: 0,
        pending_questions: 0,
        todo_count: 0,
        overlay_depth: 0,
        prompt_busy: false,
        turn_terminal: UiTurnTerminal::Done { finish_reason: None },
        token_usage: UiTokenUsage::default(),
    };

    let header = build_status_header(&state, &status, "VW", "http://localhost:1234567890/path", 3);
    assert_eq!(header.title, "新会话");
    assert_eq!(header.terminal, "完成");
    assert_eq!(header.terminal_tone, TuiTone::Success);
    assert_eq!(header.spinner, "\\");

    let window = select_visible_grouped_transcript_window(&state);
    let footer = build_status_footer(&state, &status, &window);
    assert!(!footer.pills.iter().any(|pill| pill.label.starts_with("错误 ")));
    assert!(!footer.pills.iter().any(|pill| pill.label.starts_with("令牌 ")));
    assert!(!footer.pills.iter().any(|pill| pill.label.starts_with("问题 ")));
    assert!(!footer.pills.iter().any(|pill| pill.label.starts_with("待办 ")));
}

#[test]
fn prompt_host_busy_and_non_slash_modes() {
    let mut state = TuiState::default();
    state.prompt.mode = PromptMode::Busy;
    let host = build_prompt_host(&state);
    assert_eq!(host.placeholder, "输入消息…");
    assert!(host.helper_text.contains("当前轮次仍在输出"));

    state.prompt.mode = PromptMode::Search;
    state.status.model_name = Some("gpt-5".to_string());
    let host = build_prompt_host(&state);
    assert_eq!(host.placeholder, "输入消息…");
    assert!(host.helper_text.contains("Enter 发送"));
    assert_eq!(host.suggestion_detail, None);
    assert_eq!(host.queued_commands.len(), 0);
    assert!(host.footer_pills.iter().any(|pill| pill.label == "模型 gpt-5"));
}

#[test]
fn project_context_and_modified_files_edge_cases() {
    let mut state = TuiState::default();
    state.session.scope = None;

    let status = select_status_summary(&state);
    let project = build_project_context_host(&state, &status);
    assert!(project.pills.iter().any(|pill| pill.label == "范围 -"));
    assert!(project.body_lines.iter().any(|line| line.contains("项目: -")));
    assert!(project.body_lines.iter().any(|line| line.contains("会话标题: -")));

    let files = build_modified_files_host(&state);
    assert_eq!(files.pills[0].label, "数量 0");
    assert!(files.body_lines[0].contains("没有文件变更"));
}

#[test]
fn build_modal_host_confirm_search_and_question() {
    let mut state = TuiState::default();
    assert_eq!(build_modal_host(&state), None, "empty stack yields None");

    state.overlays.push(UiOverlay::Confirm(UiConfirmOverlay {
        title: "清除数据".to_string(),
        body: "确认?".to_string(),
        confirm_label: "确认".to_string(),
        cancel_label: "取消".to_string(),
        destructive: false,
        intent: UiConfirmIntent::Generic,
    }));
    let modal = build_modal_host(&state).expect("confirm modal");
    assert_eq!(modal.title, "清除数据");
    assert!(modal.body_lines.iter().any(|line| line.contains("危险操作=false")));
    assert_eq!(modal.chips[0].label, "类型 确认");

    state.overlays.push(UiOverlay::Search(UiSearchOverlay {
        query: "test".to_string(),
        case_sensitive: true,
        matches: vec![UiSearchMatch {
            message_id: Some(UiMessageId::new("m1")),
            start: 0,
            end: 4,
            preview: "test preview".to_string(),
        }],
        selected_index: Some(0),
    }));
    let modal = build_modal_host(&state).expect("search modal");
    assert_eq!(modal.title, "搜索");
    assert!(modal.body_lines.iter().any(|line| line.contains("大小写: 区分")));

    state.overlays.push(UiOverlay::Question(UiQuestionOverlay {
        request_id: "r1".to_string(),
        session_id: "s1".to_string(),
        prompts: vec![UiQuestionPrompt {
            header: "问题标题".to_string(),
            question: "这是问题内容?".to_string(),
            options: vec![
                UiQuestionOption {
                    label: "选项A".to_string(),
                    description: "描述A".to_string(),
                    preview: None,
                },
                UiQuestionOption {
                    label: "选项B".to_string(),
                    description: String::new(),
                    preview: Some("预览B".to_string()),
                },
            ],
            multiple: false,
            allow_custom_input: false,
        }],
        answers: vec![vec!["选项B".to_string()]],
        tool: Some(UiQuestionToolContext {
            message_id: "m1".to_string(),
            call_id: "c1".to_string(),
        }),
        selected_index: 0,
    }));
    let modal = build_modal_host(&state).expect("question modal");
    assert_eq!(modal.title, "工具提问回退");
    assert!(modal.body_lines.iter().any(|line| line.contains("当前回答: 选项B")));
    assert!(modal.body_lines.iter().any(|line| line.contains("预览:")));
}

#[test]
fn build_modal_host_todo_task_and_palette() {
    let mut state = TuiState::default();

    state.overlays.push(UiOverlay::Todo(UiTodoOverlay {
        session_id: Some("s1".to_string()),
        items: vec![
            UiTodoItem {
                id: "1".to_string(),
                content: "任务一".to_string(),
                status: "pending".to_string(),
                priority: "high".to_string(),
            },
            UiTodoItem {
                id: "2".to_string(),
                content: "任务二".to_string(),
                status: "completed".to_string(),
                priority: "low".to_string(),
            },
        ],
        selected_index: 0,
        dirty: true,
    }));
    let modal = build_modal_host(&state).expect("todo modal");
    assert_eq!(modal.title, "待办");
    assert!(modal.body_lines.iter().any(|line| line.contains("脏标记: true")));
    assert!(modal.body_lines.iter().any(|line| line.contains("状态汇总")));

    state.overlays.push(UiOverlay::Task(UiTaskOverlay {
        session_id: Some("s1".to_string()),
        turn_terminal: UiTurnTerminal::Pending,
        pending_questions: 1,
        todo_count: 2,
        sync_error: Some("network error".to_string()),
        steps: vec![UiTaskStepItem {
            message_id: UiMessageId::new("m1"),
            step_index: 0,
            state: UiStepState::Running,
            started_ms: 100,
            finished_ms: None,
            model: None,
            finish_reason: None,
            usage: UiTokenUsage::default(),
        }],
        selected_index: 0,
    }));
    let modal = build_modal_host(&state).expect("task modal");
    assert_eq!(modal.title, "任务面板");
    assert!(modal.body_lines.iter().any(|line| line.contains("任务同步错误: network error")));

    state.overlays.push(UiOverlay::CommandPalette(UiCommandPaletteOverlay {
        query: "test".to_string(),
        items: vec![UiPaletteItem {
            id: "1".to_string(),
            label: "测试命令".to_string(),
            detail: None,
            source: "Builtin".to_string(),
            argument_hint: None,
            available: true,
            unavailable_reason: None,
            replacement: "/test".to_string(),
        }],
        selected_index: Some(0),
        first_visible_index: 0,
        visible_rows: 8,
        notice: None,
    }));
    let modal = build_modal_host(&state).expect("palette modal");
    assert_eq!(modal.title, "命令面板");
    assert!(modal.body_lines.iter().any(|line| line.contains("查询: /test")));
}

#[test]
fn slash_palette_render_shows_source_argument_and_disabled_reason() {
    let mut state = TuiState::default();
    state.overlays.push(UiOverlay::CommandPalette(UiCommandPaletteOverlay {
        query: "dep".to_string(),
        items: vec![
            UiPaletteItem {
                id: "deploy".to_string(),
                label: "/deploy".to_string(),
                detail: Some("Deploy the selected workflow".to_string()),
                source: "Workflow".to_string(),
                argument_hint: Some("<environment>".to_string()),
                available: true,
                unavailable_reason: None,
                replacement: "/deploy ".to_string(),
            },
            UiPaletteItem {
                id: "disabled".to_string(),
                label: "/disabled".to_string(),
                detail: Some("Disabled workflow command".to_string()),
                source: "Workflow".to_string(),
                argument_hint: None,
                available: false,
                unavailable_reason: Some("workflow service is offline".to_string()),
                replacement: "/disabled".to_string(),
            },
        ],
        selected_index: Some(0),
        first_visible_index: 0,
        visible_rows: 8,
        notice: Some("workflow service is offline".to_string()),
    }));

    let modal = build_modal_host(&state).expect("palette modal");
    let joined = modal.body_lines.join("\n");
    assert!(joined.contains("Workflow"));
    assert!(joined.contains("<environment>"));
    assert!(joined.contains("workflow service is offline"));

    for (width, height) in [(80, 24), (120, 40)] {
        let snapshot = render_tui_snapshot_with_theme(
            width,
            height,
            &state,
            "VW",
            "gateway://test",
            0,
            super::TuiThemeName::Dark,
            TerminalCapabilities::default(),
        )
        .expect("palette should render");
        assert_eq!((snapshot.width, snapshot.height), (width, height));
        assert_eq!(snapshot.cells.len(), usize::from(width) * usize::from(height));
    }
}

#[test]
fn build_modal_host_error_mcp_memory() {
    let mut state = TuiState::default();

    state.overlays.push(UiOverlay::Error(UiErrorOverlay {
        title: "错误标题".to_string(),
        message: "出错了".to_string(),
        recoverable: true,
    }));
    let modal = build_modal_host(&state).expect("recoverable error");
    assert_eq!(modal.title, "错误标题");
    assert!(modal.body_lines.iter().any(|line| line.contains("Esc 关闭弹层")));

    state.overlays.push(UiOverlay::Error(UiErrorOverlay {
        title: "致命错误".to_string(),
        message: "无法恢复".to_string(),
        recoverable: false,
    }));
    let modal = build_modal_host(&state).expect("fatal error");
    assert!(modal.body_lines.iter().any(|line| line.contains("需要显式恢复")));

    state.overlays.push(UiOverlay::Mcp(UiMcpOverlay {
        config_source: "local".to_string(),
        servers: vec![UiMcpServerInfo {
            name: "server1".to_string(),
            transport: McpServerTransport::Stdio,
            address: "./path".to_string(),
        }],
        selected_index: 0,
    }));
    let modal = build_modal_host(&state).expect("mcp modal");
    assert_eq!(modal.title, "MCP");
    assert!(modal.body_lines.iter().any(|line| line.contains("服务器数: 1")));
    assert!(modal.body_lines.iter().any(|line| line.contains("[stdio]")));

    state.overlays.push(UiOverlay::Memory(UiMemoryOverlay {
        entries: vec![UiMemoryEntry {
            scope: "workspace".to_string(),
            filename: "note.md".to_string(),
            path: "/tmp/note.md".to_string(),
            preview_lines: vec!["line1".to_string()],
            total_lines: 1,
        }],
        selected_index: 0,
    }));
    let modal = build_modal_host(&state).expect("memory modal");
    assert_eq!(modal.title, "记忆");
    assert!(modal.body_lines.iter().any(|line| line.contains("条目数: 1")));
}

#[test]
fn prompt_multiline_viewport_starts_at_top_and_tracks_cursor() {
    let theme = super::TuiTheme::default();

    let mut prompt: super::TuiPromptHost = build_prompt_host(&TuiState::default());
    prompt.placeholder = "输入消息…".to_string();

    let empty = render_prompt_value_lines(&prompt, theme, 3);
    assert_eq!(empty.hidden_above, 0);
    assert_eq!(empty.cursor.x, 0);
    assert_eq!(empty.cursor.y, 0);
    assert_eq!(empty.lines.len(), 1);

    prompt.value = "hello".to_string();
    prompt.cursor_char_index = prompt.value.chars().count();
    let single_line = render_prompt_value_lines(&prompt, theme, 3);
    assert_eq!(single_line.hidden_above, 0);
    assert_eq!(single_line.cursor.x, 5);
    assert_eq!(single_line.cursor.y, 0);
    assert_eq!(single_line.lines.len(), 1);

    prompt.value = "first\nsecond".to_string();
    prompt.cursor_char_index = prompt.value.chars().count();
    let multiline = render_prompt_value_lines(&prompt, theme, 3);
    assert_eq!(multiline.hidden_above, 0);
    assert_eq!(multiline.cursor.y, 1);
    assert_eq!(multiline.lines.len(), 2);

    prompt.value = "zero\none\ntwo\nthree".to_string();
    prompt.cursor_char_index = prompt.value.chars().count();
    let overflowing = render_prompt_value_lines(&prompt, theme, 3);
    assert_eq!(overflowing.hidden_above, 1);
    assert_eq!(overflowing.cursor.y, 2);
    assert_eq!(overflowing.lines.len(), 3);
}

#[test]
fn prompt_selection_spans_follow_graphemes_across_lines() {
    let theme = super::TuiTheme::default();
    let mut state = TuiState::default();
    reduce_tui_state(&mut state, TuiAction::PromptValueSet("中e\u{301}🙂\n下一行".to_string()));
    state.prompt.editor.selection = Some(EditorSelection { anchor: 1, active: 6 });

    let viewport = render_prompt_value_lines(&build_prompt_host(&state), theme, 3);
    let selected = viewport
        .lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .filter(|span| span.style.bg == Some(theme.accent))
        .map(|span| span.content.to_string())
        .collect::<Vec<_>>();

    assert_eq!(selected, vec!["e\u{301}🙂", "下一"]);
}

#[test]
fn suggestion_window_and_sidebar_more_edge_cases() {
    assert_eq!(visible_prompt_suggestion_window(5, 0, 3), (0, 3));
    assert_eq!(visible_prompt_suggestion_window(5, 2, 3), (1, 4));
    assert_eq!(visible_prompt_suggestion_window(5, 4, 3), (2, 5));

    let lines = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    assert_eq!(clip_sidebar_body_lines(&lines, 2), vec!["a", "... +2 more"]);

    assert_eq!(sidebar_lines(ratatui::text::Line::raw("head"), &lines, 2).len(), 2);
    assert_eq!(sidebar_lines(ratatui::text::Line::raw("head"), &lines, 3).len(), 3);
    assert_eq!(sidebar_lines(ratatui::text::Line::raw("head"), &[], 0).len(), 0);
}

#[test]
fn zh_labels_and_enum_labels_cover_remaining_branches() {
    let unseen = TuiUnseenRangeSummary {
        first_unseen_message: 0,
        first_unseen_item_index: 0,
        unseen_message_count: 1,
        unseen_item_count: 1,
        boundary_in_window: true,
    };
    assert_eq!(zh_unseen_pill_label(unseen), "1 条新消息");
    let unseen_multi = TuiUnseenRangeSummary {
        first_unseen_message: 0,
        first_unseen_item_index: 0,
        unseen_message_count: 5,
        unseen_item_count: 5,
        boundary_in_window: true,
    };
    assert_eq!(zh_unseen_pill_label(unseen_multi), "5 条新消息");
    assert_eq!(zh_unseen_divider_label(unseen_multi), "下方还有 5 条新消息");

    assert_eq!(zh_sticky_notice("message 42 ".to_string()), "消息 42 固定在当前视口上方。");
    assert_eq!(zh_sticky_notice("other prefix text".to_string()), "上方还有一段已固定的上下文。");

    assert_eq!(prompt_mode_label(&PromptMode::Compose), "对话");
    assert_eq!(prompt_mode_label(&PromptMode::QuestionReply), "问答");
    assert_eq!(prompt_mode_label(&PromptMode::TodoEdit), "待办");
    assert_eq!(prompt_mode_label(&PromptMode::CommandPalette), "面板");
    assert_eq!(prompt_mode_label(&PromptMode::Busy), "忙碌");
    assert_eq!(prompt_mode_label(&PromptMode::SlashCommand), "命令");
    assert_eq!(prompt_mode_label(&PromptMode::Search), "搜索");

    assert_eq!(overlay_kind_label(UiOverlayKind::Confirm), "确认");
    assert_eq!(overlay_kind_label(UiOverlayKind::Search), "搜索");
    assert_eq!(overlay_kind_label(UiOverlayKind::Question), "提问");
    assert_eq!(overlay_kind_label(UiOverlayKind::Todo), "待办");
    assert_eq!(overlay_kind_label(UiOverlayKind::Task), "任务");
    assert_eq!(overlay_kind_label(UiOverlayKind::CommandPalette), "命令面板");
    assert_eq!(overlay_kind_label(UiOverlayKind::Report), "报告");
    assert_eq!(overlay_kind_label(UiOverlayKind::Error), "错误");
    assert_eq!(overlay_kind_label(UiOverlayKind::Memory), "记忆");

    assert_eq!(step_state_label(&UiStepState::Pending), "等待中");
    assert_eq!(step_state_label(&UiStepState::Running), "进行中");
    assert_eq!(step_state_label(&UiStepState::Complete), "完成");
    assert_eq!(step_state_label(&UiStepState::Cancelled), "已取消");
}

#[test]
fn numeric_and_unicode_helpers_handle_saturation_and_tiny_windows() {
    assert_eq!(usize_to_u16(0), 0);
    assert_eq!(usize_to_u16(u16::MAX as usize), u16::MAX);
    assert_eq!(usize_to_u16(usize::MAX), u16::MAX);

    assert_eq!(visible_prompt_suggestion_window(4, usize::MAX, 0), (4, 4));
    assert_eq!(truncate_label("你好世界", 3), "你好世");
    assert_eq!(truncate_label("你好世界", 4), "你好世界");
    assert_eq!(truncate_start("你好世界", 2), "世界");
    assert_eq!(truncate_start("你好世界", 4), "你好世界");
}

#[test]
fn window_sticky_label_distinguishes_anchor_tail_and_history() {
    let summary = |sticky_message, follow_tail| TuiWindowSummary {
        top_message: 0,
        sticky_message,
        follow_tail,
        total_items: 0,
        top_item_index: 0,
        start_item_index: 0,
        end_item_index: 0,
        covered_message_start: 0,
        covered_message_end: 0,
    };

    assert_eq!(zh_window_sticky_label(summary(Some(17), false)), "停在 m17");
    assert_eq!(zh_window_sticky_label(summary(None, true)), "末尾");
    assert_eq!(zh_window_sticky_label(summary(None, false)), "浏览历史");
}
