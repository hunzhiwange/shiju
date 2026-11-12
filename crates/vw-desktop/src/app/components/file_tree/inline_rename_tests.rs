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
    app.file_tree_rename_value = "原始视频.mov—用于检查中文与超长名称的原位编辑".into();
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

#[tokio::test]
async fn inline_editor_preserves_tree_row_alignment() {
    use crate::app::components::file_tree::tree_list::{build_changes_list, build_file_tree_list};
    use crate::app::components::file_tree::tree_list_tests::{RowScene, alignment_app};
    let mut renderer = crate::app::components::apps_usage_details_tests::renderer().await;
    for theme in [Theme::Light, Theme::Dark] {
        for changes in [false, true] {
            for (path, name) in [
                ("sp6", "sp6"),
                ("sp6/原始视频.mov", "原始视频.mov"),
                ("sp7/.shiju/配置.json", "配置.json"),
            ] {
                let mut app = alignment_app();
                let build = if changes { build_changes_list } else { build_file_tree_list };
                let original = RowScene::new(build(&app), &mut renderer, &theme);
                let icon = original.icon(name).clone();
                let label = original.label(name);
                drop(original);
                app.file_tree_rename_path = Some(format!("/tree-fixture/{path}"));
                app.file_tree_rename_value = name.into();
                let renamed = RowScene::new(build(&app), &mut renderer, &theme);
                assert_eq!(renamed.inputs.len(), 1);
                let input = renamed.inputs[0];
                let rename_icon = renamed
                    .icons
                    .iter()
                    .find(|(_, bounds)| (bounds.center_y() - input.center_y()).abs() < 1.0)
                    .expect("rename icon");
                assert_eq!(rename_icon.0, icon.0);
                assert_eq!(rename_icon.1.x, icon.1.x);
                assert_eq!(input.x, label.x, "input frame occupies the name slot");
                assert_eq!(input.x, 25.0 + path.matches('/').count() as f32 * 20.0);
            }
        }
    }
}

#[test]
fn inline_input_has_no_border_or_background_in_all_states() {
    for theme in [Theme::Light, Theme::Dark] {
        for status in [
            text_input::Status::Active,
            text_input::Status::Hovered,
            text_input::Status::Focused { is_hovered: false },
            text_input::Status::Disabled,
        ] {
            let style = input_style(&theme, status);
            assert_eq!(style.border.width, 0.0);
            assert_eq!(style.background, Background::Color(Color::TRANSPARENT));
        }
    }
}

#[tokio::test]
async fn inline_editor_keeps_original_tree_height() {
    use crate::app::components::file_tree::tree_list::build_file_tree_list;
    use iced::advanced::{layout, widget::Tree};
    let mut renderer = crate::app::components::apps_usage_details_tests::renderer().await;
    let (mut app, _) = App::new();
    app.project_path = Some("/tree".into());
    app.file_index_cache.insert("/tree".into(), vec!["/tree/中文.md".into()]);
    let limits = layout::Limits::new(Size::ZERO, Size::new(320.0, 500.0));
    let original = {
        let mut element = build_file_tree_list(&app);
        let mut tree = Tree::new(&element);
        element.as_widget_mut().layout(&mut tree, &renderer, &limits).size()
    };
    app.file_tree_rename_path = Some("/tree/中文.md".into());
    app.file_tree_rename_value = "中文.md".into();
    let mut element = build_file_tree_list(&app);
    let mut tree = Tree::new(&element);
    let renamed = element.as_widget_mut().layout(&mut tree, &renderer, &limits).size();
    assert_eq!(original.height, renamed.height);
    for theme in [Theme::Light, Theme::Dark] {
        let mut ui = UserInterface::build(
            build_file_tree_list(&app),
            Size::new(320.0, 500.0),
            Cache::new(),
            &mut renderer,
        );
        ui.draw(
            &mut renderer,
            &theme,
            &renderer::Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
    }
}
