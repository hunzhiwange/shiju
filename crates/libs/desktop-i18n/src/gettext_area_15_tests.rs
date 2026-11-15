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
