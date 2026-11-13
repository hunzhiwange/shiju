use crate::app::components::editor::Editor;
use crate::app::message::editor::EditorOption;
use crate::app::{App, PreviewTab};
use iced::widget::Id;
use iced_code_editor::{CodeEditor, IndentStyle, Message};
use vw_config_types::ui::GatewayClientSystemSettingsConfig;
#[test]
fn ecp04_options_change_only_active_editor() {
    let mut active = CodeEditor::new("hello", "txt");
    let other = CodeEditor::new("world", "txt");
    for option in EditorOption::ALL {
        let previous = option.enabled(&active);
        let untouched = option.enabled(&other);
        option.set(&mut active, !previous);
        assert_eq!(option.enabled(&active), !previous);
        assert_eq!(option.enabled(&other), untouched);
        option.set(&mut active, previous);
        assert_eq!(option.enabled(&active), previous);
    }
    assert_eq!(active.content(), "hello");
}
#[test]
fn ecp04_vim_shortcut_and_menu_share_state() {
    let mut editor = CodeEditor::new("hello", "txt");
    EditorOption::Vim.set(&mut editor, true);
    assert!(format!("{:?}", editor.vim_mode()).contains("Normal"));
    let _ = editor.update(&Message::VimKey('i'));
    assert!(format!("{:?}", editor.vim_mode()).contains("Insert"));
    let _ = editor.update(&Message::ToggleVimMode);
    assert!(!EditorOption::Vim.enabled(&editor));
    assert_eq!(editor.content(), "hello");
}
#[test]
fn ecp04_tab_uses_selected_indent_style() {
    for style in IndentStyle::ALL {
        let mut editor = CodeEditor::new("", "txt");
        editor.set_indent_style(style);
        let _ = editor.update(&Message::Tab);
        let expected = match style {
            IndentStyle::Spaces(n) => " ".repeat(usize::from(n)),
            IndentStyle::Tab => "\t".into(),
        };
        assert_eq!(editor.content(), expected);
    }
}

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

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_ECP02_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let home = tempfile::tempdir().unwrap();
    let config = GatewayClientSystemSettingsConfig {
        host: "[".into(),
        servers: Vec::new(),
        ..Default::default()
    };
    std::fs::write(
        home.path().join("gateway-client-bootstrap.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let test = format!("{}::{test}", module_path!().split_once("::").unwrap().1);
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &test])
        .env(CHILD, "1")
        .env("VIBEWINDOW_CONFIG_DIR", home.path())
        .env("VIBEWINDOW_WORKSPACE", home.path())
        .current_dir(home.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed; 0 failed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

#[test]
fn ecp04_message_changes_only_selected_tab() {
    if !isolated("ecp04_message_changes_only_selected_tab") {
        return;
    }
    use crate::app::message::editor::{EditorMessage, update};
    let mut app = App::new().0;
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
    let _view = super::editor_options::view(&app);
}

#[test]
fn editor_config_app_options_keep_settings_open() {
    if !isolated("editor_config_app_options_keep_settings_open") {
        return;
    }
    use crate::app::message::editor::{EditorMessage, EditorPopover, update};
    let mut app = App::new().0;
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
    if !isolated("editor_config_app_wrap_column_validates_active_instance") {
        return;
    }
    use crate::app::message::editor::{EditorMessage, EditorPopover, update};
    let mut app = App::new().0;
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
    let _ = super::editor_options::view(&app);
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
    if !isolated("editor_config_app_messages_change_real_editing") {
        return;
    }
    use crate::app::message::editor::{EditorMessage, update};
    let mut app = App::new().0;
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
    if !isolated("editor_config_app_renders_chinese_and_runtime_wrap") {
        return;
    }
    use crate::app::message::editor::{EditorMessage, update};
    use iced::advanced::renderer::Headless;
    use iced_runtime::user_interface::{Cache, UserInterface};
    let mut renderer = iced::futures::executor::block_on(iced::Renderer::new(
        iced::Font::DEFAULT,
        14.0.into(),
        Some("tiny-skia"),
    ))
    .expect("software renderer");
    let mut app = App::new().0;
    app.preview_tabs = vec![tab("/tmp/中文.rs", None)];
    app.active_preview_path = Some("/tmp/中文.rs".into());
    let size = iced::Size::new(800.0, 800.0);
    for theme in [iced::Theme::Dark, iced::Theme::Light] {
        for width in ["4", "80"] {
            let _ = update(&mut app, EditorMessage::SetWrapColumn(width.into()));
            let mut ui = UserInterface::build(
                super::editor_options::view(&app),
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
                editor.view().map(|_| crate::app::Message::Editor(EditorMessage::ClosePopover)),
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
