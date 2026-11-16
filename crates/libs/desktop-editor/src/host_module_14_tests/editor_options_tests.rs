use super::fixture::update;
// 验证领域选项消息、活动实例选择与中文真实绘制路径。
use super::fixture::Fixture;
use crate::components::editor::Editor;
use crate::message::editor::EditorOption;
use crate::preview::PreviewTab;
use iced::widget::Id;
use iced_code_editor::{IndentStyle, Message};

fn tab(path: &str, server_key: Option<&'static str>) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: path.rsplit('/').next().unwrap_or(path).to_string(),
        content: "fn main() {}".to_string(),
        is_dirty: false,
        truncated: false,
        read_only: false,
        auto_save_revision: 0,
        editor: Editor::new("fn main() {}", "rust"),
        scroll_id: Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: server_key,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: Some(format!("file://{path}")),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: Some("rust".to_string()),
    }
}

#[test]
fn ecp04_message_changes_only_selected_tab() {
    use crate::message::editor::EditorMessage;
    let mut app = Fixture::default();
    app.preview_tabs = vec![tab("/tmp/a.rs", None), tab("/tmp/b.rs", None)];
    app.active_preview_path = Some("/tmp/a.rs".into());
    let previous = app.preview_tabs[1].editor.inner.wrap_enabled();
    let _ = update(&mut app, EditorMessage::SetOption(EditorOption::Wrap, !previous));
    assert_eq!(app.preview_tabs[0].editor.inner.wrap_enabled(), !previous);
    assert_eq!(app.preview_tabs[1].editor.inner.wrap_enabled(), previous);
    app.active_preview_path = Some("/tmp/b.rs".into());
    let _ = update(&mut app, EditorMessage::SetIndentStyle(IndentStyle::Tab));
    assert_eq!(app.preview_tabs[1].editor.inner.indent_style(), IndentStyle::Tab);
    assert!(!app.preview_tabs.iter().any(|tab| tab.is_dirty));
    let _view = super::fixture::options(&app);
}

#[test]
fn editor_config_app_options_keep_settings_open() {
    use crate::message::editor::{EditorMessage, EditorPopover};
    let mut app = Fixture::default();
    app.preview_tabs = vec![tab("/tmp/a.rs", None)];
    app.active_preview_path = Some("/tmp/a.rs".into());
    app.editor_popover = Some(EditorPopover::Settings);
    for option in EditorOption::ALL {
        let previous = option.enabled(&app.preview_tabs[0].editor.inner);
        let _ = update(&mut app, EditorMessage::SetOption(option, !previous));
        assert_eq!(option.enabled(&app.preview_tabs[0].editor.inner), !previous);
        assert_eq!(app.editor_popover, Some(EditorPopover::Settings));
    }
    assert!(!app.preview_tabs[0].is_dirty);
}

#[test]
fn editor_config_app_wrap_column_validates_active_instance() {
    use crate::message::editor::{EditorMessage, EditorPopover};
    let mut app = Fixture::default();
    app.preview_tabs = vec![tab("/tmp/a.rs", None), tab("/tmp/b.rs", None)];
    app.preview_tabs[0].read_only = true;
    app.active_preview_path = Some("/tmp/a.rs".into());
    for value in ["4", "80", "1"] {
        app.editor_popover = Some(EditorPopover::Settings);
        let _ = update(&mut app, EditorMessage::SetWrapColumn(value.into()));
        assert_eq!(app.preview_tabs[0].editor.inner.wrap_column(), Some(value.parse().unwrap()));
        assert_eq!(app.preview_tabs[1].editor.inner.wrap_column(), None);
        assert_eq!(app.editor_popover, Some(EditorPopover::Settings));
    }
    for value in ["0", "-1", "abc", "1.5", "184467440737095516160"] {
        let _ = update(&mut app, EditorMessage::SetWrapColumn(value.into()));
        assert_eq!(app.preview_tabs[0].editor.inner.wrap_column(), Some(1));
    }
    let _ = update(&mut app, EditorMessage::SetOption(EditorOption::Wrap, false));
    let _ = update(&mut app, EditorMessage::SetWrapColumn("20".into()));
    assert_eq!(app.preview_tabs[0].editor.inner.wrap_column(), Some(1));
    let _ = update(&mut app, EditorMessage::SetOption(EditorOption::Wrap, true));
    let _ = update(&mut app, EditorMessage::SetWrapColumn(String::new()));
    assert_eq!(app.preview_tabs[0].editor.inner.wrap_column(), None);
    app.active_preview_path = Some("/tmp/b.rs".into());
    let _ = update(&mut app, EditorMessage::SetWrapColumn("8".into()));
    let _ = update(&mut app, EditorMessage::ClosePopover);
    let _ = update(&mut app, EditorMessage::TogglePopover(EditorPopover::Settings));
    assert_eq!(app.preview_tabs[1].editor.inner.wrap_column(), Some(8));
    assert_eq!(app.editor_popover, Some(EditorPopover::Settings));
    app.active_preview_path = None;
    let _ = super::fixture::options(&app);
    let _ = update(&mut app, EditorMessage::SetWrapColumn("9".into()));
    assert_eq!(app.preview_tabs[1].editor.inner.wrap_column(), Some(8));
    assert!(
        app.preview_tabs
            .iter()
            .all(|tab| !tab.is_dirty && tab.editor.inner.content() == tab.content)
    );
}

