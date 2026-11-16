use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::advanced::{Layout, layout, mouse, renderer};
use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
#[tokio::test(flavor = "multi_thread")]
async fn text_tool_error_card_dark_chinese_detail_render_path_is_safe() {
    let mut app = desktop_session::message::chat::input::tool_detail::ToolDetailState {
        tool_detail_dialog: None,
        current_line_height: 20.0,
    };
    let full_error = format!("权限检查失败：中文错误标题\n{}", "错误详情\n".repeat(4_096));
    let visible = format!(
        "tool search\n{}",
        serde_json::json!({
            "status": "error",
            "input": "{}",
            "error": full_error,
        })
    );
    let detail = desktop_session::message::chat::input::tool_detail::tool_detail_from_raw(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &visible,
        desktop_chat_tools::tools::mcp_view::is_mcp_tool_raw(&visible),
        desktop_chat_tools::tools::workflow_view::workflow_node_detail_body,
    );
    let _task = desktop_session::message::chat::input::tool_detail::handle_open_tool_detail(
        &mut app, 11, 12, detail,
    );
    let dialog = app.tool_detail_dialog.as_ref().expect("detail dialog opens");
    assert_eq!(dialog.content.trim_end(), full_error.trim_end());
    assert!(dialog.title.contains("失败"));

    let mut element: iced::Element<'_, desktop_chat_tools::tools::detail_input::Message> =
        desktop_chat_tools::tools::tool_detail_dialog::tool_detail_dialog_view(
            desktop_chat_tools::tools::detail_input::DetailInput {
                i18n: desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
                line_height: app.current_line_height,
                dialog: Some(desktop_chat_tools::tools::detail_input::DialogInput {
                    title: &dialog.title,
                    editor: &dialog.editor,
                    editor_id: &dialog.editor_id,
                    scroll_top_line: dialog.scroll_top_line,
                    context_menu_open: dialog.context_menu_open,
                    context_menu_pos: dialog.context_menu_pos,
                }),
            },
        )
        .expect("detail dialog view");
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let bounds = Size::new(960.0, 720.0);
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
    let pixels = renderer.screenshot(Size::new(960, 720), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));

    if let Ok(path) = std::env::var("VW_TEXT_TOOL_DETAIL_SCREENSHOT") {
        image::save_buffer(path, &pixels, 960, 720, image::ColorType::Rgba8)
            .expect("save detail screenshot");
    }
}
