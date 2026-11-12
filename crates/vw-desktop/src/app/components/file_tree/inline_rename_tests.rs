use super::*;
use iced::advanced::{clipboard, renderer, widget::operation};
use iced::{Event, Size, Theme, keyboard, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

#[test]
fn selection_preserves_extensions_and_counts_graphemes() {
    for (name, directory, end) in [
        ("ACP验收.md", false, 5),
        ("archive.tar.gz", false, 11),
        (".gitignore", false, 10),
        ("dev.docs", true, 8),
        ("e\u{301}📚.md", false, 2),
    ] {
        assert_eq!(selection_end(name, directory), end, "{name}");
    }
}

#[tokio::test]
async fn inline_editor_renders_chinese_and_submits_without_row_actions() {
    let mut renderer = crate::app::components::apps_usage_details_tests::renderer().await;
    let (mut app, _) = App::new();
    app.file_tree_rename_value = "ACP验收.md".into();
    for theme in [Theme::Light, Theme::Dark] {
        for directory in [false, true] {
            let mut ui = UserInterface::build(
                view(&app, 2, directory, true),
                Size::new(320.0, 120.0),
                Cache::new(),
                &mut renderer,
            );
            ui.operate(&mut renderer, &mut operation::focusable::focus(INPUT_ID.into()));
            ui.operate(
                &mut renderer,
                &mut operation::text_input::select_range(INPUT_ID.into(), 0, 5),
            );
            ui.draw(
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
            let event = Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Enter),
                modified_key: keyboard::Key::Named(keyboard::key::Named::Enter),
                physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Enter),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::NONE,
                text: None,
                repeat: false,
            });
            let mut messages = Vec::new();
            let _ = ui.update(
                &[event],
                mouse::Cursor::Unavailable,
                &mut renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            assert!(matches!(
                messages.as_slice(),
                [Message::Project(ProjectMessage::FileTreeRenameSave)]
            ));
        }
    }
}
