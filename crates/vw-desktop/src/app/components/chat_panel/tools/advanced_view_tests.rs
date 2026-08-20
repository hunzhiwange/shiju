use super::advanced_view::{
    advanced_base_title, advanced_fallback_body, advanced_metadata_text, advanced_success_body,
    advanced_summary, agent_tool_detail_message, agent_tool_primary_message, agent_tool_session_id,
    compact_browser_success_summary, format_browser_success_body, is_advanced_surface_tool,
    nested_string,
    result_data_message, tool_advanced_view,
};
use super::apply_patch_view::tool_apply_patch_view;
use super::git_diff_view::tool_git_diff_view;
use super::mcp_view::tool_mcp_view;
use super::question_view::tool_question_view;
use super::skill_view::tool_skill_view;
use super::todo_views::tool_todowrite_compact_view;
use super::tool_renderer::render_shared_tool_view;
use super::web_view::tool_web_view;
use super::workflow_view::tool_workflow_view;
use crate::app::state::explicit_advanced_tool_surface_spec;
use crate::app::{App, Message, message};
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::advanced::{Layout, layout, mouse, renderer};
use iced::widget::{column, container};
use iced::{Color, Font, Length, Pixels, Rectangle, Size, Theme};
use serde_json::json;

#[test]
fn advanced_surface_tool_accepts_known_tools_and_mcp_prefixes() {
    assert!(is_advanced_surface_tool("AgentTool"));
    assert!(is_advanced_surface_tool("Agent"));
    assert!(is_advanced_surface_tool("browser_open"));
    assert!(is_advanced_surface_tool("open_browser_page"));
    assert!(is_advanced_surface_tool("mcp_linear"));
    assert!(!is_advanced_surface_tool("bash"));
}

#[test]
fn nested_string_walks_object_path() {
    let value = json!({"result":{"data":{"message":"done"}}});
    assert_eq!(nested_string(&value, &["result", "data", "message"]), Some("done"));
    assert_eq!(nested_string(&value, &["result", "missing"]), None);
    assert_eq!(nested_string(&json!({"result":{"message":"   "}}), &["result", "message"]), None);
}

#[test]
fn result_data_message_reads_nested_message() {
    assert_eq!(
        result_data_message(&json!({"result":{"data":{"message":"ok"}}})),
        Some("ok".to_string())
    );
}

#[test]
fn advanced_base_title_uses_explicit_and_builtin_labels() {
    assert_eq!(advanced_base_title("Agent", None), "AgentTool");
    assert_eq!(advanced_base_title("browser", None), "浏览器");
    assert_eq!(advanced_base_title("browser_open", None), "打开页面");
    assert_eq!(advanced_base_title("read_file", None), "读取");
    assert_eq!(
        advanced_base_title("mcp_linear", explicit_advanced_tool_surface_spec("mcp_linear")),
        "mcp_* 集成"
    );
}

#[test]
fn advanced_summary_prefers_summary_and_formats_inputs() {
    assert_eq!(advanced_summary("AgentTool", &json!({"summary":" ready "})), "ready");
    assert_eq!(
        advanced_summary(
            "AgentTool",
            &json!({"input":"{\"agent\":\"reviewer\",\"prompt\":\"inspect patch\"}"})
        ),
        "reviewer · inspect patch"
    );
    assert_eq!(
        advanced_summary(
            "browser",
            &json!({"input":"{\"action\":\"click\",\"selector\":\"#save\"}"})
        ),
        "click · #save"
    );
    assert_eq!(
        advanced_summary(
            "browser",
            &json!({"input":"{\"action\":\"open\",\"url\":\"local-page\"}"})
        ),
        "open · local-page"
    );
    assert_eq!(advanced_summary("browser", &json!({"input":"not-json"})), "not-json");
    assert_eq!(
        advanced_summary("browser_open", &json!({"input":"{\"url\":\"https://example.test\"}"})),
        "https://example.test"
    );
    assert_eq!(advanced_summary("unknown", &json!({"input":"{}"})), "");
}

#[test]
fn advanced_summary_handles_partial_agent_and_default_browser_inputs() {
    assert_eq!(
        advanced_summary("AgentTool", &json!({"input":"{\"agent\":\"reviewer\"}"})),
        "调用 reviewer"
    );
    assert_eq!(
        advanced_summary("Agent", &json!({"input":"{\"task\":\"inspect patch\"}"})),
        "inspect patch"
    );
    assert_eq!(advanced_summary("Agent", &json!({"input":"{}"})), "");
    assert_eq!(advanced_summary("browser", &json!({"input":"{}"})), "browser");
}

