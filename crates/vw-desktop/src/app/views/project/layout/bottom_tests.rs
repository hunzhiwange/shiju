use crate::app::{App, Message};
fn test_app() -> App {
    let (app, _task) = App::new();
    app
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn bottom_panel_builds_empty_space_when_terminal_hidden() {
    let mut app = test_app();
    app.terminal.is_visible = false;

    keep_element(super::bottom_panel(&app));
}

#[test]
fn bottom_panel_builds_terminal_when_visible() {
    let mut app = test_app();
    app.terminal.is_visible = true;
    app.terminal.height = 240.0;

    keep_element(super::bottom_panel(&app));
}

#[test]
fn bottom_panel_clamps_negative_visible_terminal_height() {
    let mut app = test_app();
    app.terminal.is_visible = true;
    app.terminal.height = -24.0;

    keep_element(super::bottom_panel(&app));
}

#[test]
fn bottom_panel_builds_terminal_at_zero_height() {
    let mut app = test_app();
    app.terminal.is_visible = true;
    app.terminal.height = 0.0;

    keep_element(super::bottom_panel(&app));
}

#[test]
fn bottom_panel_builds_terminal_at_exactly_handle_height() {
    let mut app = test_app();
    app.terminal.is_visible = true;
    const HIT_HEIGHT: f32 = crate::app::views::project::handles::VResizeHandle::HIT_HEIGHT;
    app.terminal.height = HIT_HEIGHT;

    keep_element(super::bottom_panel(&app));
}

#[test]
fn bottom_panel_builds_terminal_at_large_height() {
    let mut app = test_app();
    app.terminal.is_visible = true;
    app.terminal.height = 4096.0;

    keep_element(super::bottom_panel(&app));
}

async fn terminal_resize_probe(
    y: f32,
) -> (Vec<Message>, iced::mouse::Interaction, Vec<iced::Rectangle>) {
    use iced::advanced::{Layout, Shell, clipboard, layout, renderer::Headless, widget::Tree};
    use iced::{Event, Font, Pixels, Point, Rectangle, Size, mouse};
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia renderer");
    let mut app = test_app();
    app.terminal.is_visible = true;
    app.terminal.height = 240.0;
    let mut element = super::bottom_panel(&app);
    let mut tree = Tree::new(element.as_widget());
    let viewport = Rectangle::with_size(Size::new(640.0, 480.0));
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, viewport.size()),
    );
    let layout = Layout::new(&node);
    let cursor = mouse::Cursor::Available(Point::new(100.0, y));
    let interaction =
        element.as_widget().mouse_interaction(&tree, layout, cursor, &viewport, &renderer);
    let mut messages = Vec::new();
    element.as_widget_mut().update(
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        layout,
        cursor,
        &renderer,
        &mut clipboard::Null,
        &mut Shell::new(&mut messages),
        &viewport,
    );
    let children = layout
        .children()
        .next()
        .expect("terminal column")
        .children()
        .map(|child| child.bounds())
        .collect();
    (messages, interaction, children)
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_resize_inner_edges_start_drag_and_show_resize_cursor() {
    for y in [0.5, 7.5] {
        let (messages, interaction, _) = terminal_resize_probe(y).await;
        assert!(
            messages.iter().any(|message| matches!(
                message,
                Message::Terminal(crate::app::message::TerminalMessage::DragStarted)
            )),
            "y={y}"
        );
        assert_eq!(interaction, iced::mouse::Interaction::ResizingVertically, "y={y}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_resize_outer_edges_do_not_start_drag() {
    for y in [-0.5, 8.5] {
        let (messages, interaction, _) = terminal_resize_probe(y).await;
        assert!(
            !messages.iter().any(|message| matches!(
                message,
                Message::Terminal(crate::app::message::TerminalMessage::DragStarted)
            )),
            "y={y}"
        );
        assert_ne!(interaction, iced::mouse::Interaction::ResizingVertically);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_resize_layout_reserves_handle_without_covering_content() {
    let (_, _, children) = terminal_resize_probe(4.0).await;
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].height, 8.0);
    assert_eq!(children[1].y, children[0].y + 8.0);
    assert_eq!(children[1].height, 232.0);
    assert_eq!(children[1].y + children[1].height, 240.0);
}