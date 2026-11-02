use vw_config_types::ui::DesktopLocale;

pub fn text(locale: DesktopLocale, source: &'static str) -> &'static str {
    super::I18n::new(locale).__(source)
}

pub fn operation_error(locale: DesktopLocale, raw: &str) -> String {
    crate::__!(super::I18n::new(locale), "操作失败：%s", raw)
        .expect("valid operation error template")
}

pub fn domain_label(locale: DesktopLocale, index: usize) -> String {
    crate::__!(super::I18n::new(locale), "域名 %u", index).expect("valid domain label template")
}
