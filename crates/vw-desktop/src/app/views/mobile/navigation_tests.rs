use super::*;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size};

#[test]
fn selected_session_has_visible_background_in_both_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        assert_ne!(
            list_style(&theme, button::Status::Active, true).background,
            list_style(&theme, button::Status::Active, false).background
        );
        assert_eq!(
            list_style(&theme, button::Status::Active, true).text_color,
            theme.palette().text
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn session_list_rows_draw_chinese_and_route_selection() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    for theme in [Theme::Light, Theme::Dark] {
        for selected in [false, true] {
            for width in [280.0, 400.0] {
                let mut element = session_row(
                    "使用 shiju 技能剪辑视频：项目会话标题很长时仍应完整可读",
                    ViewMessage::MobileSessionSelected("/project".into(), "session".into()),
                    selected,
                );
                let mut tree = Tree::new(element.as_widget());
                let viewport = Rectangle::with_size(Size::new(width, 300.0));
                let node = element.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(Size::ZERO, viewport.size()),
                );
                assert_eq!(node.size().width, width);
                assert!(node.size().height >= 44.0);
                let cursor = mouse::Cursor::Available(Point::new(20.0, 20.0));
                element.as_widget().draw(
                    &tree,
                    &mut renderer,
                    &theme,
                    &renderer::Style::default(),
                    Layout::new(&node),
                    cursor,
                    &viewport,
                );
                let mut messages = Vec::new();
                for event in [
                    mouse::Event::ButtonPressed(mouse::Button::Left),
                    mouse::Event::ButtonReleased(mouse::Button::Left),
                ] {
                    element.as_widget_mut().update(
                        &mut tree,
                        &Event::Mouse(event),
                        Layout::new(&node),
                        cursor,
                        &renderer,
                        &mut clipboard::Null,
                        &mut Shell::new(&mut messages),
                        &viewport,
                    );
                }
                assert!(
                    matches!(messages.as_slice(), [Message::View(ViewMessage::MobileSessionSelected(path, id))] if path == "/project" && id == "session")
                );
            }
        }
    }
}
