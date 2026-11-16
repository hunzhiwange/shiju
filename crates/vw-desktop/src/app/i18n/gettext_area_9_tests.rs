use super::I18n;
use config_ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_9_existing_toast_renders_current_locale() {
    let mut app = crate::app::App::new().0;
    let raw = "用户/%n/%s";
    let _task = app.show_localized_toast(
        crate::app::state::ToastKind::Error,
        "加载失败：%s",
        vec![raw.into()],
    );
    let toast = app.active_toast.as_ref().unwrap();
    let message = toast.localized_message.as_ref().unwrap();
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "加载失败：用户/%n/%s"),
        (DesktopLocale::TraditionalChinese, "載入失敗：用户/%n/%s"),
        (DesktopLocale::English, "Loading failed: 用户/%n/%s"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_9_notifications_and_errors_retain_semantic_source() {
    let mut app = crate::app::App::new().0;
    let raw = "service 中文 %s %n";
    app.push_localized_notification("未找到 MCP 服务器：%s", vec![raw.into()]);
    assert_eq!(
        app.notifications.last().unwrap().render(I18n::new(DesktopLocale::English)),
        format!("MCP server not found: {raw}")
    );
    app.set_localized_input_error(
        super::input::ErrorField::General,
        "加载工作区列表失败: %s",
        vec![raw.into()],
    );
    app.desktop_locale = DesktopLocale::English;
    assert_eq!(
        app.render_input_error(
            super::input::ErrorField::General,
            app.error_message.as_deref().unwrap()
        ),
        format!("Failed to load workspaces: {raw}")
    );
    assert_eq!(
        app.render_input_error(super::input::ErrorField::General, "raw external error"),
        "raw external error"
    );
}
