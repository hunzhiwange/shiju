use crate::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_9_notifications_and_errors_retain_semantic_source() {
    let raw = "service 中文 %s %n";
    let feedback = crate::input::Feedback::new("加载工作区列表失败: %s", vec![raw.into()]);
    let fallback = feedback.fallback.clone();
    let mut state = crate::input::State::default();
    state.localized_input_errors.insert(crate::input::ErrorField::General, feedback);
    assert_eq!(
        state.render_error(
            crate::input::ErrorField::General,
            I18n::new(DesktopLocale::English),
            &fallback
        ),
        format!("Failed to load workspaces: {raw}")
    );
    assert_eq!(
        state.render_error(
            crate::input::ErrorField::General,
            I18n::new(DesktopLocale::English),
            "raw external error"
        ),
        "raw external error"
    );
}

#[test]
fn desktop_gettext_area_18_builtin_tabs_translate_without_changing_user_titles() {
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        let i18n = I18n::new(locale);
        assert_eq!(crate::other::tab_label(i18n, "knowledge", "cached"), i18n.__("知识库"));
        assert_eq!(crate::other::tab_label(i18n, "project:user", "知识库/%s/%n"), "知识库/%s/%n");
    }
}
