use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_9_notifications_and_errors_retain_semantic_source() {
    let mut notifications = Vec::new();
    let mut next_id = 0;
    let raw = "service 中文 %s %n";
    crate::components::notification::push_localized_notification(
        &mut notifications,
        &mut next_id,
        "未找到 MCP 服务器：%s",
        vec![raw.into()],
    );
    assert_eq!(
        notifications.last().unwrap().render(I18n::new(DesktopLocale::English)),
        format!("MCP server not found: {raw}")
    );
}
