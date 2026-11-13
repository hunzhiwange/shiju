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

#[test]
fn ecp01_popovers_close_on_escape_outside_and_tab_switch() {
    if !isolated("ecp01_popovers_close_on_escape_outside_and_tab_switch") {
        return;
    }
    let mut app = App::new().0;
    app.file_manager_show_changes = false;
    app.screen = crate::app::Screen::Project;
    app.preview_tabs = vec![tab("/tmp/a.rs", None)];
    app.active_preview_path = Some("/tmp/a.rs".into());
    let _ = editor::update(&mut app, EditorMessage::TogglePopover(EditorPopover::Help));
    assert_eq!(app.editor_popover, Some(EditorPopover::Help));
    let _ = editor::update(&mut app, EditorMessage::TogglePopover(EditorPopover::Help));
    assert_eq!(app.editor_popover, None);
    let _ = editor::update(&mut app, EditorMessage::TogglePopover(EditorPopover::Settings));
    let _ = editor::update(&mut app, EditorMessage::ClosePopover);

    assert_eq!(app.editor_popover, None);
    for kind in [EditorPopover::Actions, EditorPopover::Settings, EditorPopover::Help] {
        let _ = editor::update(&mut app, EditorMessage::TogglePopover(kind));
        assert_eq!(app.editor_popover, Some(kind));
    }
    let _ = crate::app::message::preview::tab::update(
        &mut app,
        crate::app::message::PreviewMessage::Select(String::new()),
    );
    assert_eq!(app.editor_popover, None);
    let _ = editor::update(&mut app, EditorMessage::TogglePopover(EditorPopover::Help));
    assert_eq!(app.editor_popover, None);
    app.active_preview_path = Some("/tmp/a.rs".into());
    let _ = editor::update(&mut app, EditorMessage::TogglePopover(EditorPopover::Help));
    let _ = crate::app::message::preview::tab::update(
        &mut app,
        crate::app::message::PreviewMessage::Close("/tmp/a.rs".into()),
    );
    assert_eq!(app.editor_popover, None);
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
async fn ecp01_opening_popover_preserves_editor_bounds() {
    if !isolated("ecp01_opening_popover_preserves_editor_bounds") {
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
    };
    let closed = measure(&app, &mut renderer);
    for kind in [EditorPopover::Actions, EditorPopover::Settings, EditorPopover::Help] {
        let _ = editor::update(&mut app, EditorMessage::TogglePopover(kind));
        assert_eq!(measure(&app, &mut renderer), closed);
        for event in [
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                modified_key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                physical_key: iced::keyboard::key::Physical::Unidentified(
                    iced::keyboard::key::NativeCode::Unidentified,
                ),
                location: iced::keyboard::Location::Standard,
                modifiers: iced::keyboard::Modifiers::default(),
                text: None,
                repeat: false,
            }),
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        ] {
            use iced_runtime::user_interface::{Cache, UserInterface};
            let mut ui = UserInterface::build(
                super::view(&app),
                iced::Size::new(900.0, 600.0),
                Cache::new(),
                &mut renderer,
            );
            let mut messages = Vec::new();
            let _ = ui.update(
                &[event],
                iced::mouse::Cursor::Available(iced::Point::new(450.0, 550.0)),
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
            assert!(
                messages.iter().any(|msg| matches!(
                    msg,
                    crate::app::Message::Editor(EditorMessage::ClosePopover)
                )),
                "{messages:?}"
            );
            assert!(!messages.iter().any(|msg| matches!(
                msg,
                crate::app::Message::Preview(crate::app::message::PreviewMessage::EditorEvent(_))
            )));
        }
        let _ = editor::update(&mut app, EditorMessage::ClosePopover);
    }
}
