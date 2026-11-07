//! 验证编辑器设置的主题来源、数值边界与字段行布局。
//! 跟随系统与独立主题两条路径均覆盖最小、最大配置值。

use super::*;
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;

// 默认应用提供完整主题上下文，用例仅改写编辑器设置的目标字段。
fn test_app() -> App {
    App::new().0
}

// 这些用例验证所有状态均可构建，不依赖窗口渲染后的像素结果。
fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn field_row_wraps_controls() {
    keep_element(field_row("编辑器主题", "说明", text("control")));
}

#[test]
fn view_uses_app_theme_when_following_system_theme() {
    let mut app = test_app();
    app.editor_follow_system_theme = true;
    app.app_theme = iced::Theme::Dark;
    app.editor_theme = iced::Theme::Light;
    app.current_font_size = 15.0;
    app.current_line_height = 24.0;
    app.auto_adjust_line_height = true;

    keep_element(view(&app));
}

#[test]
fn view_uses_editor_theme_when_not_following_system_theme() {
    let mut app = test_app();
    app.editor_follow_system_theme = false;
    app.app_theme = iced::Theme::Light;
    app.editor_theme = iced::Theme::Dark;
    app.current_font_size = 30.0;
    app.current_line_height = 60.0;
    app.auto_adjust_line_height = false;

    keep_element(view(&app));
}

#[test]
fn view_default_values_does_not_panic() {
    let app = test_app();
    keep_element(view(&app));
}

#[test]
fn view_with_min_boundary_values() {
    let mut app = test_app();
    app.editor_follow_system_theme = true;
    app.current_font_size = 10.0;
    app.current_line_height = 10.0;
    keep_element(view(&app));
}

#[test]
fn view_with_max_boundary_values() {
    let mut app = test_app();
    app.editor_follow_system_theme = false;
    app.current_font_size = 30.0;
    app.current_line_height = 60.0;
    keep_element(view(&app));
}

#[test]
fn view_keeps_intro_section_and_six_setting_rows() {
    let app = test_app();
    let element = view(&app);
    let tree = Tree::new(element.as_widget());

    assert_eq!(tree.children.len(), 2);
    assert_eq!(tree.children[0].children.len(), 2);
    let content = &tree.children[1];
    assert_eq!(content.children.len(), 2);
    assert_eq!(content.children[0].children.len(), 2);

    let panel_column = &content.children[1];
    assert_eq!(panel_column.children.len(), 11);
    for row_index in [0, 2, 4, 6, 8, 10] {
        let field = &panel_column.children[row_index];
        assert_eq!(field.children.len(), 2);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn metrics_slider_previews_touch_and_wheel_and_commits_on_touch_release() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, Shell, Widget, layout};
    use iced::{Event, Font, Pixels, Point, Size, keyboard, mouse, touch};

    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut slider = metrics_slider(10.0..=30.0, 14.0, |size| {
        Message::Editor(EditorMessage::FontSizePreview(size))
    });
    let mut tree = Tree::new(&slider as &dyn Widget<Message, iced::Theme, iced::Renderer>);
    let node = slider.layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(100.0, 40.0)),
    );
    let bounds = node.bounds();
    let start = Point::new(40.0, bounds.center_y());
    let end = Point::new(65.0, bounds.center_y());
    let id = touch::Finger(0);
    let mut dispatch = |event, position| {
        let mut messages = Vec::new();
        slider.update(
            &mut tree,
            &event,
            Layout::new(&node),
            mouse::Cursor::Available(position),
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut Shell::new(&mut messages),
            &bounds,
        );
        messages
    };

    let pressed =
        dispatch(Event::Touch(touch::Event::FingerPressed { id, position: start }), start);
    let moved = dispatch(Event::Touch(touch::Event::FingerMoved { id, position: end }), end);
    let released = dispatch(Event::Touch(touch::Event::FingerLifted { id, position: end }), end);
    assert!(matches!(pressed.as_slice(), [Message::Editor(EditorMessage::FontSizePreview(_))]));
    assert!(matches!(moved.as_slice(), [Message::Editor(EditorMessage::FontSizePreview(_))]));
    assert!(matches!(released.as_slice(), [Message::Editor(EditorMessage::CommitMetrics)]));

    assert!(
        dispatch(
            Event::Keyboard(keyboard::Event::ModifiersChanged(keyboard::Modifiers::CTRL)),
            end
        )
        .is_empty()
    );
    let wheel = dispatch(
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
        }),
        end,
    );
    assert!(matches!(wheel.as_slice(), [Message::Editor(EditorMessage::FontSizePreview(_))]));
}
