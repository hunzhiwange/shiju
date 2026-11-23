use super::{
    I18n,
    format::{Argument, Message},
};
use config_ui::DesktopLocale;

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

#[test]
fn desktop_gettext_sprintf_renders_source_for_unusable_translation() {
    use crate::format::format_translation;

    let source = "格式化回退测试：%s，数量 %d";
    let arguments = [Argument::from("用户 %n 原文"), Argument::from(7)];
    for translation in [
        "",
        " \n\t",
        "%n",
        "%",
        "%999999s",
        "%.4097s",
        "%s %u",
        "%s",
        "%s %d %s",
        "%2$s %1$d",
        "%1$s %s",
        "%2$s",
    ] {
        assert_eq!(
            format_translation(source, translation, &arguments).unwrap(),
            "格式化回退测试：用户 %n 原文，数量 7",
            "{translation:?}"
        );
    }
    assert_eq!(
        format_translation(source, "%2$d items: %1$s", &arguments).unwrap(),
        "7 items: 用户 %n 原文"
    );
    assert_eq!(format_translation("原文", " \n", &[]).unwrap(), "原文");
    assert_eq!(format_translation("", "", &[]).unwrap(), "");
}

#[test]
fn desktop_gettext_sprintf_missing_catalog_key_renders_nested_lines() {
    let source = "__desktop_sprintf_fallback_unique_20261124__ 原文：%s";
    for locale in [DesktopLocale::English, DesktopLocale::TraditionalChinese] {
        let i18n = I18n::new(locale);
        assert_eq!(i18n.__(source), source);
        let message = Message::lines(vec![
            Message::nested(
                "__desktop_sprintf_fallback_outer_20261124__ 外层：%s".into(),
                vec![Message::new(source, vec!["用户 %n 原文".into()])],
            ),
            Message::new(source, vec!["第二行".into()]),
        ]);
        assert_eq!(
            message.render(i18n).unwrap(),
            "__desktop_sprintf_fallback_outer_20261124__ 外层：__desktop_sprintf_fallback_unique_20261124__ 原文：用户 %n 原文\n__desktop_sprintf_fallback_unique_20261124__ 原文：第二行"
        );
    }
}

#[test]
fn desktop_gettext_sprintf_fallback_preserves_source_and_argument_errors() {
    use crate::format::{FormatError, format_translation};

    assert_eq!(
        format_translation("%n", "%s", &["raw".into()]),
        Err(FormatError("unsupported directive"))
    );
    assert_eq!(format_translation("%s", "%n", &[]), Err(FormatError("argument count mismatch")));
    assert_eq!(
        format_translation("%d", "%n", &["raw".into()]),
        Err(FormatError("argument type mismatch"))
    );
    assert_eq!(
        format_translation("literal", "", &[1.into()]),
        Err(FormatError("argument count mismatch"))
    );
}

#[test]
fn desktop_gettext_sprintf_strict_translation_validation_remains_available() {
    use crate::format::{FormatError, validate_translation};

    assert_eq!(validate_translation("%s %d", "%2$d %1$s"), Ok(()));
    assert_eq!(validate_translation("%s", "%n"), Err(FormatError("unsupported directive")));
    assert_eq!(
        validate_translation("%s", "%u"),
        Err(FormatError("translation signature mismatch"))
    );
    assert_eq!(validate_translation("%s", " \n"), Err(FormatError("empty translation")));
    assert_eq!(validate_translation("%s", "%4097s"), Err(FormatError("format limit exceeded")));
    assert_eq!(validate_translation("%n", "%s"), Err(FormatError("unsupported directive")));
}
