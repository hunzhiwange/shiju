use crate::app::components::editor::Editor;
use crate::app::message::editor::{self, EditorMessage, EditorPopover};
use crate::app::{App, PreviewTab};
use iced::widget::Id;
use vw_config_types::ui::GatewayClientSystemSettingsConfig;
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
    const CHILD: &str = "SHIJU_ECP01_CHILD";
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

#[tokio::test]
async fn ecp07_popover_layout_is_stable_across_sizes() {
    if !isolated("ecp07_popover_layout_is_stable_across_sizes") {
        return;
    }
    use iced::advanced::{layout, renderer::Headless, widget::Tree};
    let mut renderer =
        iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let mut app = App::new().0;
    app.file_manager_show_changes = false;
    app.screen = crate::app::Screen::Project;
    app.preview_tabs = vec![tab("/tmp/a.rs", None)];
    app.active_preview_path = Some("/tmp/a.rs".into());
    fn bounds(node: &layout::Node, out: &mut Vec<iced::Rectangle>) {
        out.push(node.bounds());
        for child in node.children() {
            bounds(child, out);
        }
    }
    let measure = |app: &App, renderer: &mut iced::Renderer, size: iced::Size| {
        let mut view = super::view(app);
        let mut tree = Tree::new(&view);
        let node = view.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(iced::Size::ZERO, size),
        );
        let mut result = Vec::new();
        bounds(&node, &mut result);
        result
    };
    for theme in [iced::Theme::Light, iced::Theme::Dark] {
        app.app_theme = theme.clone();
        app.editor_theme = theme;
        for size in [
            iced::Size::new(640.0, 480.0),
            iced::Size::new(900.0, 600.0),
            iced::Size::new(1440.0, 900.0),
        ] {
            let closed = measure(&app, &mut renderer, size);
            assert!(!closed.is_empty());
            for kind in [EditorPopover::Actions, EditorPopover::Settings, EditorPopover::Help] {
                let content = app.preview_tabs[0].editor.content();
                let cursor = app.preview_tabs[0].editor.cursor_position();
                let _ = editor::update(&mut app, EditorMessage::TogglePopover(kind));
                assert_eq!(measure(&app, &mut renderer, size), closed, "{kind:?} at {size:?}");
                let _ = editor::update(&mut app, EditorMessage::ClosePopover);
                assert_eq!(measure(&app, &mut renderer, size), closed);
                assert_eq!(app.preview_tabs[0].editor.content(), content);
                assert_eq!(app.preview_tabs[0].editor.cursor_position(), cursor);
            }
        }
    }
}

#[test]
fn ecp07_chinese_input_and_undo_preserve_document() {
    use iced_code_editor::{CodeEditor, Message};
    let mut editor = CodeEditor::new("", "txt");
    let _ = editor.update(&Message::Paste("中文输入，编辑器".into()));
    assert_eq!(editor.content(), "中文输入，编辑器");
    let _ = editor.update(&Message::Undo);
    assert_eq!(editor.content(), "");
}
