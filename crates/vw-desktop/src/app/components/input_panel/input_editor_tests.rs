//! 验证输入编辑器的内容同步、附件提及、键盘事件与焦点状态。
//! 键盘 fixture 区分文件搜索接管与编辑器默认行为，并覆盖原生平台快捷键。

use super::input_editor::build_input_editor;
use crate::app::{Message, message};
use iced::keyboard;
fn named_key(named: keyboard::key::Named) -> keyboard::Key {
    keyboard::Key::Named(named)
}

/// Custom composer bindings run even when Iced reports an unfocused editor.
/// Exercise the real sibling widgets so terminal paste/Enter cannot send chat.
#[cfg(not(target_arch = "wasm32"))]
// 宿主保留：终端焦点与宿主输入编辑器事件路由契约。
#[tokio::test(flavor = "multi_thread")]
async fn terminal_focus_composer_does_not_capture_terminal_paste_or_enter() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, Shell, layout, widget::Tree};
    use iced::{Element, Event, Font, Pixels, Point, Rectangle, Size};
    struct Clipboard;
    impl iced::advanced::Clipboard for Clipboard {
        fn read(&self, _: iced::advanced::clipboard::Kind) -> Option<String> {
            Some("中文 paste".into())
        }
        fn write(&mut self, _: iced::advanced::clipboard::Kind, _: String) {}
    }
    let mut app = crate::app::App::new().0;
    app.terminal = crate::app::TerminalState::blank_with_settings(
        true,
        crate::app::Shell::Bash,
        crate::app::TerminalTheme::System,
        "monospace".into(),
        15.0,
        300.0,
    );
    app.terminal
        .add_terminal_with_command("focus".into(), Some(std::env::temp_dir()), "exec cat".into())
        .expect("PTY");
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("renderer");
    let mut element: Element<'_, Message> = iced::widget::column![
        iced::widget::container(build_input_editor(&app, &app.input_editor, false, false).0)
            .height(100),
        iced::widget::container(crate::app::components::terminal_panel::view(&app)).height(300),
    ]
    .into();
    let mut tree = Tree::new(element.as_widget());
    let size = Size::new(800.0, 400.0);
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, size),
    );
    let mut dispatch = |event, point| {
        let mut messages = Vec::new();
        element.as_widget_mut().update(
            &mut tree,
            &event,
            Layout::new(&node),
            iced::mouse::Cursor::Available(point),
            &renderer,
            &mut Clipboard,
            &mut Shell::new(&mut messages),
            &Rectangle::with_size(size),
        );
        messages
    };
    dispatch(
        Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        Point::new(100.0, 200.0),
    );
    let key_event = |key: keyboard::Key, modifiers| {
        Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyV),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    };
    #[cfg(target_os = "macos")]
    let paste_modifiers = keyboard::Modifiers::LOGO;
    #[cfg(not(target_os = "macos"))]
    let paste_modifiers = keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT;
    let mut terminal_commands = |key, modifiers| {
        let point = Point::new(100.0, 200.0);
        dispatch(Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)), point);
        let messages = dispatch(key_event(key, modifiers), point);
        assert!(
            !messages.iter().any(|message| matches!(message, Message::Chat(_))),
            "unfocused composer published a chat action: {messages:?}"
        );
        messages
            .into_iter()
            .filter_map(|message| match message {
                Message::Terminal(message::TerminalMessage::Event(
                    iced_term::Event::BackendCall(_, command),
                )) => Some(command),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let paste = terminal_commands(keyboard::Key::Character("v".into()), paste_modifiers);
    assert!(matches!(
        paste.as_slice(),
        [iced_term::BackendCommand::Paste(text)] if text == "中文 paste"
    ));
    let enter =
        terminal_commands(named_key(keyboard::key::Named::Enter), keyboard::Modifiers::empty());
    assert!(matches!(
        enter.as_slice(),
        [iced_term::BackendCommand::Write(bytes)] if bytes == b"\n"
    ));
    dispatch(
        Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        Point::new(100.0, 30.0),
    );
    let messages = dispatch(
        key_event(named_key(keyboard::key::Named::Enter), keyboard::Modifiers::empty()),
        Point::new(100.0, 30.0),
    );
    assert!(
        messages
            .iter()
            .any(|message| matches!(message, Message::Chat(message::ChatMessage::SendPressed)))
    );
}
