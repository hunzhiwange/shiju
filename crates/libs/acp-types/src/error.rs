//! 基础 ACP 错误类型定义。

#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    #[error("acp command is empty")]
    EmptyCommand,
    #[error("failed to spawn acp agent process: {0}")]
    Spawn(std::io::Error),
    #[error("failed to capture acp stdin")]
    MissingStdin,
    #[error("failed to capture acp stdout")]
    MissingStdout,
    #[error("acp initialize failed: {0}")]
    Initialize(String),
    #[error("{0}")]
    GeminiStartupTimeout(String),
    #[error("acp new_session failed: {0}")]
    NewSession(String),
    #[error("{0}")]
    ClaudeSessionCreateTimeout(String),
    #[error("acp load_session failed: {0}")]
    LoadSession(String),
    #[error("acp resume_session failed: {0}")]
    ResumeSession(String),
    #[error("{0}")]
    SetSessionConfigOption(String),
    #[error("{0}")]
    SetSessionModel(String),
    #[error("acp prompt failed: {0}")]
    Prompt(String),
    #[error("acp cancel failed: {0}")]
    Cancel(String),
    #[error("acp prompt task join failed: {0}")]
    PromptJoin(String),
    #[error("acp session changed: expected={expected} actual={actual}")]
    SessionChanged { expected: String, actual: String },
}
