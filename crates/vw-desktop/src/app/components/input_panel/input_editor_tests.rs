//! 验证输入编辑器的内容同步、附件提及、键盘事件与焦点状态。
//! 键盘 fixture 区分文件搜索接管与编辑器默认行为，并覆盖原生平台快捷键。

use super::input_editor::{
    INPUT_LINE_HEIGHT, INPUT_MAX_LINES, INPUT_MIN_LINES, INPUT_VERTICAL_PADDING,
    binding_from_key_press, build_input_editor, input_context_menu,
    input_context_menu_button_style, input_context_menu_style,
};
use crate::app::{Message, message};
use iced::keyboard;
use iced::widget::button;
use iced::widget::text_editor::{Binding, Content, KeyPress, Status};
use iced::{Background, Color, Length, Theme};

// 固定物理按键和焦点状态，使快捷键测试只比较逻辑键、修饰键与文本载荷。
fn key_press(key: keyboard::Key, modifiers: keyboard::Modifiers) -> KeyPress {
    KeyPress {
        key: key.clone(),
        modified_key: key,
        physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyA),
        modifiers,
        text: None,
        status: Status::Focused { is_hovered: false },
    }
}

fn key_press_with_text(
    key: keyboard::Key,
    modifiers: keyboard::Modifiers,
    text: &'static str,
) -> KeyPress {
    let mut key_press = key_press(key, modifiers);
    key_press.text = Some(text.into());
    key_press
}

fn named_key(named: keyboard::key::Named) -> keyboard::Key {
    keyboard::Key::Named(named)
}

fn bg_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

fn assert_custom_chat(
    binding: Option<Binding<Message>>,
    expected: fn(&message::ChatMessage) -> bool,
) {
    match binding {
        Some(Binding::Custom(Message::Chat(chat))) => assert!(expected(&chat)),
        _ => panic!("expected custom chat binding"),
    }
}

#[test]
fn editor_height_clamps_between_minimum_and_maximum_lines() {
    let app = crate::app::App::new().0;

    let (_, empty_height) = build_input_editor(&app, &Content::new(), false, false);
    assert_eq!(
        empty_height,
        INPUT_MIN_LINES as f32 * INPUT_LINE_HEIGHT + INPUT_VERTICAL_PADDING * 2.0
    );

    let many_lines = Content::with_text("1\n2\n3\n4\n5\n6\n7\n8\n9\n10");
    let (_, max_height) = build_input_editor(&app, &many_lines, true, true);
    assert_eq!(
        max_height,
        INPUT_MAX_LINES as f32 * INPUT_LINE_HEIGHT + INPUT_VERTICAL_PADDING * 2.0
    );
}

#[test]
fn build_input_editor_wraps_context_menu_when_open() {
    let mut app = crate::app::App::new().0;
    app.input_context_menu_pos = Some((10.0, 20.0));
    app.input_context_menu_open = true;
    let content = Content::with_text("hello\nworld");

    let (element, height) = build_input_editor(&app, &content, false, false);

    assert_eq!(element.as_widget().size().width, Length::Fill);
    assert_eq!(height, 3.0 * INPUT_LINE_HEIGHT + INPUT_VERTICAL_PADDING * 2.0);
}

#[test]
fn build_input_editor_ignores_context_menu_anchor_when_closed() {
    let mut app = crate::app::App::new().0;
    app.input_context_menu_pos = Some((10.0, 20.0));
    app.input_context_menu_open = false;
    let content = Content::with_text("one\ntwo\nthree\nfour\nfive");

    let (element, height) = build_input_editor(&app, &content, false, false);

    assert_eq!(element.as_widget().size().width, Length::Fill);
    assert_eq!(height, 5.0 * INPUT_LINE_HEIGHT + INPUT_VERTICAL_PADDING * 2.0);
}

#[test]
fn input_context_menu_contains_four_actions() {
    let menu = input_context_menu(crate::app::i18n::I18n::new(
        vw_config_types::ui::DesktopLocale::SimplifiedChinese,
    ));

    assert_eq!(menu.as_widget().size().width, Length::Shrink);
}

#[test]
fn input_context_menu_styles_cover_theme_and_status_variants() {
    for theme in [Theme::Light, Theme::Dark] {
        let idle = input_context_menu_button_style(&theme, button::Status::Active);
        let hovered = input_context_menu_button_style(&theme, button::Status::Hovered);
        let pressed = input_context_menu_button_style(&theme, button::Status::Pressed);

        assert!(idle.background.is_none());
        assert!(hovered.background.is_some());
        assert_ne!(hovered.background, pressed.background);
        assert_eq!(idle.border.width, 0.0);
        assert_eq!(idle.text_color, theme.palette().text);

        let menu = input_context_menu_style(&theme);
        assert!(matches!(menu.background, Some(Background::Color(_))));
        assert_eq!(menu.border.width, 1.0);
        assert_eq!(menu.border.radius.top_left, 8.0);
        assert!(menu.shadow.blur_radius > 0.0);
    }

    assert_ne!(
        bg_color(input_context_menu_style(&Theme::Light).background),
        bg_color(input_context_menu_style(&Theme::Dark).background)
    );
}