#[test]
fn advanced_metadata_text_formats_tool_specific_parts() {
    assert_eq!(
        advanced_metadata_text(
            "AgentTool",
            &json!({"result":{"data":{"agent":" reviewer ","session_id":" s1 "}}}),
            None
        ),
        "reviewer · session s1"
    );
    assert_eq!(
        advanced_metadata_text(
            "browser",
            &json!({
                "renderHint":{"metadata":{"action":" open ","backend":" playwright "}},
                "result":{"data":{"result":{"url":"local-page"}}}
            }),
            None
        ),
        "open · playwright · local-page"
    );
    assert_eq!(
        advanced_metadata_text(
            "browser",
            &json!({"result":{"data":{"result":{"title":"Page title"}}}}),
            None
        ),
        "Page title"
    );
    assert_eq!(
        advanced_metadata_text(
            "browser_open",
            &json!({"renderHint":{"metadata":{"browser":"Chrome"}},"result":{"data":{"url":"local-page"}}}),
            None
        ),
        "Chrome · local-page"
    );
    assert_eq!(
        advanced_metadata_text(
            "open_browser_page",
            &json!({"renderHint":{"metadata":{"url":"https://example.test"}}}),
            None
        ),
        "https://example.test"
    );
    assert_eq!(advanced_metadata_text("unknown", &json!({}), None), "");
    assert_eq!(
        advanced_metadata_text(
            "mcp_linear",
            &json!({}),
            explicit_advanced_tool_surface_spec("mcp_linear")
        ),
        "状态: planned"
    );
}

#[test]
fn advanced_fallback_body_formats_explicit_and_agent_defaults() {
    assert_eq!(
        advanced_fallback_body(
            "mcp_linear",
            &json!({}),
            explicit_advanced_tool_surface_spec("mcp_linear")
        ),
        "mcp_* 集成 当前已明确标记为 planned，桌面端先提供清晰状态，不补完整后端。"
    );
    assert_eq!(
        advanced_fallback_body("AgentTool", &json!({"result":{"data":{"message":"done"}}}), None),
        "done"
    );
    assert_eq!(advanced_fallback_body("browser", &json!({}), None), "");
}

