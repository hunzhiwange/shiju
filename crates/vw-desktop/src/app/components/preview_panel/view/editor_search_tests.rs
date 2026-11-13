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

#[test]
fn ecp06_search_closes_surrounding_panels() {
    if !isolated("ecp06_search_closes_surrounding_panels") {
        return;
    }
    let mut app = App::new().0;
    app.preview_tabs = vec![tab("/tmp/a.rs", None)];
    app.active_preview_path = Some("/tmp/a.rs".into());
    app.show_preview_settings = true;
    app.editor_popover = Some(EditorPopover::Help);
    let _ = editor::update(&mut app, EditorMessage::OpenSearch);
    assert!(!app.show_preview_settings);
    assert_eq!(app.editor_popover, None);
}

fn send(app: &mut App, event: iced_code_editor::Message) {
    let _ = crate::app::message::preview::update(
        app,
        crate::app::message::PreviewMessage::EditorEvent(event),
    );
}

#[test]
fn ecp06_search_replace_updates_only_active_document() {
    if !isolated("ecp06_search_replace_updates_only_active_document") {
        return;
    }
    use iced_code_editor::Message as E;
    let mut app = App::new().0;
    app.preview_auto_save_mode = crate::app::PreviewAutoSaveMode::Off;
    app.preview_tabs = vec![tab("/tmp/a.rs", None), tab("/tmp/b.rs", None)];
    app.active_preview_path = Some("/tmp/a.rs".into());
    let _ = editor::update(&mut app, EditorMessage::OpenReplace);
    send(&mut app, E::SearchQueryChanged("main".into()));
    send(&mut app, E::ReplaceQueryChanged("start".into()));
    send(&mut app, E::ReplaceNext);
    assert_eq!(app.preview_tabs[0].editor.content(), "fn start() {}");
    assert_eq!(app.preview_tabs[1].editor.content(), "fn main() {}");
    assert!(app.preview_tabs[0].is_dirty);
    assert!(app.preview_tabs[0].auto_save_revision > 0);
    send(&mut app, E::Undo);
    assert_eq!(app.preview_tabs[0].editor.content(), "fn main() {}");
    send(&mut app, E::SearchQueryChanged("FN".into()));
    send(&mut app, E::ToggleCaseSensitive);
    send(&mut app, E::ReplaceAll);
    assert_eq!(app.preview_tabs[0].editor.content(), "fn main() {}");
    send(&mut app, E::ToggleCaseSensitive);
    send(&mut app, E::FindNext);
    send(&mut app, E::FindPrevious);
    send(&mut app, E::SearchDialogTab);
    send(&mut app, E::SearchDialogShiftTab);
    send(&mut app, E::ReplaceAll);
    assert_eq!(app.preview_tabs[0].editor.content(), "start main() {}");
    send(&mut app, E::Undo);
    assert_eq!(app.preview_tabs[0].editor.content(), "fn main() {}");
    let _ = editor::update(&mut app, EditorMessage::CloseSearch);
}

#[test]
fn ecp06_read_only_blocks_replace() {
    if !isolated("ecp06_read_only_blocks_replace") {
        return;
    }
    use iced_code_editor::Message as E;
    let mut app = App::new().0;
    app.preview_tabs = vec![tab("/tmp/a.rs", None)];
    app.preview_tabs[0].read_only = true;
    app.active_preview_path = Some("/tmp/a.rs".into());
    let _ = editor::update(&mut app, EditorMessage::OpenReplace);
    send(&mut app, E::OpenSearchReplace);
    send(&mut app, E::SearchQueryChanged("main".into()));
    send(&mut app, E::ReplaceQueryChanged("changed".into()));
    send(&mut app, E::ReplaceNext);
    send(&mut app, E::ReplaceAll);
    assert_eq!(app.preview_tabs[0].editor.content(), "fn main() {}");
    assert!(!app.preview_tabs[0].is_dirty);
    assert_eq!(app.preview_tabs[0].auto_save_revision, 0);
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
