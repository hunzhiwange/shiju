//! 验证共享工具渲染器对各工具类型、别名和生命周期状态的分派。
//! 通用 MCP 载荷和渲染提示可修正路由，但无法解析的工具块必须返回无视图。

use super::tool_renderer::{
    SharedToolRenderKind, render_shared_tool_view, shared_tool_render_kind,
};
use super::tool_meta::{
    tool_activity_header, tool_header_title, tool_status_badge, tool_visual_state,
};
use super::{CommandItem, tool_command_summary_view};
use crate::app::{App, Message};
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::advanced::{Layout, layout, mouse, renderer};
use iced::widget::{Space, column, container};
use iced::{Color, Font, Length, Pixels, Rectangle, Size, Theme};

fn app() -> App {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/vibe-window".to_string());
    app
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn command_status_icon_stays_next_to_command() {
    async fn status_x(viewport_width: f32) -> f32 {
        let renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("tiny-skia headless renderer should be available");
        let action = Space::new().width(Length::Fixed(22.0)).into();
        let mut element: iced::Element<'_, Message> = tool_activity_header(
            "bash",
            "运行",
            "cargo clippy --all-targets --all-features",
            tool_visual_state("completed"),
            action,
        )
        .into();
        let limits = layout::Limits::new(Size::ZERO, Size::new(viewport_width, 100.0));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);

        Layout::new(&node)
            .children()
            .nth(2)
            .expect("command status icon")
            .bounds()
            .x
    }

    let narrow = status_x(480.0).await;
    let wide = status_x(960.0).await;

    assert!((wide - narrow).abs() < 1.0, "narrow={narrow}, wide={wide}");
}

#[test]
fn shared_tool_render_kind_routes_known_tools() {
    assert_eq!(shared_tool_render_kind("tool bash\n{}"), Some(SharedToolRenderKind::Bash));
    assert_eq!(shared_tool_render_kind("tool skill\n{}"), Some(SharedToolRenderKind::Skill));
    assert_eq!(
        shared_tool_render_kind("tool workflow_node\n{}"),
        Some(SharedToolRenderKind::Workflow)
    );
    assert_eq!(shared_tool_render_kind("tool custom_tool\n{}"), Some(SharedToolRenderKind::Text));
}

#[test]
fn shared_tool_render_kind_routes_aliases_and_special_tools() {
    assert_eq!(shared_tool_render_kind("tool glob_search\n{}"), Some(SharedToolRenderKind::Files));
    assert_eq!(
        shared_tool_render_kind("tool browser_open\n{}"),
        Some(SharedToolRenderKind::Advanced)
    );
    assert_eq!(shared_tool_render_kind("tool mcp_demo\n{}"), Some(SharedToolRenderKind::Mcp));
    assert_eq!(shared_tool_render_kind("tool MCP\n{}"), Some(SharedToolRenderKind::Mcp));
    assert_eq!(shared_tool_render_kind("invalid"), None);
}

#[test]
fn shared_tool_render_kind_routes_all_supported_kinds() {
    for (raw, expected) in [
        ("tool shell\n{}", SharedToolRenderKind::Bash),
        ("tool Config\n{}", SharedToolRenderKind::Config),
        ("tool todowrite\n{}", SharedToolRenderKind::TodoCompact),
        ("tool todoread\n{}", SharedToolRenderKind::TodoRead),
        ("tool apply_patch\n{}", SharedToolRenderKind::ApplyPatch),
        ("tool file_read\n{}", SharedToolRenderKind::Read),
        ("tool pdf_read\n{}", SharedToolRenderKind::Read),
        ("tool read_file\n{}", SharedToolRenderKind::Read),
        ("tool write\n{}", SharedToolRenderKind::Files),
        ("tool file_write\n{}", SharedToolRenderKind::Files),
        ("tool file_edit\n{}", SharedToolRenderKind::Files),
        ("tool notebook_edit\n{}", SharedToolRenderKind::Files),
        ("tool glob\n{}", SharedToolRenderKind::Files),
        ("tool grep\n{}", SharedToolRenderKind::Files),
        ("tool content_search\n{}", SharedToolRenderKind::Files),
        ("tool codesearch\n{}", SharedToolRenderKind::Files),
        ("tool lsp\n{}", SharedToolRenderKind::Lsp),
        ("tool git_operations\n{}", SharedToolRenderKind::GitDiff),
        ("tool AskUserQuestion\n{}", SharedToolRenderKind::Question),
        ("tool fetch_webpage\n{}", SharedToolRenderKind::Web),
        ("tool http_request\n{}", SharedToolRenderKind::Web),
        ("tool web_search\n{}", SharedToolRenderKind::Web),
        ("tool AgentTool\n{}", SharedToolRenderKind::Advanced),
        ("tool Agent\n{}", SharedToolRenderKind::Advanced),
        ("tool browser\n{}", SharedToolRenderKind::Advanced),
        ("tool open_browser_page\n{}", SharedToolRenderKind::Advanced),
        ("tool enter_worktree\n{}", SharedToolRenderKind::Advanced),
        ("tool exit_worktree\n{}", SharedToolRenderKind::Advanced),
        ("tool task_complete\n{}", SharedToolRenderKind::Advanced),
        ("tool mcp_linear\n{}", SharedToolRenderKind::Mcp),
        ("tool custom\n{}", SharedToolRenderKind::Text),
    ] {
        assert_eq!(shared_tool_render_kind(raw), Some(expected), "{raw}");
    }
}

