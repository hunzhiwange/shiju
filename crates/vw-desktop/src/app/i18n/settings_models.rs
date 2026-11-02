//! Display-only model settings catalog. Provider IDs and model content stay verbatim.
use vw_config_types::ui::DesktopLocale;

pub fn text(locale: DesktopLocale, source: &str) -> &str {
    super::I18n::new(locale).__(source)
}

pub fn enabled_count(locale: DesktopLocale, enabled: usize, total: usize) -> String {
    crate::__!(super::I18n::new(locale), "%u / %u 已启用", enabled, total).expect("valid count")
}
pub fn model_count(locale: DesktopLocale, count: usize) -> String {
    crate::__!(super::I18n::new(locale), "%u 个模型", count).expect("valid count")
}
pub fn provider_count(locale: DesktopLocale, count: usize) -> String {
    crate::__!(super::I18n::new(locale), "%u 个提供商", count).expect("valid count")
}
pub fn route_title(locale: DesktopLocale, index: usize) -> String {
    crate::__!(super::I18n::new(locale), "路由 %u", index).expect("valid route")
}
pub fn model_metadata(locale: DesktopLocale, tools: bool, images: bool, context: &str) -> String {
    let i18n = super::I18n::new(locale);
    let supported = |value| i18n.__(if value { "支持" } else { "不支持" });
    crate::__!(
        i18n,
        "工具: %s · 图片输入: %s · 上下文限制: %s",
        supported(tools),
        supported(images),
        context
    )
    .expect("valid metadata")
}
pub mod connection_text;
pub mod help;
pub mod messages;
pub fn modality_label(locale: DesktopLocale, modality: &str) -> &str {
    let source = match modality {
        "text" => "文本",
        "image" => "图片",
        "audio" => "音频",
        "video" => "视频",
        _ => return modality,
    };
    super::I18n::new(locale).__(source)
}
