use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_9_existing_toast_renders_current_locale() {
    let raw = "用户/%n/%s";
    let (fallback, message) =
        crate::toast::localized_toast_message("加载失败：%s", vec![raw.into()]);
    let mut state = crate::toast::State::new();
    crate::toast::show_toast(
        &mut state.active_toast,
        &mut state.next_toast_id,
        crate::toast::ToastKind::Error,
        fallback,
    );
    state.active_toast.as_mut().unwrap().localized_message = Some(message);
    let message = state.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "加载失败：用户/%n/%s"),
        (DesktopLocale::TraditionalChinese, "載入失敗：用户/%n/%s"),
        (DesktopLocale::English, "Loading failed: 用户/%n/%s"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
    }
}

#[test]
fn desktop_gettext_area_10_dynamic_feedback_retains_raw_arguments() {
    let raw = "用户目录/%n/%s/原始错误";
    let (fallback, message) =
        crate::toast::localized_toast_message("已重命名为 %s", vec![raw.into()]);
    let mut state = crate::toast::State::new();
    crate::toast::show_toast(
        &mut state.active_toast,
        &mut state.next_toast_id,
        crate::toast::ToastKind::Success,
        fallback,
    );
    state.active_toast.as_mut().unwrap().localized_message = Some(message);
    let message = state.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "已重命名为 "),
        (DesktopLocale::TraditionalChinese, "已重新命名為 "),
        (DesktopLocale::English, "Renamed to "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
}