#[test]
fn shared_tool_render_kind_detects_mcp_from_generic_tool_payload() {
    let raw = r#"tool tools
{"output":"[{\"server\":{\"name\":\"rustcodegraph\"},\"resources\":[]}]"}"#;

    assert_eq!(shared_tool_render_kind(raw), Some(SharedToolRenderKind::Mcp));
}

#[test]
fn shared_tool_render_kind_uses_render_hint_canonical_name() {
    assert_eq!(
        shared_tool_render_kind(
            r#"tool unknown
{"renderHint":{"metadata":{"canonical_tool_id":"git_diff"}}}"#
        ),
        Some(SharedToolRenderKind::GitDiff)
    );
}

#[test]
fn render_shared_tool_view_rejects_unparseable_tool_blocks() {
    let app = app();

    assert!(render_shared_tool_view(&app, 0, 0, "tool bash").is_none());
    assert!(render_shared_tool_view(&app, 0, 0, "bash\n{}").is_none());
    assert!(render_shared_tool_view(&app, 0, 0, "tool \n{}").is_none());
}

#[test]
fn render_shared_tool_view_dispatches_each_tool_kind() {
    let app = app();

    for (idx, visible) in [
        (
            0,
            r#"tool bash
{"input":"echo ok","output":"ok"}"#,
        ),
        (
            2,
            r#"tool config
{"status":"completed","result":{"data":{"success":true,"operation":"get","setting":"model","value":"gpt-5"}}}"#,
        ),
        (
            3,
            r#"tool todowrite
{"status":"completed","input":"{\"todos\":[{\"status\":\"pending\",\"content\":\"todo\"}]}"}"#,
        ),
        (
            4,
            r#"tool todoread
{"status":"completed","output":"[]"}"#,
        ),
        (
            5,
            r#"tool apply_patch
{"status":"running","input":"*** Begin Patch\n*** End Patch"}"#,
        ),
        (
            6,
            r#"tool read
{"input":"{\"filePath\":\"src/main.rs\",\"offset\":1,\"limit\":2}"}"#,
        ),
        (
            7,
            r#"tool write
{"input":"{\"path\":\"src/main.rs\",\"content\":\"fn main() {}\"}","status":"completed"}"#,
        ),
        (
            8,
            r#"tool lsp
{"status":"completed","output":"plain lsp output"}"#,
        ),
        (
            9,
            r#"tool git_diff
{"input":"{}","status":"completed","result":{"data":{"hunks":[{"file":"src/main.rs","lines":[{"text":"+new"}]}]}}}"#,
        ),
        (
            10,
            r#"tool question
{"status":"completed","summary":"finished"}"#,
        ),
        (
            11,
            r#"tool web_fetch
{"status":"completed","summary":"Fetched","output":"body"}"#,
        ),
        (
            13,
            r#"tool AgentTool
{"input":"{\"agent\":\"reviewer\",\"prompt\":\"inspect the patch\"}","result":{"data":{"message":"done","agent":"reviewer","session_id":"s1"}}}"#,
        ),
        (
            14,
            r#"tool skill
{"status":"completed","input":"{\"name\":\"imagegen\"}","output":"ok"}"#,
        ),
        (
            15,
            r#"tool workflow_node
{"status":"completed","metadata":{"title":"Node","node_type":"task"},"output":"ok"}"#,
        ),
        (
            16,
            r#"tool custom
{"status":"completed","output":"custom output"}"#,
        ),
        (
            18,
            r#"tool mcp_linear
{"status":"completed","summary":"Fetched resources","output":"[]"}"#,
        ),
    ] {
        let element = render_shared_tool_view(&app, 9, idx, visible)
            .unwrap_or_else(|| panic!("expected shared tool view for {idx}"));
        keep_element(element);
    }
}