#[test]
fn advanced_success_body_formats_agent_browser_and_plan_results() {
    assert_eq!(advanced_success_body("AgentTool", r#"{"message":" done "}"#, &json!({})), "done");
    assert_eq!(
        advanced_success_body(
            "AgentTool",
            "raw",
            &json!({"result":{"data":{"message":"from result"}}})
        ),
        "from result"
    );
    assert_eq!(advanced_success_body("AgentTool", "raw", &json!({})), "raw");
    assert_eq!(
        advanced_success_body(
            "browser",
            "raw",
            &json!({"result":{"data":{"result":{"message":"loaded"}}}})
        ),
        "loaded"
    );
    assert_eq!(advanced_success_body("unknown", "raw", &json!({})), "raw");
}

#[test]
fn format_browser_success_body_covers_open_and_result_shapes() {
    assert_eq!(
        format_browser_success_body(
            "browser_open",
            &json!({"result":{"data":{"browser":"Chrome"}}})
        ),
        Some("已在 Chrome 中打开页面。".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            "browser_open",
            &json!({"result":{"data":{"url":"local-page"}}})
        ),
        Some("已打开 local-page".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            "open_browser_page",
            &json!({"result":{"data":{"browser":"Chrome","url":"local-page"}}})
        ),
        Some("已在 Chrome 中打开 local-page".to_string())
    );
    assert_eq!(format_browser_success_body("browser_open", &json!({"result":{"data":{}}})), None);
    assert_eq!(format_browser_success_body("browser", &json!({})), None);
    assert_eq!(
        format_browser_success_body(
            "browser",
            &json!({"result":{"data":{"result":{"text":"hello"}}}})
        ),
        Some("hello".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            "browser",
            &json!({"result":{"data":{"result":{"path":"/tmp/out"}}}})
        ),
        Some("结果路径: /tmp/out".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            "browser",
            &json!({"result":{"data":{"result":{"title":"Title","url":"local-page"}}}})
        ),
        Some("Title\nlocal-page".to_string())
    );
    assert_eq!(
        format_browser_success_body("browser", &json!({"result":{"data":{"result":{"value":1}}}})),
        Some("{\n  \"value\": 1\n}".to_string())
    );
}

#[test]
fn format_browser_success_body_uses_single_title_or_url() {
    assert_eq!(
        format_browser_success_body(
            "browser",
            &json!({"result":{"data":{"result":{"title":"Only title"}}}})
        ),
        Some("Only title".to_string())
    );
    assert_eq!(
        format_browser_success_body(
            "browser",
            &json!({"result":{"data":{"result":{"url":"local-page"}}}})
        ),
        Some("local-page".to_string())
    );
    assert_eq!(result_data_message(&json!({"result":{"data":{"message":"   "}}})), None);
}

#[test]
fn compact_browser_success_summary_merges_multiline_snapshot_text() {
    assert_eq!(
        compact_browser_success_summary("Captured page snapshot\n\nsnapshot · browser\n  Get text"),
        "Captured page snapshot snapshot · browser Get text"
    );
}

#[test]
fn advanced_view_rejects_unknown_tools_and_invalid_json() {
    let app = App::new().0;

    assert!(tool_advanced_view(&app, 0, 0, "tool browser").is_none());
    assert!(tool_advanced_view(&app, 0, 0, "browser\n{}").is_none());
    assert!(tool_advanced_view(&app, 0, 0, "tool bash\n{}").is_none());
    assert!(tool_advanced_view(&app, 0, 0, "tool browser\nnot-json").is_none());
    assert!(tool_advanced_view(&app, 0, 0, "tool AgentTool\n{}").is_none());
}

#[test]
fn advanced_view_renders_agent_browser_and_errors() {
    let mut app = App::new().0;
    app.chat_tool_hovered_idx = Some((4_u64 << 32) | 2);

    assert!(
        tool_advanced_view(
            &app,
            4,
            2,
            r#"tool AgentTool
{"input":"{\"agent\":\"reviewer\",\"prompt\":\"inspect the patch\"}","result":{"data":{"message":"done","agent":"reviewer","session_id":"s1"}}}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            4,
            2,
            r#"tool browser
{"renderHint":{"metadata":{"action":"open","backend":"playwright"}},"result":{"data":{"result":{"title":"Example","url":"https://example.com"}}}}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            4,
            2,
            r#"tool browser
{"result":{"data":{"result":{"text":"Captured page snapshot\n\nsnapshot · browser\nGet text"}}}}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            4,
            2,
            r#"tool browser
{"status":"error","error":"navigation failed"}"#
        )
        .is_some()
    );
}

#[test]
fn advanced_view_renders_running_and_planned_explicit_surfaces() {
    let app = App::new().0;

    assert!(
        tool_advanced_view(
            &app,
            0,
            0,
            r#"tool mcp_linear
{"input":"{}"}"#
        )
        .is_some()
    );
}

#[test]
fn advanced_view_renders_empty_body_context_menu_and_browser_open() {
    let mut app = App::new().0;
    app.chat_context_menu_target =
        Some(crate::app::components::chat_panel::utils::chat_context_target_key(1, Some(3)));
    app.chat_context_menu_pos = Some((24.0, 32.0));

    assert!(
        tool_advanced_view(
            &app,
            1,
            3,
            r#"tool browser
{"summary":"metadata only","result":{"data":{"result":{"title":"Only title"}}}}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            1,
            3,
            r#"tool browser_open
{"output":"opened","result":{"data":{"browser":"Chrome","url":"local-page"}}}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            1,
            3,
            r#"tool open_browser_page
{"result":{"success":false,"data":{"browser":"Chrome"}},"error":"denied by user"}"#
        )
        .is_some()
    );
}

#[test]
fn agent_tool_session_id_supports_running_and_completed_payloads() {
    assert_eq!(
        agent_tool_session_id(&json!({
            "status": "running",
            "metadata": {"child_session_id": " child-running "}
        }))
        .as_deref(),
        Some("child-running")
    );
    assert_eq!(
        agent_tool_session_id(&json!({
            "status": "completed",
            "result": {"data": {"session_id": " child-done "}}
        }))
        .as_deref(),
        Some("child-done")
    );
    assert_eq!(agent_tool_session_id(&json!({})), None);
}

#[test]
fn agent_tool_primary_click_opens_child_and_safely_falls_back_to_detail() {
    let visible = "tool AgentTool\n{\"status\":\"running\"}";
    let value = json!({
        "status": "running",
        "metadata": {"child_session_id": "child-1"}
    });

    let message = agent_tool_primary_message(Some("/work/project"), &value, 2, 3, visible);
    assert!(matches!(
        message,
        Message::Chat(message::ChatMessage::OpenAgentSession {
            ref project_path,
            ref session_id,
        }) if project_path == "/work/project" && session_id == "child-1"
    ));

    for project_path in [None, Some(""), Some("   ")] {
        let message = agent_tool_primary_message(project_path, &value, 2, 3, visible);
        assert!(matches!(
            message,
            Message::Chat(message::ChatMessage::OpenToolDetail(2, 3, ref raw))
                if raw == visible
        ));
    }
}

#[test]
fn agent_tool_right_click_opens_full_result_detail() {
    let visible = "tool AgentTool\n{\"status\":\"completed\",\"output\":\"done\"}";
    assert!(matches!(
        agent_tool_detail_message(4, 5, visible),
        Message::Chat(message::ChatMessage::OpenToolDetail(4, 5, ref raw))
            if raw == visible
    ));
}

#[test]
fn advanced_view_renders_agent_tool_compact_cards_for_all_states() {
    let mut app = App::new().0;
    app.project_path = Some("/work/project".to_string());

    for (index, raw) in [
        (
            0,
            r#"tool AgentTool
{"status":"running","input":"{\"agent\":\"coder\",\"task\":\"分析文件\"}","metadata":{"child_session_id":"child-running"}}"#,
        ),
        (
            1,
            r#"tool AgentTool
{"status":"completed","input":"{\"agent\":\"coder\",\"task\":\"分析文件\"}","output":"完整返回结果","result":{"data":{"session_id":"child-done"}}}"#,
        ),
        (
            2,
            r#"tool AgentTool
{"status":"error","input":"{\"agent\":\"coder\",\"task\":\"分析文件\"}","error":"执行失败","metadata":{"child_session_id":"child-failed"}}"#,
        ),
        (
            3,
            r#"tool AgentTool
{"status":"waiting","input":"{\"agent\":\"coder\",\"task\":\"分析文件\"}","metadata":{"child_session_id":"child-waiting"}}"#,
        ),
        (
            4,
            r#"tool AgentTool
{"status":"cancelled","input":"{\"agent\":\"coder\",\"task\":\"分析文件\"}","metadata":{"child_session_id":"child-cancelled"}}"#,
        ),
    ] {
        assert!(
            tool_advanced_view(&app, 8, index, raw).is_some(),
            "expected compact AgentTool card for state {index}"
        );
    }
}

#[test]
fn specialized_tool_headers_use_shared_lifecycle_contract() {
    for (name, source) in [
        ("advanced", include_str!("advanced_view.rs")),
        ("apply_patch", include_str!("apply_patch_view.rs")),
        ("git_diff", include_str!("git_diff_view.rs")),
        ("mcp", include_str!("mcp_view.rs")),
        ("question", include_str!("question_view.rs")),
        ("skill", include_str!("skill_view.rs")),
        ("todo", include_str!("todo_views.rs")),
        ("web", include_str!("web_view.rs")),
        ("workflow", include_str!("workflow_view.rs")),
    ] {
        assert!(
            source.contains("tool_activity_header("),
            "{name} must render its lifecycle through the shared activity header"
        );
    }

    let app = App::new().0;
    let fixtures = [
        tool_advanced_view(
            &app,
            20,
            0,
            r#"tool AgentTool
{"status":"waiting","input":"{\"agent\":\"reviewer\",\"task\":\"检查变更\"}"}"#,
        ),
        tool_apply_patch_view(
            &app,
            20,
            1,
            r#"tool apply_patch
{"status":"running","input":"*** Begin Patch\n*** End Patch"}"#,
        ),
        tool_git_diff_view(
            &app,
            20,
            2,
            r#"tool git_diff
{"status":"running","input":"{}"}"#,
        ),
        tool_mcp_view(
            &app,
            20,
            3,
            r#"tool mcp_linear
{"status":"completed","summary":"Linear"}"#,
        ),
        tool_question_view(
            &app,
            20,
            4,
            r#"tool question
{"status":"completed","input":"{\"questions\":[{\"header\":\"范围\",\"question\":\"继续吗？\",\"options\":[]}]}","output":"{}"}"#,
        ),
        tool_skill_view(
            &app,
            20,
            5,
            r#"tool skill
{"status":"failed","input":"{\"name\":\"review\"}","error":"加载失败"}"#,
        ),
        tool_todowrite_compact_view(
            &app,
            20,
            6,
            r#"tool todowrite
{"status":"cancelled","input":"{\"todos\":[]}"}"#,
        ),
        tool_web_view(
            &app,
            20,
            7,
            r#"tool web_search
{"status":"denied","summary":"查询资料","error":"permission denied","permission":{"state":"denied"}}"#,
        ),
        tool_workflow_view(
            &app,
            20,
            8,
            r#"tool workflow_node
{"status":"cancelled","metadata":{"title":"整理结果"}}"#,
        ),
    ];

    for (index, element) in fixtures.into_iter().enumerate() {
        assert!(element.is_some(), "specialized fixture {index} must render");
    }
}

#[test]
fn specialized_tool_actions_survive_header_migration() {
    let agent_visible = "tool AgentTool\n{\"status\":\"running\"}";
    let agent_value = json!({
        "status": "running",
        "metadata": {"child_session_id": "child-activity-row"}
    });
    assert!(matches!(
        agent_tool_primary_message(Some("/work/project"), &agent_value, 3, 4, agent_visible),
        Message::Chat(message::ChatMessage::OpenAgentSession {
            ref project_path,
            ref session_id,
        }) if project_path == "/work/project" && session_id == "child-activity-row"
    ));
    assert!(matches!(
        agent_tool_detail_message(3, 4, agent_visible),
        Message::Chat(message::ChatMessage::OpenToolDetail(3, 4, ref raw))
            if raw == agent_visible
    ));
}

#[test]
fn legacy_boolean_tool_header_has_no_callers() {
    assert!(!include_str!("tool_meta.rs").contains("impl From<bool> for ToolVisual"));

    for (name, source) in [
        ("advanced", include_str!("advanced_view.rs")),
        ("apply_patch", include_str!("apply_patch_view.rs")),
        ("git_diff", include_str!("git_diff_view.rs")),
        ("mcp", include_str!("mcp_view.rs")),
        ("question", include_str!("question_view.rs")),
        ("skill", include_str!("skill_view.rs")),
        ("todo", include_str!("todo_views.rs")),
        ("web", include_str!("web_view.rs")),
        ("workflow", include_str!("workflow_view.rs")),
    ] {
        assert!(
            !source.contains("tool_header_title("),
            "{name} still calls the legacy title-only header"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn specialized_tool_headers_dark_permission_and_failure_render_path_is_safe() {
    let mut app = App::new().0;
    app.project_path = Some("/work/project".to_string());
    app.chat_tool_hovered_idx = Some((40_u64 << 32) | 3);
    let payloads = [
        r#"tool AgentTool
{"status":"waiting","input":"{\"agent\":\"reviewer\",\"task\":\"检查中文界面\"}","metadata":{"child_session_id":"child-1"}}"#,
        r#"tool apply_patch
{"status":"denied","input":"*** Begin Patch\n*** Update File: src/界面.rs\n*** End Patch","error":"用户拒绝补丁","permission":{"state":"denied"}}"#,
        r#"tool git_diff
{"status":"running","input":"{}"}"#,
        r#"tool mcp_linear
{"status":"completed","summary":"Linear 项目"}"#,
        r#"tool question
{"status":"completed","input":"{\"questions\":[{\"header\":\"范围\",\"question\":\"继续执行吗？\",\"options\":[]}]}","output":"{}"}"#,
        r#"tool skill
{"status":"failed","input":"{\"name\":\"review\"}","error":"技能加载失败"}"#,
        r#"tool todowrite
{"status":"cancelled","input":"{\"todos\":[{\"status\":\"pending\",\"content\":\"检查状态槽\"}]}"}"#,
        r#"tool web_search
{"status":"error","summary":"搜索工具活动行","error":"approval required before web search"}"#,
        r#"tool workflow_node
{"status":"completed","output":"{\"answer\":\"已整理\"}","metadata":{"title":"整理结果","workflow_yaml":"nodes: []"}}"#,
    ];

    let mut rows = column![].spacing(12).width(Length::Fill);
    for (tool_idx, payload) in payloads.iter().enumerate() {
        rows = rows.push(
            render_shared_tool_view(&app, 40, tool_idx, payload)
                .unwrap_or_else(|| panic!("expected specialized payload: {payload}")),
        );
    }

    let mut element: iced::Element<'_, Message> =
        container(rows).padding(24).width(Length::Fill).into();
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let bounds = Size::new(1120.0, 900.0);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, bounds),
    );
    let viewport = Rectangle::with_size(bounds);
    element.as_widget().draw(
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
    let pixels = renderer.screenshot(Size::new(1120, 900), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));

    if let Ok(path) = std::env::var("VW_SPECIALIZED_TOOL_HEADERS_SCREENSHOT") {
        image::save_buffer(path, &pixels, 1120, 900, image::ColorType::Rgba8)
            .expect("save specialized tool header screenshot");
    }
}
