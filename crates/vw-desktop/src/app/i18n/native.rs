//! Native UI fallback labels retain project names and user tab titles verbatim.
use super::I18n;

pub(crate) fn tab_title(i18n: I18n, title: String, is_fallback: bool) -> String {
    if is_fallback { i18n.__("未命名").to_owned() } else { title }
}

/// Diagnostic payloads are arguments, never translation templates.
pub(crate) fn update_diagnostic(i18n: I18n, diagnostic: &str, restart: bool) -> String {
    let source = if restart {
        "%s\n请从应用安装目录重新启动。"
    } else {
        "%s\n安装包已保留，可重新启动应用后重试。"
    };
    super::format::Message::new(source, vec![diagnostic.into()])
        .render(i18n)
        .expect("validated native update template")
}
