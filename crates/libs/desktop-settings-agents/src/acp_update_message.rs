#[derive(Debug, Clone)]
pub enum AcpMessage {
    StartDiagnostic,
    TestAgent(String),
    InstallAgent(String),
    InstallCompleted {
        agent: String,
        result: Result<String, String>,
    },
    CancelDiagnostic,
    DiagnosticCancelled,
    DiagnosticCompleted {
        id: String,
        result: Result<String, String>,
    },
    EditAgent(String),
    CancelEditor,
    RestoreDefaultArguments,
    RestoreDefaultEnvironment,
    ArgumentChanged(usize, String),
    AddArgument,
    RemoveArgument(usize),
    EnvironmentChanged(usize, bool, String),
    AddEnvironment,
    RemoveEnvironment(usize),
    Refresh,
    Loaded(Result<desktop_config::AcpSettingsSnapshot, String>),
    NewAgentPanelToggled,
    NewAgentNameChanged(String),
    NewAgentCommandChanged(String),
    NewAgentArgsChanged(String),
    AddAgent,
    AddAgentCompleted {
        agent: String,
        result: Result<desktop_config::AcpSettingsSnapshot, String>,
    },
    CopyCommand {
        agent: String,
        command: String,
    },
    ClearCopiedCommand(String),
    SetEnabled {
        agent: String,
        enabled: bool,
    },
    SetEnabledCompleted {
        agent: String,
        enabled: bool,
        result: Result<desktop_config::AcpSettingsSnapshot, String>,
    },
}


#[derive(Debug, Clone)] pub enum Toast { Error(String), Info(String), Success(String) }
#[derive(Debug, Clone)] pub enum Message { Acp(AcpMessage), After(std::time::Duration, AcpMessage) }
