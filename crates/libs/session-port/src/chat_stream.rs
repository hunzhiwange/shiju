//! 服务端聊天流收集结果，不包含收集或执行动作。

#[derive(Debug, Clone)]
pub struct GatewayChatStreamCollected {
    pub output: String,
    pub message_id: Option<String>,
    pub parent_message_id: Option<String>,
}
