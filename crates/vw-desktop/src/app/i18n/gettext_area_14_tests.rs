use super::I18n;
use config_ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_14_launch_feedback_switches_language() {
    use crate::app::message::shiju_apps::{self, ShijuAppsMessage};
    let (mut app, _) = crate::app::App::new();
    let _ = shiju_apps::update(&mut app, ShijuAppsMessage::OpenInstalled("raw/%s/%n/中文".into()));
    let toast = app.active_toast.as_ref().expect("sign-in feedback");
    let message = toast.localized_message.as_ref().expect("semantic feedback");
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "识矩应用尚未运行就绪"),
        (DesktopLocale::TraditionalChinese, "識矩應用尚未就緒"),
        (DesktopLocale::English, "Shiju app is not ready"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
    }
}

#[test]
fn desktop_gettext_area_14_feedback_catalog_covers_message_paths() {
    let en = super::catalog::parse_catalog(include_str!("../../../../libs/desktop-i18n/src/locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("../../../../libs/desktop-i18n/src/locales/zh-TW.json")).unwrap();
    let calls = regex::Regex::new(
        r#"show_localized_toast\(\s*crate::app::state::ToastKind::\w+,\s*("(?:[^"\\]|\\.)*")"#,
    )
    .unwrap();
    let mut count = 0;
    for source in [
        include_str!("../message/shiju_apps.rs"),
        include_str!("../message/skill_hub.rs"),
        include_str!("../message/skill_hub/connectors.rs"),
        include_str!("../message/skill_hub/public.rs"),
    ] {
        for captures in calls.captures_iter(source) {
            let key: String = serde_json::from_str(&captures[1]).unwrap();
            assert!(en.contains_key(&key), "English feedback missing: {key}");
            assert!(tw.contains_key(&key), "Traditional feedback missing: {key}");
            count += 1;
        }
    }
    assert!(count > 60);
}

#[test]
fn desktop_gettext_area_14_cache_error_keeps_semantics_until_render() {
    let error = crate::app::skill_hub::SkillCacheSyncError::new("识矩 CLI 不能是符号链接");
    let message = super::format::Message::nested(
        "服务端已更新，本地缓存待同步：%s，请重试".into(),
        vec![error.message()],
    );
    let en = message.render(I18n::new(DesktopLocale::English)).unwrap();
    assert!(en.contains("Shiju CLI cannot be a symbolic link"));
    let tw = message.render(I18n::new(DesktopLocale::TraditionalChinese)).unwrap();
    assert!(tw.contains("識矩 CLI 不可為符號連結"));
    let raw = super::format::Message::literal("raw/%n/%s/中文".into());
    assert_eq!(raw.render(I18n::new(DesktopLocale::English)).unwrap(), "raw/%n/%s/中文");
}

#[test]
fn desktop_gettext_area_14_public_helpers_have_catalog_entries() {
    let en = super::catalog::parse_catalog(include_str!("../../../../libs/desktop-i18n/src/locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("../../../../libs/desktop-i18n/src/locales/zh-TW.json")).unwrap();
    let calls = regex::Regex::new(r#"\.__\(\s*("(?:[^"\\]|\\.)*")"#).unwrap();
    for source in [
        include_str!("../views/expert_hub/public_skills.rs"),
        include_str!("../views/expert_hub/list.rs"),
        include_str!("../views/expert_hub/shiju_apps/store.rs"),
        include_str!("../views/expert_hub/shiju_apps/mine.rs"),
    ] {
        for captures in calls.captures_iter(source) {
            let key: String = serde_json::from_str(&captures[1]).unwrap();
            assert!(en.contains_key(&key), "missing English: {key}");
            assert!(tw.contains_key(&key), "missing Traditional Chinese: {key}");
        }
    }
}

#[test]
fn desktop_gettext_area_14_status_display_preserves_stable_identifier() {
    let status = crate::app::skill_hub::MineSkillStatus::PendingReview;
    assert_eq!(status.as_str(), "pending_review");
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "待审核"),
        (DesktopLocale::TraditionalChinese, "待審核"),
        (DesktopLocale::English, "Pending review"),
    ] {
        assert_eq!(I18n::new(locale).__(status.label()), expected);
        assert_eq!(status.as_str(), "pending_review");
    }
}

#[test]
fn desktop_gettext_area_14_launch_error_metadata_rejects_stale_generation() {
    use crate::app::shiju_apps::{ShijuAppLaunchState, ShijuAppsState};
    let source = "当前账户没有已启用的 IAM API 密钥";
    let mut state = ShijuAppsState::default();
    state.launch = ShijuAppLaunchState::Error {
        generation: 7,
        installation_id: "raw/%n".into(),
        message: source.into(),
    };
    state.launch_feedback =
        Some((7, source.into(), crate::app::skill_hub::SkillHubError::localized(source, vec![])));
    assert_eq!(
        state.render_launch_error(source, I18n::new(DesktopLocale::English)),
        "No enabled IAM API keys for this account"
    );
    assert_eq!(
        state.render_launch_error(source, I18n::new(DesktopLocale::TraditionalChinese)),
        "目前帳戶沒有已啟用的 IAM API 金鑰"
    );
    state.launch_feedback =
        Some((6, source.into(), crate::app::skill_hub::SkillHubError::localized(source, vec![])));
    assert_eq!(state.render_launch_error(source, I18n::new(DesktopLocale::English)), source);
    let raw = "服务错误/%n/%s/中文";
    assert_eq!(state.render_launch_error(raw, I18n::new(DesktopLocale::English)), raw);
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
        app.active_toast
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
