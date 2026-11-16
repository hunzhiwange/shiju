use super::I18n;
pub fn lsp_finishing(i18n: I18n, server: &str) -> String {
    crate::__!(i18n, "LSP: %s (即将完成...)", server).expect("valid LSP completion template")
}

pub fn tab_label(i18n: I18n, id: &str, raw: &str) -> String {
    let source = match id {
        "home" => "首页",
        "apps" => "应用",
        "usage" => "用量",
        "knowledge" => "知识库",
        "workflow_manager" => "任务编排",
        "workflow_tool" => "工作流",
        _ => return raw.to_owned(),
    };
    i18n.__(source).to_owned()
}

/// Keep service and user titles raw unless they match retained generated feedback.
pub fn session_title(
    i18n: I18n,
    raw: &str,
    preview: Option<&str>,
    feedback: Option<&super::input::Feedback>,
) -> String {
    if raw.trim().is_empty() {
        return preview.map(str::to_owned).unwrap_or_else(|| i18n.__("新会话").to_owned());
    }
    feedback.map(|feedback| feedback.render(i18n, raw)).unwrap_or_else(|| raw.to_owned())
}
