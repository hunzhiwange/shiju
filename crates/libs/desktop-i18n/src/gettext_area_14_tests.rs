use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_14_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "暂无最近项目"),
        (DesktopLocale::TraditionalChinese, "暫無最近專案"),
        (DesktopLocale::English, "No recent projects"),
    ] {
        assert_eq!(I18n::new(locale).__("暂无最近项目"), expected);
    }
}

#[test]
fn desktop_gettext_area_14_feedback_preserves_raw_error_parameters() {
    let raw = "服务错误/%s/%n/中文";
    let message = super::format::Message::new("技能安装失败：%s", vec![raw.into()]);
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "技能安装失败："),
        (DesktopLocale::TraditionalChinese, "技能安裝失敗："),
        (DesktopLocale::English, "Skill installation failed: "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
}

#[test]
fn desktop_gettext_area_14_navigation_matches_locale_source_and_identifier() {
    let english = I18n::new(DesktopLocale::English);
    for query in ["知识库", "knowledge", "KNOWLEDGE"] {
        assert!(super::apps::matches_navigation(english, "知识库", query));
    }
    assert!(!super::apps::matches_navigation(english, "知识库", "unknown"));
    assert!(super::apps::matches_navigation(english, "任务编排", "workflow_manager"));
    assert!(super::apps::matches_navigation(
        I18n::new(DesktopLocale::TraditionalChinese),
        "任务编排",
        "任務編排"
    ));
}
