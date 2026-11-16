//! 宿主契约：主题弹窗中的真实事件经 App.update 更新草稿，取消清理草稿。
//! 局部标签按钮、各页及颜色选择器构建见领域库 desktop_split_16_tests。

use desktop_settings_appearance::system_settings_themes::editor::editor_tab_labels;
use crate::app::state::ThemeEditorTab;
use crate::app::{App, Message};
use iced::Element;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn settings_tabs_editors_appearance_theme() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::I18n;
    use crate::app::message::settings::{SettingsMessage, ThemesMessage};

    fn overlay(app: &App) -> Element<'_, Message> {
        let background =
            iced::widget::Space::new().width(iced::Length::Fill).height(iced::Length::Fill);
        super::super::editor_overlay(app, background.into())
    }

    tab_appearance_tests::run_at_least(
        concat!(module_path!(), "::settings_tabs_editors_appearance_theme"),
        420,
        || App::new().0,
        |scene, app| {
            let dispatch = |app: &mut App, message| {
                let _ = app.update(Message::Settings(SettingsMessage::Themes(message)));
            };
            let draft_name = "主题草稿 · Theme draft";
            dispatch(app, ThemesMessage::NewThemeRequested);
            dispatch(app, ThemesMessage::DraftNameChanged(draft_name.into()));
            let i18n = I18n::new(app.general_settings.desktop_locale);
            let labels: Vec<_> =
                editor_tab_labels().iter().map(|(label, _)| i18n.__(label)).collect();
            scene.save_preview(app, overlay(app), "theme");
            let message = scene.check(app, overlay(app), &labels, 1);
            assert!(matches!(
                message,
                SettingsMessage::Themes(ThemesMessage::EditorTabSelected(ThemeEditorTab::Palette))
            ));
            let _ = app.update(Message::Settings(message));
            assert_eq!(app.themes_settings.editor_tab, ThemeEditorTab::Palette);
            scene.check_modal_scroll(overlay(app), i18n.__("新建主题"), i18n.__("保存"));

            for (&label, &(_, tab)) in labels.iter().zip(editor_tab_labels()) {
                let message = scene.click(overlay(app), label);
                let _ = app.update(Message::Settings(message));
                assert_eq!(app.themes_settings.editor_tab, tab);
                assert_eq!(app.themes_settings.draft.as_ref().unwrap().name, draft_name);
            }
            let cancel = scene.click(overlay(app), i18n.__("取消"));
            assert!(matches!(cancel, SettingsMessage::Themes(ThemesMessage::CancelEdit)));
            let _ = app.update(Message::Settings(cancel));
            assert!(app.themes_settings.draft.is_none());
        },
    )
    .await;
}
