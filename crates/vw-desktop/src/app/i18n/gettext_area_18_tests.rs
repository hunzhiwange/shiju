use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_18_uses_chinese_keys() {
    let raw = "server/%s/%n/原始";
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "即将完成..."),
        (DesktopLocale::TraditionalChinese, "即將完成..."),
        (DesktopLocale::English, "Finishing..."),
    ] {
        let rendered = super::other::lsp_finishing(I18n::new(locale), raw);
        assert_eq!(rendered, format!("LSP: {raw} ({expected})"));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_18_account_labels_follow_current_locale() {
    let (mut app, _) = crate::app::App::new();
    app.account.operation = crate::app::state::AccountOperation::StartingBrowser;
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "正在登录…"),
        (DesktopLocale::TraditionalChinese, "正在登入…"),
        (DesktopLocale::English, "Signing in…"),
    ] {
        app.desktop_locale = locale;
        assert_eq!(crate::app::components::account_status::trigger_label(&app), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_18_knowledge_feedback_switches_after_creation() {
    use crate::app::message::knowledge_tool::{self, KnowledgeToolMessage};
    let (mut app, _) = crate::app::App::new();
    let _ = knowledge_tool::update(&mut app, KnowledgeToolMessage::CreateDataset);
    let toast = app.active_toast.as_ref().expect("validation feedback");
    let message = toast.localized_message.as_ref().expect("semantic feedback");
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "知识库名称不能为空"),
        (DesktopLocale::TraditionalChinese, "知識庫名稱不能為空"),
        (DesktopLocale::English, "Knowledge base name is required"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
        assert_eq!(
            app.knowledge.notification_feedback.as_ref().unwrap().render(I18n::new(locale)),
            expected
        );
    }
}

#[test]
fn desktop_gettext_area_18_builtin_tabs_translate_without_changing_user_titles() {
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        let i18n = I18n::new(locale);
        assert_eq!(super::other::tab_label(i18n, "knowledge", "cached"), i18n.__("知识库"));
        assert_eq!(super::other::tab_label(i18n, "project:user", "知识库/%s/%n"), "知识库/%s/%n");
        assert_eq!(super::other::tool_name(i18n, "自定义/%s/%n"), "自定义/%s/%n");
    }
}
