use super::*;
use crate::app::message::ProjectMessage;
fn test_app() -> App { App::new().0 }
#[tokio::test]
async fn project_edit_general_form_controls_render_and_update_draft() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::I18n;
    use crate::project_message::edit::ProjectEditTab;
    use iced::widget::Space;

    fn overlay(app: &App) -> Element<'_, Message> {
        super::super::with_project_edit(
            app,
            Space::new().width(Length::Fill).height(Length::Fill).into(),
        )
    }

    tab_appearance_tests::run_at_least(
        concat!(module_path!(), "::project_edit_general_form_controls_render_and_update_draft"),
        420,
        test_app,
        |scene, app| {
            app.project_editor.project_edit_path = Some(format!("/fixture/{}", "nested-project/".repeat(30)));
            app.project_editor.project_edit_tab = ProjectEditTab::General;
            app.project_editor.project_edit_name = "测试项目 · Sample project".into();
            app.project_editor.project_edit_icon = "圖".into();
            app.project_editor.project_edit_icon_color = "#60a5fa".into();

            let name = "修訂项目 · Renamed";
            let message = scene.edit_modal_input(overlay(app), &app.project_editor.project_edit_name, name);
            assert!(matches!(&message, Message::Project(ProjectMessage::ProjectEditNameChanged(value)) if value == name));
            let _ = app.update(message);
            assert_eq!(app.project_editor.project_edit_name, name);

            let icon = "檔";
            let message = scene.edit_modal_input(overlay(app), &app.project_editor.project_edit_icon, icon);
            assert!(matches!(&message, Message::Project(ProjectMessage::ProjectEditIconChanged(value)) if value == icon));
            let _ = app.update(message);
            assert_eq!(app.project_editor.project_edit_icon, icon);

            let i18n = I18n::new(app.desktop_locale);
            scene.check_modal_scroll(overlay(app), i18n.__("编辑项目"), i18n.__("保存"));
            assert!(matches!(
                scene.click_message(overlay(app), i18n.__("保存")),
                Message::Project(ProjectMessage::ProjectEditSaved)
            ));

            app.project_editor.project_edit_tab = ProjectEditTab::Refresh;
            app.project_editor.project_edit_session_refresh_interval_seconds_input = "23".into();
            let message = scene.edit_modal_input(overlay(app), "23", "57");
            assert!(matches!(&message, Message::Project(ProjectMessage::ProjectEditSessionRefreshIntervalSecondsInputChanged(value)) if value == "57"));
            let _ = app.update(message);
            assert_eq!(app.project_editor.project_edit_session_refresh_interval_seconds_input, "57");

            let message = scene.click_message(overlay(app), i18n.__("取消"));
            assert!(matches!(message, Message::Project(ProjectMessage::ProjectEditCanceled)));
            let _ = app.update(message);
            assert!(app.project_editor.project_edit_path.is_none());
        },
    )
    .await;
}
