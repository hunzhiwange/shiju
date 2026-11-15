//! ACP 连接测试的类型化状态，供卡片与编辑弹窗共用。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpDiagnosticStatus {
    Running,
    Connected,
    AuthenticationRequired,
    StartupFailed,
    HandshakeFailed,
    TimedOut,
    Cancelled,
    Unavailable,
}

impl AcpDiagnosticStatus {
    pub fn description(self) -> &'static str {
        match self {
            Self::Running => "正在网关启动进程并验证 ACP 握手…",
            Self::Connected => "启动成功 · 握手成功",
            Self::AuthenticationRequired => "启动成功 · 握手成功 · 需要认证（未自动认证）",
            Self::StartupFailed => "进程启动失败，请检查网关主机上的命令与环境",
            Self::HandshakeFailed => "启动成功 · ACP 握手失败",
            Self::TimedOut => "启动成功 · ACP 握手超时",
            Self::Cancelled => "诊断已取消",
            Self::Unavailable => "网关诊断不可用或不支持，请检查网关版本与连接",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AcpSettingsState {
    pub installed: std::collections::BTreeMap<String, bool>,
    pub installing_agent: Option<String>,
    pub diagnostic_id: Option<String>,
    pub diagnostic_status: Option<AcpDiagnosticStatus>,
    pub diagnostic_agent: Option<String>,
    pub diagnostic_results: std::collections::BTreeMap<String, AcpDiagnosticStatus>,
    pub catalog: std::collections::BTreeMap<String, config_aggregate::AcpAgentConfig>,
    pub defaults:
        std::collections::BTreeMap<String, config_aggregate::AcpAgentConfig>,
    pub enabled: std::collections::BTreeSet<String>,
    pub editing_agent: Option<String>,
    pub editor_args: Vec<String>,
    pub editor_env: Vec<(String, String)>,
    pub new_agent_expanded: bool,
    pub new_agent_name_input: String,
    pub new_agent_command_input: String,
    pub new_agent_args_input: String,
    pub loading: bool,
    pub adding_agent: bool,
    pub saving_agent: Option<String>,
    pub copied_agent: Option<String>,
    pub save_error: Option<String>,
    pub status_message: Option<String>,
    pub status_translation: Option<desktop_i18n::format::Message>,
    pub error_translation: Option<desktop_i18n::format::Message>,
}

impl Default for AcpSettingsState {
    fn default() -> Self {
        Self {
            installed: Default::default(),
            installing_agent: None,
            diagnostic_id: None,
            diagnostic_status: None,
            diagnostic_agent: None,
            diagnostic_results: std::collections::BTreeMap::new(),
            catalog: std::collections::BTreeMap::new(),
            defaults: std::collections::BTreeMap::new(),
            enabled: std::collections::BTreeSet::new(),
            editing_agent: None,
            editor_args: Vec::new(),
            editor_env: Vec::new(),
            new_agent_expanded: false,
            new_agent_name_input: String::new(),
            new_agent_command_input: String::new(),
            new_agent_args_input: String::new(),
            loading: false,
            adding_agent: false,
            saving_agent: None,
            copied_agent: None,
            save_error: None,
            status_message: None,
            status_translation: None,
            error_translation: None,
        }
    }
}
