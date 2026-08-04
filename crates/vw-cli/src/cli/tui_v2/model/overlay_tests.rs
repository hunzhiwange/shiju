use vw_shared::question;
use vw_shared::todo::Todo;

use super::overlay::{
    McpServerTransport, OverlayFocus, OverlayState, UiCommandPaletteOverlay, UiConfirmIntent,
    UiConfirmOverlay, UiErrorOverlay, UiMcpOverlay, UiMcpServerInfo, UiMemoryEntry,
    UiMemoryOverlay, UiOverlay, UiOverlayKind, UiPaletteItem, UiQuestionOverlay,
    UiQuestionSurfaceKind, UiReportOverlay, UiSearchMatch, UiSearchOverlay, UiTaskOverlay,
    UiTaskStepItem, UiTodoOverlay,
};
use super::ui_message::{UiMessageId, UiStepState, UiTokenUsage, UiTurnTerminal};

#[test]
fn overlay_state_push_pop_and_clear_manage_focus() {
    let mut state = OverlayState::default();
    assert_eq!(state.focus, OverlayFocus::Prompt);
    assert!(state.active().is_none());

    state.push(UiOverlay::Confirm(UiConfirmOverlay {
        title: "Confirm".to_string(),
        body: "Proceed?".to_string(),
        confirm_label: "Yes".to_string(),
        cancel_label: "No".to_string(),
        destructive: false,
        intent: UiConfirmIntent::Generic,
    }));
    assert_eq!(state.focus, OverlayFocus::Overlay);
    assert_eq!(state.active().map(UiOverlay::kind), Some(UiOverlayKind::Confirm));

    state.push(UiOverlay::Error(UiErrorOverlay {
        title: "Oops".to_string(),
        message: "failed".to_string(),
        recoverable: true,
    }));
    assert_eq!(state.pop().map(|overlay| overlay.kind()), Some(UiOverlayKind::Error));
    assert_eq!(state.focus, OverlayFocus::Overlay);
    assert_eq!(state.pop().map(|overlay| overlay.kind()), Some(UiOverlayKind::Confirm));
    assert_eq!(state.focus, OverlayFocus::Prompt);
    assert_eq!(state.pop(), None);

    state.push(UiOverlay::Search(UiSearchOverlay::default()));
    state.clear();
    assert!(state.stack.is_empty());
    assert_eq!(state.focus, OverlayFocus::Prompt);
}

#[test]
fn ui_overlay_kind_covers_core_variants() {
    let overlays = [
        UiOverlay::Confirm(UiConfirmOverlay {
            title: String::new(),
            body: String::new(),
            confirm_label: String::new(),
            cancel_label: String::new(),
            destructive: false,
            intent: UiConfirmIntent::Generic,
        }),
        UiOverlay::Search(UiSearchOverlay::default()),
        UiOverlay::Question(question_overlay(None, "Need input?", "Choose")),
        UiOverlay::Todo(UiTodoOverlay::default()),
        UiOverlay::Task(UiTaskOverlay::default()),
        UiOverlay::CommandPalette(UiCommandPaletteOverlay::default()),
        UiOverlay::Report(UiReportOverlay::default()),
        UiOverlay::Error(UiErrorOverlay {
            title: "Error".to_string(),
            message: "boom".to_string(),
            recoverable: false,
        }),
        UiOverlay::Mcp(UiMcpOverlay::default()),
        UiOverlay::Memory(UiMemoryOverlay::default()),
    ];
    let expected = [
        UiOverlayKind::Confirm,
        UiOverlayKind::Search,
        UiOverlayKind::Question,
        UiOverlayKind::Todo,
        UiOverlayKind::Task,
        UiOverlayKind::CommandPalette,
        UiOverlayKind::Report,
        UiOverlayKind::Error,
        UiOverlayKind::Mcp,
        UiOverlayKind::Memory,
    ];

    for (overlay, kind) in overlays.iter().zip(expected) {
        assert_eq!(overlay.kind(), kind);
    }
}

#[test]
fn question_overlay_from_request_preserves_prompts_answers_and_tool_context() {
    let request = question::Request {
        id: "req-1".to_string(),
        session_id: "session-1".to_string(),
        questions: vec![
            question::Info {
                question: "Pick one".to_string(),
                header: "Header".to_string(),
                options: vec![question::OptionInfo {
                    label: "A".to_string(),
                    description: "Alpha".to_string(),
                    preview: Some("preview".to_string()),
                }],
                multiple: Some(true),
                custom: Some(true),
            },
            question::Info {
                question: "Second".to_string(),
                header: "More".to_string(),
                options: Vec::new(),
                multiple: None,
                custom: None,
            },
        ],
        tool: Some(question::ToolMeta {
            message_id: "msg-1".to_string(),
            call_id: "call-1".to_string(),
        }),
    };

    let overlay = UiQuestionOverlay::from_request(&request);
    assert_eq!(overlay.request_id, "req-1");
    assert_eq!(overlay.session_id, "session-1");
    assert_eq!(overlay.prompts.len(), 2);
    assert_eq!(overlay.answers, vec![Vec::<String>::new(), Vec::<String>::new()]);
    assert!(overlay.prompts[0].multiple);
    assert!(overlay.prompts[0].allow_custom_input);
    assert_eq!(overlay.prompts[0].options[0].preview.as_deref(), Some("preview"));
    assert!(overlay.is_tool_backed());
    assert_eq!(overlay.surface_kind(), UiQuestionSurfaceKind::ToolFallback);
}