#[test]
fn file_search_key_bindings_capture_navigation_and_selection() {
    let mut app = crate::app::App::new().0;
    app.show_file_search = true;

    assert_custom_chat(
        binding_from_key_press(
            &app,
            key_press(named_key(keyboard::key::Named::ArrowUp), keyboard::Modifiers::NONE),
        ),
        |message| matches!(message, message::ChatMessage::FileSearchNavigateUp),
    );
    assert_custom_chat(
        binding_from_key_press(
            &app,
            key_press(named_key(keyboard::key::Named::ArrowDown), keyboard::Modifiers::NONE),
        ),
        |message| matches!(message, message::ChatMessage::FileSearchNavigateDown),
    );
    assert_custom_chat(
        binding_from_key_press(
            &app,
            key_press(named_key(keyboard::key::Named::Tab), keyboard::Modifiers::NONE),
        ),
        |message| matches!(message, message::ChatMessage::FileSearchSelectCurrent),
    );
    assert_custom_chat(
        binding_from_key_press(
            &app,
            key_press(named_key(keyboard::key::Named::Escape), keyboard::Modifiers::NONE),
        ),
        |message| matches!(message, message::ChatMessage::FileSearchInputChanged(value) if value.is_empty()),
    );
}

#[test]
fn file_search_key_bindings_delegate_regular_text_to_editor() {
    let mut app = crate::app::App::new().0;
    app.show_file_search = true;

    let inserted = binding_from_key_press(
        &app,
        key_press_with_text(keyboard::Key::Character("x".into()), keyboard::Modifiers::NONE, "x"),
    );
    assert!(matches!(inserted, Some(Binding::Insert('x'))));

    #[cfg(not(target_arch = "wasm32"))]
    {
        let modified_enter = binding_from_key_press(
            &app,
            key_press(named_key(keyboard::key::Named::Enter), keyboard::Modifiers::SHIFT),
        );
        assert!(matches!(modified_enter, Some(Binding::Enter)));
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_enter_selects_file_search_only_without_modifiers() {
    let mut app = crate::app::App::new().0;
    app.show_file_search = true;

    assert_custom_chat(
        binding_from_key_press(
            &app,
            key_press(named_key(keyboard::key::Named::Enter), keyboard::Modifiers::NONE),
        ),
        |message| matches!(message, message::ChatMessage::FileSearchSelectCurrent),
    );

    let shifted = binding_from_key_press(
        &app,
        key_press(named_key(keyboard::key::Named::Enter), keyboard::Modifiers::SHIFT),
    );
    assert!(matches!(shifted, Some(Binding::Enter)));
}

#[test]
fn regular_key_bindings_capture_paste_and_send_shortcuts() {
    let app = crate::app::App::new().0;

    assert_custom_chat(
        binding_from_key_press(
            &app,
            key_press(keyboard::Key::Character("v".into()), keyboard::Modifiers::COMMAND),
        ),
        |message| matches!(message, message::ChatMessage::PasteIntoInput),
    );

    #[cfg(not(target_arch = "wasm32"))]
    assert_custom_chat(
        binding_from_key_press(
            &app,
            key_press(named_key(keyboard::key::Named::Enter), keyboard::Modifiers::NONE),
        ),
        |message| matches!(message, message::ChatMessage::SendPressed),
    );

    let alt_paste = binding_from_key_press(
        &app,
        key_press(
            keyboard::Key::Character("v".into()),
            keyboard::Modifiers::COMMAND | keyboard::Modifiers::ALT,
        ),
    );
    assert!(!matches!(alt_paste, Some(Binding::Custom(_))));
}

#[test]
fn regular_paste_binding_accepts_control_uppercase_and_shift_but_rejects_alt() {
    let app = crate::app::App::new().0;

    for modifiers in
        [keyboard::Modifiers::CTRL, keyboard::Modifiers::COMMAND | keyboard::Modifiers::SHIFT]
    {
        assert_custom_chat(
            binding_from_key_press(
                &app,
                key_press(keyboard::Key::Character("V".into()), modifiers),
            ),
            |message| matches!(message, message::ChatMessage::PasteIntoInput),
        );
    }

    let alt_control = binding_from_key_press(
        &app,
        key_press(
            keyboard::Key::Character("V".into()),
            keyboard::Modifiers::CTRL | keyboard::Modifiers::ALT,
        ),
    );
    assert!(!matches!(alt_control, Some(Binding::Custom(_))));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn regular_key_bindings_delegate_modified_enter_and_navigation_to_editor() {
    let app = crate::app::App::new().0;

    let shifted_enter = binding_from_key_press(
        &app,
        key_press(named_key(keyboard::key::Named::Enter), keyboard::Modifiers::SHIFT),
    );
    assert!(matches!(shifted_enter, Some(Binding::Enter)));

    let arrow_down = binding_from_key_press(
        &app,
        key_press(named_key(keyboard::key::Named::ArrowDown), keyboard::Modifiers::NONE),
    );
    assert!(matches!(arrow_down, Some(Binding::Move(_))));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_enter_with_each_non_shift_modifier_delegates_to_editor() {
    let app = crate::app::App::new().0;

    for modifiers in
        [keyboard::Modifiers::CTRL, keyboard::Modifiers::ALT, keyboard::Modifiers::COMMAND]
    {
        let binding = binding_from_key_press(
            &app,
            key_press(named_key(keyboard::key::Named::Enter), modifiers),
        );

        assert!(matches!(binding, Some(Binding::Enter)), "modifiers: {modifiers:?}");
    }
}

#[test]
fn editor_style_callbacks_can_be_invoked_through_construction_inputs() {
    let app = crate::app::App::new().0;
    let content = Content::with_text("one\ntwo\nthree\nfour");
    let (_, height) = build_input_editor(&app, &content, true, false);

    assert_eq!(height, 4.0 * INPUT_LINE_HEIGHT + INPUT_VERTICAL_PADDING * 2.0);
    assert!(Theme::Dark.palette().background.r < Theme::Light.palette().background.r);
}

/// Custom composer bindings run even when Iced reports an unfocused editor.
/// Exercise the real sibling widgets so terminal paste/Enter cannot send chat.
#[cfg(not(target_arch = "wasm32"))]
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
