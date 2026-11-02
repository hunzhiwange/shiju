use super::{
    I18n,
    format::{Argument, Message},
};
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_sprintf_uses_chinese_keys() {
    let message = Message::new("保存失败：%s", vec![Argument::from("用户 %n 原文")]);
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "保存失败：用户 %n 原文"),
        (DesktopLocale::TraditionalChinese, "儲存失敗：用户 %n 原文"),
        (DesktopLocale::English, "Failed to save: 用户 %n 原文"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
    }
    assert_eq!(
        crate::__!(I18n::new(DesktopLocale::English), "保存失败：%s", "raw").unwrap(),
        "Failed to save: raw"
    );
}

#[test]
fn desktop_gettext_sprintf_checks_types_positions_and_limits() {
    let i = I18n::new(DesktopLocale::SimplifiedChinese);
    assert_eq!(crate::__!(i, "%2$s %1$6.3d %%", -12, "中文%n").unwrap(), "中文%n   -012 %");
    assert_eq!(crate::__!(i, "%.2s %.2f %u", "中文字符", 1.256, 9u32).unwrap(), "中文 1.26 9");
    for source in ["%n", "%x", "%", "%0$s", "%2$s", "%1$s %s", "%999999s", "%.s"] {
        assert!(i.format(source, &[]).is_err(), "{source}");
    }
    assert!(crate::__!(i, "%d", "text").is_err());
    assert!(crate::__!(i, "%s").is_err());
    assert!(crate::__!(i, "literal", 1).is_err());
    assert!(crate::__!(i, "%1$s %1$d", "text").is_err());
    assert_eq!(crate::__!(i, "%1$s %1$s", "原文").unwrap(), "原文 原文");
}