#[test]
fn question_overlay_surface_labels_cover_plain_and_tool_fallback() {
    let plain = question_overlay(None, "Need input?", "Continue");
    assert!(!plain.is_tool_backed());
    assert!(!plain.is_permission_request());
    assert_eq!(plain.surface_kind(), UiQuestionSurfaceKind::Question);
    assert_eq!(plain.modal_title(), "提问");
    assert_eq!(plain.request_label(), "提问");
    assert_eq!(plain.reply_error_title(), "提问回复失败");
    assert_eq!(plain.reject_error_title(), "提问拒绝失败");
    assert_eq!(plain.empty_submission_title(), "提交提问回复");
    assert_eq!(plain.empty_submission_message(), "请至少提供一个回答后再提交。");

    let tool = question_overlay(Some(tool_meta()), "Need input?", "Continue");
    assert!(tool.is_tool_backed());
    assert!(!tool.is_permission_request());
    assert_eq!(tool.surface_kind(), UiQuestionSurfaceKind::ToolFallback);
    assert_eq!(tool.modal_title(), "工具提问回退");
    assert_eq!(tool.request_label(), "工具提问");
    assert_eq!(tool.reply_error_title(), "工具提问回复失败");
    assert_eq!(tool.reject_error_title(), "工具提问拒绝失败");
    assert_eq!(tool.empty_submission_title(), "提交工具提问回复");
    assert!(tool.empty_submission_message().contains("工具回退问题"));
}

#[test]
fn question_overlay_permission_markers_choose_permission_surface_labels() {
    let overlay = question_overlay(Some(tool_meta()), "Permission required", "Allow once");

    assert!(overlay.is_permission_request());
    assert_eq!(overlay.surface_kind(), UiQuestionSurfaceKind::PermissionRequest);
    assert_eq!(overlay.modal_title(), "权限请求");
    assert_eq!(overlay.request_label(), "权限请求");
    assert_eq!(overlay.reply_error_title(), "权限请求回复失败");
    assert_eq!(overlay.reject_error_title(), "权限请求拒绝失败");
    assert_eq!(overlay.empty_submission_title(), "提交权限请求回复");
    assert!(overlay.empty_submission_message().contains("授权选项"));

    let plain = question_overlay(None, "Need input?", "Continue");
    assert!(!plain.is_tool_backed());
    assert_eq!(plain.surface_kind(), UiQuestionSurfaceKind::Question);
    assert_eq!(plain.modal_title(), "提问");
}

#[test]
fn question_overlay_permission_detection_checks_text_description_and_empty_values() {
    assert!(
        question_overlay_with_texts(
            Some(tool_meta()),
            "Header",
            "Please APPROVE the operation",
            "Continue",
            "Description",
        )
        .is_permission_request()
    );

    assert!(
        question_overlay_with_texts(
            Some(tool_meta()),
            "Header",
            "Question?",
            "Continue",
            "Always let this tool run",
        )
        .is_permission_request()
    );

    let empty = question_overlay_with_texts(Some(tool_meta()), "  ", "", "", " ");
    assert!(!empty.is_permission_request());
    assert_eq!(empty.surface_kind(), UiQuestionSurfaceKind::ToolFallback);
}

#[test]
fn todo_overlay_from_todos_maps_shared_items_and_defaults() {
    let todos = vec![Todo {
        id: "1".to_string(),
        content: "write tests".to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
    }];

    let overlay = UiTodoOverlay::from_todos(Some("session-1"), &todos);
    assert_eq!(overlay.session_id.as_deref(), Some("session-1"));
    assert_eq!(overlay.items.len(), 1);
    assert_eq!(overlay.items[0].id, "1");
    assert_eq!(overlay.items[0].content, "write tests");
    assert_eq!(overlay.items[0].status, "in_progress");
    assert_eq!(overlay.items[0].priority, "high");
    assert_eq!(overlay.selected_index, 0);
    assert!(!overlay.dirty);
}

#[test]
fn todo_overlay_from_empty_list_preserves_absent_session() {
    let overlay = UiTodoOverlay::from_todos(None, &[]);
    assert!(overlay.session_id.is_none());
    assert!(overlay.items.is_empty());
    assert_eq!(overlay.selected_index, 0);
    assert!(!overlay.dirty);
}

