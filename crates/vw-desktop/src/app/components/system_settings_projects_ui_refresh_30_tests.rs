//! Exercise project settings through the headless Iced runtime.
#![cfg(not(target_arch = "wasm32"))]

use super::{App, Element, Message};
use crate::app::i18n::I18n;
use crate::app::message::ProjectMessage;
use crate::app::state::ProjectEditTab;
use config_ui::GatewayClientSystemSettingsConfig;

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_30_CHILD";
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
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("{}::{test}", module_path!().split_once("::").unwrap().1)])
        .env(CHILD, "1")
        .env("HOME", home.path())
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

const PATH: &str = "/fixture/很长的專案目錄/abcdefghijklmnopqrstuvwxyz/abcdefghijklmnopqrstuvwxyz/abcdefghijklmnopqrstuvwxyz";
const NAME: &str = "项目 Fixture 專案";

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    app.navigation_tabs.project_path = None;
    app.project_editor.project_edit_path = None;
    app.workspace_state.recent_projects = vec![PATH.into()];
    app.workspace_state.recent_projects_edits = vec![NAME.into()];
    app
}

const ICON: &str = "圖";
const COLOR: &str = "#8b5cf6";
const SCRIPT: &str = "printf '專案' > start-command-ran\nprintf '第二行'";

fn set_project_draft(app: &mut App) {
    app.project_editor.project_edit_path = Some(PATH.into());
    app.project_editor.project_edit_name = NAME.into();
    app.project_editor.project_edit_icon = ICON.into();
    app.project_editor.project_edit_icon_color = COLOR.into();
    app.project_editor.project_edit_start_script = SCRIPT.into();
    app.project_editor.project_edit_start_script_editor =
        iced::widget::text_editor::Content::with_text(SCRIPT);
    app.project_editor.project_edit_worktree_enabled = true;
    app.project_editor.project_edit_task_board_settings.max_concurrent = 4;
    app.project_editor.project_edit_max_concurrent_input = "4".into();
    app.project_editor.project_edit_session_refresh_interval_seconds_input = "123".into();
    app.project_editor.project_edit_task_board_refresh_interval_seconds_input = "321".into();
    app.project_editor.project_edit_task_board_scheduler_tick_interval_seconds_input = "27".into();
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_30_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_30_preserves_values_and_disabled_states") {
        return;
    }
    let mut app = fixture();
    set_project_draft(&mut app);
    let messages = [Message::Project(ProjectMessage::ProjectEditCanceled)];
    let update = crate::app::message::project::update;
    let [Message::Project(cancel)] = messages.as_slice() else { unreachable!() };
    let dirs = crate::app::project_dirs().unwrap();
    let disk = dirs.data_local_dir().join("recent_projects_meta.json");
    let before = std::fs::read(&disk).ok();
    assert!(update(&mut app, cancel.clone()).units() == 0);
    assert!(app.project_editor.project_edit_path.is_none());
    assert_eq!(std::fs::read(&disk).ok(), before);
    assert!(!std::path::Path::new("start-command-ran").exists());

    set_project_draft(&mut app);
    let save = ProjectMessage::ProjectEditSaved;
    drop(update(&mut app, save.clone()));
    for _ in 0..100 {
        if disk.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let saved = crate::app::projects::load_recent_projects_meta();
    let project = saved.iter().find(|meta| meta.path == PATH).unwrap();
    assert_eq!(project.name, NAME);
    assert_eq!(project.icon.as_deref(), Some(ICON));
    assert_eq!(project.icon_color.as_deref(), Some(COLOR));
    assert_eq!(project.worktree_start_command.as_deref(), Some(SCRIPT));
    assert_eq!(project.session_refresh_interval_seconds, 123);
    let schedule = project.task_board_settings.as_ref().unwrap();
    assert_eq!(schedule.refresh_interval_seconds, 321);
    assert_eq!(schedule.scheduler_tick_interval_seconds, 27);
    assert_eq!(schedule.max_concurrent, 4);
    drop(update(&mut app, ProjectMessage::ProjectEditOpened(PATH.into())));
    assert_eq!(app.project_editor.project_edit_path.as_deref(), Some(PATH));
    assert_eq!(app.project_editor.project_edit_name, NAME);
    assert_eq!(app.project_editor.project_edit_icon, ICON);
    assert_eq!(app.project_editor.project_edit_icon_color, COLOR);
    assert_eq!(app.project_editor.project_edit_start_script, SCRIPT);
    assert_eq!(app.project_editor.project_edit_max_concurrent_input, "4");
    assert!(!std::path::Path::new("start-command-ran").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_peripheral_appearance_project() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::impls::with_project_edit;

    fn overlay(app: &App) -> Element<'_, Message> {
        let background =
            iced::widget::Space::new().width(iced::Length::Fill).height(iced::Length::Fill);
        with_project_edit(app, background.into())
    }

    tab_appearance_tests::run_at_least(
        concat!(module_path!(), "::settings_tabs_peripheral_appearance_project"),
        420,
        fixture,
        |scene, app| {
            set_project_draft(app);
            app.project_editor.project_edit_tab = ProjectEditTab::General;
            let i18n = I18n::new(app.general_settings.desktop_locale);
            let tabs =
                ["基础信息", "启动与工作区", "刷新策略", "调度与保护"].map(|label| i18n.__(label));
            scene.save_preview(app, overlay(app), "project-edit");
            let message = scene.check_message(app, overlay(app), &tabs, 3, 14);
            assert!(matches!(
                message,
                Message::Project(ProjectMessage::ProjectEditTabSelected(
                    ProjectEditTab::Scheduling
                ))
            ));
            let _ = app.update(message);
            assert_eq!(app.project_editor.project_edit_tab, ProjectEditTab::Scheduling);
            assert_eq!(app.project_editor.project_edit_start_script, SCRIPT);
            scene.check_modal_scroll(overlay(app), i18n.__("编辑项目"), i18n.__("保存"));
        },
    )
    .await;
}
