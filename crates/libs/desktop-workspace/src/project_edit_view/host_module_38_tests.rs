use super::*;
use crate::project_message::edit::Event as ProjectMessage;
fn test_app() -> Fixture {
    Fixture::new()
}
#[tokio::test]
async fn project_edit_general_form_controls_render_and_update_draft() {
    use self::appearance as tab_appearance_tests;
    use crate::project_message::edit::ProjectEditTab;
    use desktop_i18n::I18n;
    use iced::widget::Space;

    fn overlay(app: &Fixture) -> Element<'_, Message> {
        super::with_project_edit(
            app.view(),
            Space::new().width(Length::Fill).height(Length::Fill).into(),
            |event| event,
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

#[path = "../../../desktop-settings-shell/src/host_module_30_tests/tab_appearance.rs"]
mod shared;
mod appearance {
    use super::shared;
    use super::*;
    impl shared::AppearanceMessage for Message {
        type Settings = Self;
        fn none() -> Self {
            Self::None
        }
        fn settings(self) -> Self {
            self
        }
    }
    pub async fn run_at_least(
        _: &str,
        min_width: u32,
        fixture: impl Fn() -> Fixture,
        check: impl Fn(&mut shared::TabAppearance<'_, Message>, &mut Fixture),
    ) {
        use iced::advanced::renderer::Headless;
        {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in desktop_assets::fonts::load_all() {
                fonts.load_font(font);
            }
        }
        let mut renderer = iced::Renderer::new(
            iced::Font::with_name("Noto Sans CJK SC"),
            14.0.into(),
            Some("tiny-skia"),
        )
        .await
        .unwrap();
        for locale in [
            config_ui::DesktopLocale::SimplifiedChinese,
            config_ui::DesktopLocale::TraditionalChinese,
            config_ui::DesktopLocale::English,
        ] {
            for theme in [Theme::Light, Theme::Dark] {
                let mut app = fixture();
                app.desktop_locale = locale;
                app.app_theme = theme.clone();
                let mut scene = shared::TabAppearance {
                    renderer: &mut renderer,
                    theme,
                    width: if locale == config_ui::DesktopLocale::English {
                        min_width
                    } else {
                        640
                    },
                    message: std::marker::PhantomData,
                };
                check(&mut scene, &mut app);
            }
        }
    }
}
impl Fixture {
    fn update(&mut self, message: Message) {
        let Message::Project(event) = message else { panic!("unexpected project event") };
        use crate::project_message::edit::{State, handle};
        let _ = handle(
            &mut State {
                editor: &mut self.project_editor,
                hovered_recent_project: &mut None,
                project_path: &mut None,
                project_tools_menu_path: &mut None,
                project_worktree_enabled: &mut Default::default(),
                recent_project_remove_confirm_path: &mut None,
                recent_projects: &mut Vec::new(),
                recent_projects_edits: &mut Vec::new(),
                recent_projects_meta: &mut Vec::new(),
                session_menu_anchor: &mut None,
                session_menu_id: &mut None,
                task_board_settings: &mut Default::default(),
                work_current_project_menu: &mut None,
            },
            event,
        );
    }
}
