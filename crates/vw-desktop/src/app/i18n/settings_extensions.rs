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

pub(crate) fn archived_chat_count(locale: DesktopLocale, count: usize) -> String {
    crate::__!(super::I18n::new(locale), "%u 个聊天", count).expect("valid archive count template")
}

pub(crate) fn archived_chat_updated(locale: DesktopLocale, updated: u64) -> String {
    let formatted = i64::try_from(updated)
        .ok()
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|time| time.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| super::I18n::new(locale).__("未知时间").to_owned());
    crate::__!(super::I18n::new(locale), "更新于 %s", formatted)
        .expect("valid archive time template")
}
