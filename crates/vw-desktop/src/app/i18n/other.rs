//! Remaining desktop labels use the caller's current locale.
use super::I18n;

pub fn lsp_finishing(i18n: I18n, server: &str) -> String {
    crate::__!(i18n, "LSP: %s (即将完成...)", server).expect("valid LSP completion template")
}

pub(crate) fn tool_name(i18n: I18n, id: &str) -> String {
    let source = crate::app::state::tool_display_name(id);
    if source == id { source } else { i18n.__(&source).to_owned() }
}
