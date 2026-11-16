use super::I18n;
use config_ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_14_launch_feedback_switches_language() {
    use crate::app::message::shiju_apps::{self, ShijuAppsMessage};
    let (mut app, _) = crate::app::App::new();
    let _ = shiju_apps::update(&mut app, ShijuAppsMessage::OpenInstalled("raw/%s/%n/中文".into()));
    let toast = app.toast_state.active_toast.as_ref().expect("sign-in feedback");
    let message = toast.localized_message.as_ref().expect("semantic feedback");
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "识矩应用尚未运行就绪"),
        (DesktopLocale::TraditionalChinese, "識矩應用尚未就緒"),
        (DesktopLocale::English, "Shiju app is not ready"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_14_binding_rejection_keeps_display_semantics() {
    use crate::app::message::shiju_apps::{self, ShijuAppsMessage};
    use crate::app::shiju_apps::ShijuAppLaunchState;
    let (mut app, _) = crate::app::App::new();
    app.shiju_apps.launch = ShijuAppLaunchState::SelectingApiKey {
        generation: 5,
        installation_id: "raw/%n".into(),
        can_edit: true,
        api_keys: vec![],
        selected_api_key_id: None,
        error: None,
    };
    let _ = shiju_apps::update(&mut app, ShijuAppsMessage::SaveApiKeyBinding);
    let source = "请选择一个已启用的 IAM API 密钥";
    assert_eq!(
        app.shiju_apps.render_launch_error(source, I18n::new(DesktopLocale::English)),
        "Select an enabled IAM API key"
    );
    assert_eq!(
        app.toast_state
            .active_toast
            .as_ref()
            .unwrap()
            .localized_message
            .as_ref()
            .unwrap()
            .render(I18n::new(DesktopLocale::TraditionalChinese))
            .unwrap(),
        "請選擇已啟用的 IAM API 金鑰"
    );
}
