//! Exercise native mouse selection against the real Markdown layout.

use super::*;
use crate::app::Screen;
use crate::app::components::chat_panel::tool_text_support::selected_chat_text_for_message;
use crate::app::message::ViewMessage;
use iced::advanced::clipboard;
use iced::advanced::renderer::Headless;
use iced::futures::StreamExt;

fn renderer() -> Renderer {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    iced::futures::executor::block_on(Renderer::new(Font::DEFAULT, 15.0.into(), Some("tiny-skia")))
        .expect("headless renderer")
}

fn update(
    view: &mut Element<'_, Message>,
    tree: &mut Tree,
    node: &layout::Node,
    renderer: &Renderer,
    viewport: Rectangle,
    events: &[(Point, mouse::Event)],
) -> Vec<Message> {
    let mut messages = Vec::new();
    for &(point, event) in events {
        view.as_widget_mut().update(
            tree,
            &Event::Mouse(event),
            Layout::new(node),
            mouse::Cursor::Available(point),
            renderer,
            &mut clipboard::Null,
            &mut Shell::new(&mut messages),
            &viewport,
        );
    }
    messages
}

#[test]
fn markdown_drag_selects_wrapped_text_table_cells_and_code_in_both_directions() {
    let raw = concat!(
        "[文档](https://example.com) 和 **粗体**，",
        "这段中文会在窄窗口中换行，继续选择下面的内容。\n\n",
        "| 名称 | 值 |\n| --- | --- |\n| 数据 | 42 |\n\n",
        "```sh\npnpm install\n```",
    );
    let blocks = super::super::markdown::parse_markdown_blocks(raw);
    let expected = concat!(
        "文档 和 粗体，这段中文会在窄窗口中换行，继续选择下面的内容。\n\n",
        "名称\t值\n数据\t42\n\npnpm install",
    );
    let mut app = App::new().0;
    app.active_session_id = Some("markdown-selection".to_owned());
    app.screen = Screen::Project;
    let mut renderer = renderer();

    for reverse in [false, true] {
        let messages = {
            let mut view = super::super::markdown::markdown_body_view(&app, 0, 0, &blocks);
            let mut tree = Tree::new(view.as_widget());
            let size = Size::new(235.0, 500.0);
            let viewport = Rectangle::with_size(size);
            let node = view.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, size),
            );
            let mut fragments = Fragments::new(viewport);
            view.as_widget_mut().operate(&mut tree, Layout::new(&node), &renderer, &mut fragments);
            let first = &fragments.items.first().unwrap().bounds;
            let last = &fragments.items.last().unwrap().bounds;
            assert!(first.height > 24.0);
            let start = Point::new(first.x + 1.0, first.y + 10.0);
            let end = Point::new(last.x + last.width - 1.0, last.y + 10.0);
            let (from, to) = if reverse { (end, start) } else { (start, end) };
            let events = [
                (from, mouse::Event::ButtonPressed(mouse::Button::Left)),
                (to, mouse::Event::CursorMoved { position: to }),
                (to, mouse::Event::ButtonReleased(mouse::Button::Left)),
            ];
            let messages = update(&mut view, &mut tree, &node, &renderer, viewport, &events);

            let theme = app.theme();
            view.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &viewport,
            );
            assert!(
                matches!(
                    messages.as_slice(),
                    [Message::ChatMarkdownSelectionChanged { text: Some(text), .. }] if text == expected
                ),
                "{messages:?}"
            );
            messages
        };
        for message in messages {
            let _ = app.update(message);
        }
        assert_eq!(selected_chat_text_for_message(&app, 0).as_deref(), Some(expected));
        let task = app.update(Message::ClipboardShortcutRouted {
            kind: crate::app::ui_types::ClipboardShortcutKind::Copy,
            input_focused: false,
        });
        let mut stream = iced_runtime::task::into_stream(task).expect("copy task");
        assert!(matches!(
            iced::futures::executor::block_on(stream.next()),
            Some(iced_runtime::Action::Output(Message::CopyCode(text))) if text == expected
        ));
    }
}

#[test]
fn markdown_link_click_without_drag_still_opens_the_target() {
    let blocks = super::super::markdown::parse_markdown_blocks("[文档](https://example.com/docs)");
    let app = App::new().0;
    let renderer = renderer();
    let mut view = super::super::markdown::markdown_body_view(&app, 0, 0, &blocks);
    let mut tree = Tree::new(view.as_widget());
    let size = Size::new(235.0, 60.0);
    let node =
        view.as_widget_mut().layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, size));
    let point = Point::new(8.0, 12.0);
    let messages = update(
        &mut view,
        &mut tree,
        &node,
        &renderer,
        Rectangle::with_size(size),
        &[
            (point, mouse::Event::ButtonPressed(mouse::Button::Left)),
            (point, mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    );
    assert!(matches!(
        messages.as_slice(),
        [Message::View(ViewMessage::OpenUrlExternal(url))] if url == "https://example.com/docs"
    ));
}
