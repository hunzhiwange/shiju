use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Font, Pixels, Rectangle, Size, Theme};

use crate::app::state::WorkResourceMode;
use crate::app::assets::Icon;
use crate::app::{App, Screen};

use super::work_resources::{self, ResourceBodyKind, panel_style, resource_header_icons};

const VIEWPORT: Size = Size::new(760.0, 720.0);

#[test]
fn browser_resource_surface_has_no_panel_border() {
    let browser = panel_style(&Theme::Dark, WorkResourceMode::Browser);
    let overview = panel_style(&Theme::Dark, WorkResourceMode::Overview);

    assert_eq!(browser.border.width, 0.0);
    assert_eq!(browser.border.radius, 0.0.into());
    assert!(overview.border.width > 0.0);
}

#[test]
fn work_resource_header_uses_direction_and_fullscreen_icons() {
    assert_eq!(
        resource_header_icons(false),
        [
            Icon::ArrowLeft,
            Icon::ArrowRight,
            Icon::LayoutTextWindow,
            Icon::Fullscreen
        ]
    );
    assert_eq!(
        resource_header_icons(true),
        [
            Icon::ArrowLeft,
            Icon::ArrowRight,
            Icon::LayoutTextWindow,
            Icon::FullscreenExit
        ]
    );
}

fn code_panel_snapshot(app: &App) -> (bool, bool, f32, bool, bool, bool) {
    (
        app.show_diff,
        app.show_file_manager,
        app.split_ratio,
        app.chat_panel_fullscreen,
        app.git_diff_fullscreen,
        app.show_preview_fullscreen_overlay,
    )
}

#[test]
fn work_resource_panel_routes_overview_workspace_files_and_browser() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    std::fs::create_dir(project.path().join("src")).expect("minimal source directory");

    let mut app = App::new().0;
    app.screen = Screen::Project;
    app.project_path = Some(project.path().to_string_lossy().into_owned());
    app.git_changed_files = vec!["src/lib.rs".into(), "README.md".into()];

    for (mode, expected) in [
        (WorkResourceMode::Overview, ResourceBodyKind::Overview),
        (WorkResourceMode::WorkspaceFiles, ResourceBodyKind::WorkspaceFiles),
        (WorkResourceMode::Browser, ResourceBodyKind::Browser),
    ] {
        app.work_resource_mode = mode;
        assert_eq!(work_resources::body_kind(&app), expected);
        let _ = work_resources::view(&app, 12.0);
    }

    let overview = work_resources::overview_model(&app);
    assert_eq!(overview.project_path, project.path().to_string_lossy());
    assert_eq!(overview.changed_files, 2);
}

#[test]
fn work_resource_files_reuse_existing_file_manager_without_mutating_code_flags() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    std::fs::create_dir(project.path().join("src")).expect("minimal source directory");

    let mut app = App::new().0;
    app.screen = Screen::Project;
    app.project_path = Some(project.path().to_string_lossy().into_owned());
    app.work_resource_mode = WorkResourceMode::WorkspaceFiles;
    app.show_diff = false;
    app.show_file_manager = false;
    app.split_ratio = 0.37;
    app.chat_panel_fullscreen = true;
    app.git_diff_fullscreen = true;
    app.show_preview_fullscreen_overlay = true;
    let before = code_panel_snapshot(&app);

    let _ = work_resources::view(&app, 12.0);

    assert_eq!(code_panel_snapshot(&app), before);
}

#[tokio::test(flavor = "multi_thread")]
async fn work_resource_panel_draws_all_modes_in_dark_and_light_themes() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let project = tempfile::tempdir().expect("temporary project should be created");
    std::fs::create_dir(project.path().join("src")).expect("minimal source directory");
    let mut app = App::new().0;
    app.screen = Screen::Project;
    app.project_path = Some(project.path().to_string_lossy().into_owned());

    for theme in [Theme::Dark, Theme::Light] {
        for mode in [
            WorkResourceMode::Overview,
            WorkResourceMode::WorkspaceFiles,
            WorkResourceMode::Browser,
        ] {
            app.work_resource_mode = mode;
            let mut element = work_resources::view(&app, 12.0);
            let limits = layout::Limits::new(Size::ZERO, VIEWPORT);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
            let viewport = Rectangle::with_size(VIEWPORT);
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style::default(),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &viewport,
            );
        }
    }
}
