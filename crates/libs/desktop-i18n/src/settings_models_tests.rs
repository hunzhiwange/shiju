use super::settings_models::{enabled_count, model_metadata, text};
use config_ui::DesktopLocale;
#[test]
fn desktop_i18n_settings_models_switches_visible_text() {
    for (locale, title, count) in [
        (DesktopLocale::SimplifiedChinese, "模型", "2 / 5 已启用"),
        (DesktopLocale::TraditionalChinese, "模型", "2 / 5 已啟用"),
        (DesktopLocale::English, "Models", "2 / 5 enabled"),
    ] {
        assert_eq!(text(locale, "模型"), title);
        assert_eq!(enabled_count(locale, 2, 5), count);
        let raw = "custom:原始/🚀";
        assert!(model_metadata(locale, true, false, raw).ends_with(raw));
    }
}

#[test]
fn desktop_i18n_settings_models_pending_feedback_switches_without_replacing_raw_content() {
    use super::settings_models::messages::{Feedback, render};
    let raw = "服务返回 {literal} {} 🔐 / model/保留";
    let feedback = Feedback::owned("service %s 失败: %s", vec!["restart".into(), raw.into()]);
    let cases = [
        (DesktopLocale::SimplifiedChinese, "service restart 失败: "),
        (DesktopLocale::TraditionalChinese, "service restart 失敗: "),
        (DesktopLocale::English, "service restart failed: "),
    ];
    for (locale, prefix) in cases {
        assert_eq!(feedback.display(locale), format!("{prefix}{raw}"));
        assert_eq!(Feedback::from(raw).display(locale), raw);
        let output = Feedback::from(raw)
            .join(Feedback::owned("本机 gateway 已恢复并刷新连接状态。", vec![]));
        assert_eq!(
            output.display(locale),
            format!("{raw}\n{}", render(locale, "本机 gateway 已恢复并刷新连接状态。", &[]))
        );
    }
}