#[test]
fn editor_config_app_messages_change_real_editing() {
    use crate::message::editor::EditorMessage;
    let mut app = Fixture::default();
    app.preview_tabs = vec![tab("/tmp/a.rs", None), tab("/tmp/b.rs", None)];
    app.active_preview_path = Some("/tmp/a.rs".into());
    let _ = update(&mut app, EditorMessage::SetIndentStyle(IndentStyle::Spaces(2)));
    let _ = app.preview_tabs[0].editor.inner.update(&Message::Tab);
    assert!(app.preview_tabs[0].editor.inner.content().starts_with("  fn"));
    assert_eq!(app.preview_tabs[1].editor.inner.content(), "fn main() {}");
    let _ = app.preview_tabs[0].editor.inner.update(&Message::Undo);
    assert_eq!(app.preview_tabs[0].editor.inner.content(), "fn main() {}");
}

#[test]
fn editor_config_app_renders_chinese_and_runtime_wrap() {
    use crate::message::editor::EditorMessage;
    use iced::advanced::renderer::Headless;
    use iced_runtime::user_interface::{Cache, UserInterface};
    let mut renderer = iced::futures::executor::block_on(iced::Renderer::new(
        iced::Font::DEFAULT,
        14.0.into(),
        Some("tiny-skia"),
    ))
    .expect("software renderer");
    let mut app = Fixture::default();
    app.preview_tabs = vec![tab("/tmp/中文.rs", None)];
    app.active_preview_path = Some("/tmp/中文.rs".into());
    let size = iced::Size::new(800.0, 800.0);
    for theme in [iced::Theme::Dark, iced::Theme::Light] {
        for width in ["4", "80"] {
            let _ = update(&mut app, EditorMessage::SetWrapColumn(width.into()));
            let mut ui = UserInterface::build(
                super::fixture::options(&app),
                size,
                Cache::new(),
                &mut renderer,
            );
            ui.draw(
                &mut renderer,
                &theme,
                &iced::advanced::renderer::Style { text_color: theme.palette().text },
                iced::mouse::Cursor::Unavailable,
            );
            assert!(
                !renderer
                    .screenshot(iced::Size::new(800, 800), 1.0, theme.palette().background)
                    .is_empty()
            );
            drop(ui);
            let editor = &mut app.preview_tabs[0].editor.inner;
            let _ = editor.set_cursor(0, 12);
            let mut ui = UserInterface::build(
                editor
                    .view()
                    .map(|_| crate::components::preview_panel::view::Message::ClosePopover),
                size,
                Cache::new(),
                &mut renderer,
            );
            ui.draw(
                &mut renderer,
                &theme,
                &iced::advanced::renderer::Style { text_color: theme.palette().text },
                iced::mouse::Cursor::Unavailable,
            );
            drop(ui);
            let y = editor.cursor_screen_position().expect("visible cursor").y;
            if width == "4" {
                assert!(y > editor.line_height());
            } else {
                assert!(y <= editor.line_height());
            }
        }
    }
}
