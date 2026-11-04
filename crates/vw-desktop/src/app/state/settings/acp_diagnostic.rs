//! ACP 连接测试的类型化状态，供卡片与编辑弹窗共用。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AcpDiagnosticStatus {
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
    pub(crate) fn description(self) -> &'static str {
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
