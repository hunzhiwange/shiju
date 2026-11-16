use crate::components::editor::Editor;
use crate::components::preview_panel::view::{
    ContentMessage,
    content::{ContentTab, build_content_base},
};
use iced::advanced::widget::Tree;
struct Tab {
    path: String,
    editor: Editor,
}
#[derive(Default)]
struct Fixture {
    active_preview_path: Option<String>,
    preview_tabs: Vec<Tab>,
}
fn app() -> Fixture {
    Fixture::default()
}
fn tab(path: &str, content: &str) -> Tab {
    Tab { path: path.into(), editor: Editor::new(content, "txt") }
}
fn content_tree(app: &Fixture) -> Tree {
    let tab = app
        .active_preview_path
        .as_ref()
        .and_then(|path| app.preview_tabs.iter().find(|tab| &tab.path == path))
        .map(|tab| ContentTab {
            path: &tab.path,
            editor: &tab.editor,
            scroll_id: iced::widget::Id::unique(),
            truncated: false,
            message: None,
        });
    let empty: iced::Element<'_, ContentMessage<()>> = iced::widget::text("empty").into();
    let element = build_content_base(
        tab,
        app.active_preview_path.is_some(),
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        empty,
        #[cfg(not(target_arch = "wasm32"))]
        iced::widget::text("").into(),
    );
    Tree::new(element.as_widget())
}
#[test]
fn content_shows_missing_preview_when_active_tab_is_absent() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/missing.rs".to_string());

    let tree = content_tree(&app);

    assert!(tree.children.is_empty());
}

#[test]
fn content_builds_editor_view_for_text_tab() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/main.rs".to_string());
    app.preview_tabs = vec![tab("/tmp/main.rs", "fn main() {}")];

    let tree = content_tree(&app);

    assert_eq!(tree.children.len(), 1);
}

#[test]
fn content_builds_raster_image_view_from_extension() {
    let mut editor_app = app();
    editor_app.active_preview_path = Some("/tmp/image.txt".to_string());
    editor_app.preview_tabs = vec![tab("/tmp/image.txt", "")];
    let editor_tag = content_tree(&editor_app).tag;

    for extension in ["png", "JPG", "jpeg", "GIF", "bmp", "WebP"] {
        let path = format!("/tmp/image.{extension}");
        let mut app = app();
        app.active_preview_path = Some(path.clone());
        app.preview_tabs = vec![tab(&path, "")];

        let tree = content_tree(&app);

        assert_ne!(tree.tag, editor_tag, "{extension} should use a media view");
        assert_eq!(tree.children.len(), 1);
    }
}

#[test]
fn content_builds_svg_view_from_extension() {
    let mut editor_app = app();
    editor_app.active_preview_path = Some("/tmp/icon.txt".to_string());
    editor_app.preview_tabs = vec![tab("/tmp/icon.txt", "")];
    let editor_tag = content_tree(&editor_app).tag;

    for extension in ["svg", "SVG"] {
        let path = format!("/tmp/icon.{extension}");
        let mut app = app();
        app.active_preview_path = Some(path.clone());
        app.preview_tabs = vec![tab(&path, "")];

        let tree = content_tree(&app);

        assert_ne!(tree.tag, editor_tag, "{extension} should use a media view");
        assert_eq!(tree.children.len(), 1);
    }
}

#[test]
fn content_uses_editor_view_for_unknown_or_missing_extension() {
    let mut reference = app();
    reference.active_preview_path = Some("/tmp/main.rs".to_string());
    reference.preview_tabs = vec![tab("/tmp/main.rs", "fn main() {}")];
    let editor_tag = content_tree(&reference).tag;

    for path in [
        "/tmp/README",
        "/tmp.with.dot/README",
        "/tmp/.svg",
        "/tmp/archive.unknown",
        "/tmp/trailing.",
    ] {
        let mut app = app();
        app.active_preview_path = Some(path.to_string());
        app.preview_tabs = vec![tab(path, "plain text")];

        assert_eq!(content_tree(&app).tag, editor_tag, "{path} should use the editor view");
    }
}
