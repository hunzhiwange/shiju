// 宿主契约：App::update 驱动标签切换、禁用态与取消消息，验证草稿保持及弹窗关闭。
use crate::app::{App, Message};
use crate::app::state::ProviderHeaderDraft;
fn test_app() -> App { App::new().0 }
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn settings_tabs_editors_appearance_provider() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::settings_models::connection_text::text as tr;
    use crate::app::message::SettingsMessage;
    use crate::app::state::CustomProviderTab;

    fn overlay(app: &App) -> iced::Element<'_, Message> {
        let background =
            iced::widget::Space::new().width(iced::Length::Fill).height(iced::Length::Fill);
        super::custom_provider::view_overlays(app, background.into())
    }

    tab_appearance_tests::run_at_least(
        concat!(module_path!(), "::settings_tabs_editors_appearance_provider"),
        420,
        test_app,
        |scene, app| {
            let _ = app.update(Message::Settings(SettingsMessage::CustomProviderOpen));
            let name = "自定义草稿 · Provider draft";
            let _ = app
                .update(Message::Settings(SettingsMessage::CustomProviderNameChanged(name.into())));
            app.provider_settings.custom.headers = (0..8)
                .map(|i| ProviderHeaderDraft {
                    key: format!("x-fixture-{i}"),
                    value: "draft".into(),
                })
                .collect();
            let tr = |label| tr(app.general_settings.desktop_locale, label);
            let [basic, auth, models, title, save, cancel] =
                ["基础信息", "认证与请求头", "模型", "自定义提供商", "保存", "取消"].map(tr);
            scene.save_preview(app, overlay(app), "provider");
            let message = scene.check(app, overlay(app), &[basic, auth, models], 1);
            assert!(matches!(
                message,
                SettingsMessage::CustomProviderTabSelected(CustomProviderTab::Authentication)
            ));
            let _ = app.update(Message::Settings(message));
            scene.check_modal_scroll(overlay(app), title, save);

            app.provider_settings.custom_models_discovering = true;
            for (label, tab) in
                [(models, CustomProviderTab::Models), (basic, CustomProviderTab::Basic)]
            {
                let message = scene.click(overlay(app), label);
                let _ = app.update(Message::Settings(message));
                assert_eq!(app.provider_settings.custom_provider_tab, tab);
                scene.check_disabled(overlay(app), save);
                assert_eq!(app.provider_settings.custom.display_name, name);
                assert_eq!(app.provider_settings.custom.headers.len(), 8);
                assert!(app.provider_settings.custom_models_discovering);
            }
            let message = scene.click(overlay(app), cancel);
            assert!(matches!(message, SettingsMessage::CustomProviderClose));
            let _ = app.update(Message::Settings(message));
            assert!(!app.provider_settings.custom_provider_modal_open);
        },
    )
    .await;
}
