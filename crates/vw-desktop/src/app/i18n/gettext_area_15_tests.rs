use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_15_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "暂无通知"),
        (DesktopLocale::TraditionalChinese, "暫無通知"),
        (DesktopLocale::English, "No notifications"),
    ] {
        assert_eq!(I18n::new(locale).__("暂无通知"), expected);
    }
}

#[test]
fn desktop_gettext_area_15_counts_switch_at_display_time() {
    let message = super::format::Message::new("待审批 %u/%u", vec![2usize.into(), 7usize.into()]);
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "待审批 2/7"),
        (DesktopLocale::TraditionalChinese, "待核准 2/7"),
        (DesktopLocale::English, "Pending approval 2/7"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_15_failed_skip_preserves_error_and_switches_language() {
    use crate::app::message::{ViewMessage, view};
    let (mut app, _) = crate::app::App::new();
    let raw = "服务错误/%s/%n/中文";
    let _ = view::update(
        &mut app,
        ViewMessage::SkipAppUpdateSaved(
            crate::app::desktop_update::SkippedRelease {
                source: vw_shared::update::UpdateSource::Shiju,
                version: "99.0.0".into(),
            },
            Err(raw.into()),
        ),
    );
    let toast = app.active_toast.as_ref().expect("failure feedback");
    let message = toast.localized_message.as_ref().expect("semantic feedback");
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "跳过版本未保存："),
        (DesktopLocale::TraditionalChinese, "略過版本未儲存："),
        (DesktopLocale::English, "Skipped version was not saved: "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
}

#[test]
fn desktop_gettext_area_15_display_literals_have_both_catalog_entries() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    let calls = regex::Regex::new(r#"__\(\s*("(?:[^"\\]|\\.)*")"#).unwrap();
    let mut count = 0;
    for source in [
        include_str!("../components/about_modal.rs"),
        include_str!("../components/install_cli_modal.rs"),
        include_str!("../components/notification.rs"),
        include_str!("../components/toast.rs"),
        include_str!("../impls/app_view_modals/account_details.rs"),
        include_str!("../impls/app_view_modals/permission_modal.rs"),
        include_str!("../impls/app_view_modals/question_modal.rs"),
        include_str!("../impls/app_view_modals/rename.rs"),
        include_str!("../views/apps/modals.rs"),
        include_str!("../views/usage/raw_modal.rs"),
    ] {
        for capture in calls.captures_iter(source) {
            let key: String = serde_json::from_str(&capture[1]).unwrap();
            assert!(en.contains_key(&key), "Missing English: {key}");
            assert!(tw.contains_key(&key), "Missing Traditional Chinese: {key}");
            count += 1;
        }
    }
    assert!(count > 70);
}
