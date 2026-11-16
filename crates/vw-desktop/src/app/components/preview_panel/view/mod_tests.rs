// 保留宿主装配：Screen/WorkspaceMode、Git 面板、App 设置弹窗、菜单和 LSP progress 投影组合为真实预览入口。
//! 验证预览面板视图的内容选择、菜单状态与覆盖层组合。
//! 通过空标签、活动文件和多覆盖层状态检查入口的组合优先级。

use super::{show_fullscreen_controls, view};
use crate::app::components::editor::Editor;
use crate::app::state::WorkspaceMode;
use crate::app::{App, Message, PreviewTab, Screen};
use iced::widget::Id;

fn app() -> App {
    App::new().0
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

fn keep(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn view_builds_project_preview_with_tabs_and_content() {
    let mut app = app();
    app.screen = Screen::Project;
    app.active_preview_path = Some("/tmp/main.rs".to_string());
    app.preview_tabs = vec![tab("/tmp/main.rs", None)];

    keep(view(&app));
}

#[test]
fn app_view_builds_preview_settings_overlay_when_enabled() {
    let mut app = app();
    app.screen = Screen::Preview;
    app.show_preview_settings = true;

    keep(app.view());
}

#[test]
fn view_builds_changes_layout_without_preview_header() {
    let mut app = app();
    app.screen = Screen::Project;
    app.file_manager.file_manager_show_changes = true;

    keep(view(&app));
}

#[test]
fn view_builds_preview_without_an_active_file() {
    let mut app = app();
    app.screen = Screen::Preview;
    app.active_preview_path = None;
    app.file_manager.file_manager_show_changes = false;

    keep(view(&app));
}

#[test]
fn view_builds_breadcrumb_when_active_file_has_no_matching_tab() {
    let mut app = app();
    app.screen = Screen::Project;
    app.project_path = Some("/tmp/project".to_string());
    app.active_preview_path = Some("/tmp/project/src/missing.rs".to_string());

    keep(view(&app));
}

#[test]
fn view_builds_context_menu_with_and_without_anchor() {
    let mut app = app();
    app.show_preview_context_menu = true;

    keep(view(&app));

    app.preview_context_menu_pos = Some((18.0, 27.0));
    keep(view(&app));
}

#[test]
fn view_builds_fullscreen_exit_controls() {
    let mut app = app();
    app.screen = Screen::Project;
    app.git_diff_fullscreen = true;

    keep(view(&app));
}

#[test]
fn preview_fullscreen_controls_only_show_in_code_mode() {
    let mut app = app();
    app.screen = Screen::Project;

    app.workspace_mode = WorkspaceMode::Code;
    assert!(show_fullscreen_controls(&app));

    app.workspace_mode = WorkspaceMode::Work;
    assert!(!show_fullscreen_controls(&app));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_requires_matching_active_tab() {
    let mut app = app();
    assert!(super::active_preview_lsp_badge(&app).is_none());

    app.active_preview_path = Some("/tmp/main.rs".to_string());
    assert!(super::active_preview_lsp_badge(&app).is_none());

    app.preview_tabs = vec![tab("/tmp/main.rs", Some("rust-analyzer"))];
    assert!(super::active_preview_lsp_badge(&app).is_some());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_builds_working_state_from_progress() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/main.rs".to_string());
    app.preview_tabs = vec![tab("/tmp/main.rs", Some("rust-analyzer"))];
    app.lsp_progress.insert(
        "rust-analyzer".to_string(),
        std::collections::HashMap::from([(
            "index".to_string(),
            crate::app::preview::LspProgress {
                title: "indexing".to_string(),
                message: None,
                percentage: None,
            },
        )]),
    );

    assert!(super::active_preview_lsp_badge(&app).is_some());
}
