#[derive(Debug, Clone)]
pub enum Message {
    DialogueFlowExpandEditToolSectionToggled(bool),
    DialogueFlowExpandShellToolSectionToggled(bool),
    DialogueFlowShowReasoningSummaryToggled(bool),
    CopyCode(String), None,
}

/// `DialogueFlowPermissionLoaded` 的加载结果类型。
///
/// 使用别名让顶层设置消息保持可读，并统一错误文本传递方式。
pub type DialogueFlowPermissionLoaded = Result<String, String>;
/// `DialogueFlowUiSettingsLoaded` 的加载结果类型。
///
/// 使用别名让顶层设置消息保持可读，并统一错误文本传递方式。
pub type DialogueFlowUiSettingsLoaded = Result<(bool, bool, bool), String>;
