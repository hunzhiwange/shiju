//! Remaining desktop labels use the caller's current locale.
use super::I18n;

pub fn lsp_finishing(i18n: I18n, server: &str) -> String {
    crate::__!(i18n, "LSP: %s (即将完成...)", server).expect("valid LSP completion template")
}

pub(crate) fn tool_name(i18n: I18n, id: &str) -> String {
    let source = crate::app::state::tool_display_name(id);
    if source == id { source } else { i18n.__(&source).to_owned() }
}

pub(crate) fn tab_label(i18n: I18n, id: &str, raw: &str) -> String {
    let source = match id {
        "home" => "首页",
        "apps" => "应用",
        "usage" => "用量",
        "knowledge" => "知识库",
        "workflow_manager" => "任务编排",
        id if id == crate::apps::workflow::WORKFLOW_TOOL_TAB_ID => "工作流",
        _ => return raw.to_owned(),
    };
    i18n.__(source).to_owned()
}

pub(crate) fn session_title(app: &crate::app::App, id: &str, raw: &str) -> String {
    app.generated_session_titles
        .get(id)
        .map(|feedback| feedback.render(I18n::new(app.desktop_locale), raw))
        .unwrap_or_else(|| raw.to_owned())
}
