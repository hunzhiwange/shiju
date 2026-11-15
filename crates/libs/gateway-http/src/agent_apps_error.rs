/// 应用读取失败，保留网关状态及结构化业务错误。
#[derive(Debug, Clone, PartialEq)]
pub enum AgentAppsReadError {
    Transport(String),
    Http { status: u16, body: serde_json::Value },
    Decode(String),
}
impl std::fmt::Display for AgentAppsReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(message) | Self::Decode(message) => f.write_str(message),
            Self::Http { status, body } => write!(f, "application read HTTP {status}: {body}"),
        }
    }
}
impl std::error::Error for AgentAppsReadError {}
