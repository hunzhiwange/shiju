use crate::input_panel::{send_controls as controls, send_message::Message};
use desktop_session::message::chat::request::ChatSendBehavior;
use iced::Element;
struct Fixture {
    chat_send_behavior: ChatSendBehavior,
    files: Vec<String>,
}
impl Default for Fixture {
    fn default() -> Self {
        Self { chat_send_behavior: ChatSendBehavior::Queue, files: Vec::new() }
    }
}
fn send_button(
    app: &Fixture,
    enabled: bool,
    can_send: bool,
    requesting: bool,
) -> Element<'static, Message> {
    controls::send_button(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        app.chat_send_behavior,
        false,
        true,
        enabled,
        can_send,
        requesting,
    )
}
pub(crate) async fn click_control<'a, M: Clone + 'a>(mut element: Element<'a, M>) -> Vec<M> {
    use iced::advanced::{Layout, Shell, layout, renderer::Headless, widget::Tree};
    let renderer = <iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.0),
        Some("tiny-skia"),
    )
    .await
    .expect("headless renderer");
    let mut tree = Tree::new(element.as_widget());
    let size = iced::Size::new(400.0, 80.0);
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(iced::Size::ZERO, size),
    );
    let mut messages = Vec::new();
    for event in [
        iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left),
        iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left),
    ] {
        element.as_widget_mut().update(
            &mut tree,
            &iced::Event::Mouse(event),
            Layout::new(&node),
            iced::mouse::Cursor::Available(iced::Point::new(12.0, 12.0)),
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut Shell::new(&mut messages),
            &iced::Rectangle::with_size(size),
        );
    }
    messages
}

#[tokio::test]
async fn voice_ui_primary_empty_opens_chat() {
    let app = Fixture::default();
    // 同一编辑器依次输入空白、中文，再删除到空；与 view 一样读取 Content。
    for value in ["", "  \n", "中文", ""] {
        let editor: iced::widget::text_editor::Content =
            iced::widget::text_editor::Content::with_text(value);
        let enabled = !editor.text().trim().is_empty();
        let messages = click_control(send_button(&app, enabled, enabled, false)).await;
        if enabled {
            assert!(messages.iter().any(|m| matches!(m, Message::SendPressed)));
        } else {
            assert!(messages.iter().any(|m| matches!(m, Message::VoiceChatStart)));
        }
    }
}
#[tokio::test]
async fn voice_ui_primary_text_sends() {
    let app = Fixture::default();
    let messages = click_control(send_button(&app, true, true, false)).await;
    assert!(messages.iter().any(|m| matches!(m, Message::SendPressed)));
}
#[tokio::test]
async fn voice_ui_primary_attachment_sends() {
    let mut app = Fixture::default();
    app.files.push(String::from("fixture.txt"));
    let enabled = !app.files.is_empty();
    let messages = click_control(send_button(&app, enabled, enabled, false)).await;
    assert!(matches!(messages.as_slice(), [Message::SendPressed]));
}
#[tokio::test]
async fn voice_ui_primary_busy_preserves_send_behavior() {
    for behavior in
        [ChatSendBehavior::Queue, ChatSendBehavior::StopAndSend, ChatSendBehavior::Guide]
    {
        let mut app = Fixture::default();
        app.chat_send_behavior = behavior;
        let messages = click_control(send_button(&app, true, true, true)).await;
        assert!(messages.iter().any(|m| matches!(m, Message::SendPressed)));
        assert!(click_control(send_button(&app, false, false, true)).await.is_empty());
    }
}
