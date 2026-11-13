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
