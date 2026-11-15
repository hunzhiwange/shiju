#[derive(Debug, Clone)]
pub enum Message {
    DialogueFlowExpandEditToolSectionToggled(bool),
    DialogueFlowExpandShellToolSectionToggled(bool),
    DialogueFlowShowReasoningSummaryToggled(bool),
    CopyCode(String), None,
}