#[test]
fn search_palette_task_mcp_and_memory_models_preserve_fields() {
    let search = UiSearchOverlay {
        query: "needle".to_string(),
        matches: vec![UiSearchMatch {
            message_id: Some(UiMessageId::local("search")),
            start: 2,
            end: 8,
            preview: "a needle".to_string(),
        }],
        selected_index: Some(0),
        case_sensitive: true,
    };
    assert_eq!(
        search.matches[0].message_id.as_ref().map(UiMessageId::as_str),
        Some("local:search")
    );
    assert_eq!((search.matches[0].start, search.matches[0].end), (2, 8));
    assert!(search.case_sensitive);

    let palette = UiCommandPaletteOverlay {
        query: "op".to_string(),
        items: vec![UiPaletteItem {
            id: "open".to_string(),
            label: "Open".to_string(),
            detail: Some("Open a file".to_string()),
            source: "Builtin".to_string(),
            argument_hint: Some("<path>".to_string()),
            available: true,
            unavailable_reason: None,
            replacement: "/open ".to_string(),
        }],
        selected_index: Some(0),
        first_visible_index: 0,
        visible_rows: 8,
        notice: None,
    };
    assert_eq!(palette.items[0].detail.as_deref(), Some("Open a file"));

    let task = UiTaskOverlay {
        session_id: Some("session".to_string()),
        turn_terminal: UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) },
        pending_questions: 1,
        todo_count: 2,
        sync_error: Some("offline".to_string()),
        steps: vec![UiTaskStepItem {
            message_id: UiMessageId::gateway("step"),
            step_index: 3,
            state: UiStepState::Complete,
            started_ms: 10,
            finished_ms: Some(20),
            model: Some("model".to_string()),
            finish_reason: Some("stop".to_string()),
            usage: UiTokenUsage {
                input_tokens: 1,
                output_tokens: 2,
                cached_tokens: 3,
                reasoning_tokens: 4,
            },
        }],
        selected_index: 0,
    };
    assert_eq!(task.steps[0].message_id.as_str(), "gateway:step");
    assert_eq!(
        task.turn_terminal,
        UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) }
    );
    assert_eq!(task.sync_error.as_deref(), Some("offline"));

    let mcp = UiMcpOverlay {
        servers: vec![UiMcpServerInfo {
            name: "filesystem".to_string(),
            transport: McpServerTransport::Stdio,
            address: "vw-mcp".to_string(),
        }],
        selected_index: 0,
        config_source: "project".to_string(),
    };
    assert_eq!(mcp.servers[0].transport.label(), "stdio");
    assert_eq!(mcp.config_source, "project");

    let memory = UiMemoryOverlay {
        entries: vec![UiMemoryEntry {
            scope: "project".to_string(),
            filename: "AGENTS.md".to_string(),
            path: "/work/AGENTS.md".to_string(),
            preview_lines: vec!["rule".to_string()],
            total_lines: 1,
        }],
        selected_index: 0,
    };
    assert_eq!(memory.entries[0].preview_lines, vec!["rule".to_string()]);
    assert_eq!(memory.entries[0].total_lines, 1);
}

#[test]
fn mcp_transport_labels_are_stable() {
    assert_eq!(McpServerTransport::Stdio.label(), "stdio");
    assert_eq!(McpServerTransport::Sse.label(), "sse");
    assert_eq!(McpServerTransport::Http.label(), "http");
}

fn question_overlay(
    tool: Option<question::ToolMeta>,
    header: &str,
    option_label: &str,
) -> UiQuestionOverlay {
    UiQuestionOverlay::from_request(&question::Request {
        id: "req".to_string(),
        session_id: "session".to_string(),
        questions: vec![question::Info {
            question: "Question?".to_string(),
            header: header.to_string(),
            options: vec![question::OptionInfo {
                label: option_label.to_string(),
                description: "Description".to_string(),
                preview: None,
            }],
            multiple: None,
            custom: None,
        }],
        tool,
    })
}

fn tool_meta() -> question::ToolMeta {
    question::ToolMeta { message_id: "m".to_string(), call_id: "c".to_string() }
}

fn question_overlay_with_texts(
    tool: Option<question::ToolMeta>,
    header: &str,
    question_text: &str,
    option_label: &str,
    option_description: &str,
) -> UiQuestionOverlay {
    UiQuestionOverlay::from_request(&question::Request {
        id: "req".to_string(),
        session_id: "session".to_string(),
        questions: vec![question::Info {
            question: question_text.to_string(),
            header: header.to_string(),
            options: vec![question::OptionInfo {
                label: option_label.to_string(),
                description: option_description.to_string(),
                preview: None,
            }],
            multiple: None,
            custom: None,
        }],
        tool,
    })
}