#[test]
fn shared_tool_headers_build_for_all_lifecycle_states() {
    let app = app();

    for (idx, status) in ["waiting", "running", "completed", "failed", "cancelled"]
        .into_iter()
        .enumerate()
    {
        let visible = format!(
            "tool custom\n{}",
            serde_json::json!({
                "status": status,
                "input": "{\"query\":\"中文摘要\"}",
                "output": "detail",
                "error": (status == "failed").then_some("failed"),
            })
        );
        keep_element(
            render_shared_tool_view(&app, 20, idx, &visible)
                .unwrap_or_else(|| panic!("expected {status} tool view")),
        );

        for (tool_idx, visible) in [
            (
                100 + idx,
                format!(
                    "tool config\n{}",
                    serde_json::json!({
                        "status": status,
                        "input": "{\"setting\":\"theme\"}",
                        "result": {"data": {
                            "success": status != "failed",
                            "operation": "get",
                            "setting": "theme",
                            "value": "dark",
                            "error": (status == "failed").then_some("failed"),
                        }},
                    })
                ),
            ),
            (
                200 + idx,
                format!(
                    "tool grep\n{}",
                    serde_json::json!({
                        "status": status,
                        "input": "{\"pattern\":\"工具\",\"path\":\"src\"}",
                        "output": "- src/main.rs",
                    })
                ),
            ),
            (
                300 + idx,
                format!(
                    "tool lsp\n{}",
                    serde_json::json!({
                        "status": status,
                        "input": "{\"query\":\"工具\"}",
                        "output": "plain lsp output",
                        "error": (status == "failed").then_some("failed"),
                    })
                ),
            ),
        ] {
            keep_element(
                render_shared_tool_view(&app, 20, tool_idx, &visible)
                    .unwrap_or_else(|| panic!("expected {status} basic tool view: {visible}")),
            );
        }

        keep_element(
            tool_command_summary_view(
                &app,
                20,
                &[CommandItem {
                    tool_idx: 400 + idx,
                    raw: format!(
                        "tool bash\n{}",
                        serde_json::json!({"status": status, "input": "printf 中文"})
                    ),
                }],
            )
            .unwrap_or_else(|| panic!("expected {status} command summary")),
        );

        let visual = tool_visual_state(status);
        keep_element(tool_header_title("custom", "工具 custom", visual));
        keep_element(tool_status_badge(visual));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn shared_tool_headers_dark_chinese_render_path_is_safe() {
    let mut app = app();
    app.chat_tool_hovered_idx = Some((30_u64 << 32) | 3);
    let payloads = [
        r#"tool bash
{"status":"running","input":"{\"command\":\"cargo test --package vw-desktop -- 中文长命令摘要\"}"}"#,
        r#"tool grep
{"status":"waiting","input":"{\"pattern\":\"工具活动行\",\"path\":\"crates/vw-desktop/src\"}","output":"- crates/vw-desktop/src/app.rs"}"#,
        r#"tool config
{"status":"completed","input":"{\"setting\":\"theme\"}","result":{"data":{"success":true,"operation":"get","setting":"theme","value":"dark"}}}"#,
        r#"tool lsp
{"status":"failed","input":"{\"query\":\"中文符号\"}","error":"诊断请求失败"}"#,
        r#"tool file_write
{"status":"cancelled","input":"{\"path\":\"src/中文路径.rs\",\"content\":\"\"}"}"#,
    ];

    let mut rows = column![].spacing(12).width(Length::Fill);
    for (tool_idx, payload) in payloads.iter().enumerate() {
        rows = rows.push(
            render_shared_tool_view(&app, 30, tool_idx, payload)
                .unwrap_or_else(|| panic!("expected renderable payload: {payload}")),
        );
    }

    let mut element: iced::Element<'_, Message> =
        container(rows).padding(24).width(Length::Fill).into();
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let bounds = Size::new(960.0, 640.0);
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
    let pixels = renderer.screenshot(Size::new(960, 640), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));

    if let Ok(path) = std::env::var("VW_TOOL_HEADERS_SCREENSHOT") {
        image::save_buffer(path, &pixels, 960, 640, image::ColorType::Rgba8)
            .expect("save tool header screenshot");
    }
}
