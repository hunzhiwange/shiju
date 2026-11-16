use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_14_cache_error_keeps_semantics_until_render() {
    let error = crate::SkillCacheSyncError::new("识矩 CLI 不能是符号链接");
    let message = desktop_i18n::format::Message::nested(
        "服务端已更新，本地缓存待同步：%s，请重试".into(),
        vec![error.message()],
    );
    let en = message.render(I18n::new(DesktopLocale::English)).unwrap();
    assert!(en.contains("Shiju CLI cannot be a symbolic link"));
    let tw = message.render(I18n::new(DesktopLocale::TraditionalChinese)).unwrap();
    assert!(tw.contains("識矩 CLI 不可為符號連結"));
    let raw = desktop_i18n::format::Message::literal("raw/%n/%s/中文".into());
    assert_eq!(raw.render(I18n::new(DesktopLocale::English)).unwrap(), "raw/%n/%s/中文");
}

#[test]
fn desktop_gettext_area_14_status_display_preserves_stable_identifier() {
    let status = crate::MineSkillStatus::PendingReview;
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
