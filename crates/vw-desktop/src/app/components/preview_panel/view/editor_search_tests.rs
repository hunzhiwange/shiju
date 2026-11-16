// 保留宿主契约：搜索弹窗与完整宿主组合视图的布局边界。
use crate::app::components::editor::Editor;
use crate::app::message::editor::{self, EditorMessage};
use crate::app::{App, PreviewTab};
use iced::widget::Id;
use config_ui::GatewayClientSystemSettingsConfig;
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
async fn ecp06_search_overlay_preserves_editor_bounds() {
    if !isolated("ecp06_search_overlay_preserves_editor_bounds") {
        return;
    }
    use iced::advanced::{layout, renderer::Headless, widget::Tree};
    let mut renderer =
        iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let mut app = App::new().0;
    app.file_manager.file_manager_show_changes = false;
    app.screen = crate::app::Screen::Project;
    app.preview_tabs = vec![tab("/tmp/a.rs", None)];
    app.active_preview_path = Some("/tmp/a.rs".into());
    fn bounds(node: &layout::Node, out: &mut Vec<iced::Rectangle>) {
        out.push(node.bounds());
        for child in node.children() {
            bounds(child, out);
        }
    }
    let measure = |app: &App, renderer: &mut iced::Renderer| {
        let mut view = super::view(app);
        let mut tree = Tree::new(&view);
        let node = view.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(iced::Size::ZERO, iced::Size::new(900.0, 600.0)),
        );
        let mut result = Vec::new();
        bounds(&node, &mut result);
        result
            .into_iter()
            .filter(|rect| rect.width > 800.0 && rect.height > 400.0)
            .collect::<Vec<_>>()
    };
    let closed = measure(&app, &mut renderer);

    for message in
        [EditorMessage::OpenSearch, EditorMessage::OpenReplace, EditorMessage::CloseSearch]
    {
        let content = app.preview_tabs[0].editor.content();
        let cursor = app.preview_tabs[0].editor.cursor_position();
        let _ = editor::update(&mut app, message);
        assert_eq!(measure(&app, &mut renderer), closed);
        assert_eq!(app.preview_tabs[0].editor.content(), content);
        assert_eq!(app.preview_tabs[0].editor.cursor_position(), cursor);
    }
}
