use super::tool_detail_dialog::tool_detail_dialog_view;
use crate::app::{App, Message, message};
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::advanced::{Layout, layout, mouse, renderer};
use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
fn app() -> App { App::new().0 }
fn tool_text_detail_message(m: usize, t: usize, v: &str) -> Message { super::card_adapter::map_card(desktop_chat_tools::card_input::Message::OpenToolDetail(m,t,v.to_owned())) }
#[tokio::test(flavor = "multi_thread")]
async fn text_tool_error_card_dark_chinese_detail_render_path_is_safe() {
    let mut app = app();
    let full_error = format!("权限检查失败：中文错误标题\n{}", "错误详情\n".repeat(4_096));
    let visible = format!(
        "tool search\n{}",
        serde_json::json!({
            "status": "error",
            "input": "{}",
            "error": full_error,
        })
    );
    let Message::Chat(open_detail) = tool_text_detail_message(11, 12, &visible) else {
        panic!("expected chat detail action");
    };

    let _task = crate::app::message::chat::update(&mut app, open_detail);
    let dialog = app.tool_detail_dialog.as_ref().expect("detail dialog opens");
    assert_eq!(dialog.content.trim_end(), full_error.trim_end());
    assert!(dialog.title.contains("失败"));

    let mut element = tool_detail_dialog_view(&app).expect("detail dialog view");
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
