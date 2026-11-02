use vw_config_types::ui::DesktopLocale;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    Add,
    Delete,
    ConfirmItem,
    Cancel,
    ConfirmDelete,
    On,
    Inactive,
    AutoDetect,
    ForceEnable,
    ForceDisable,
    NoSandbox,
    AutoSelect,
    LandlockHelp,
    FirejailHelp,
    BubblewrapHelp,
    DockerBoundaryHelp,
    NoSandboxHelp,
    AutoSandboxHelp,
    BashSecurity,
    BashHelp,
    Policy,
    PolicyHelp,
    ReadOnlyConstraint,
    ReadOnlyConstraintHelp,
    DangerousShell,
    DangerousShellHelp,
    CommandRules,
    CommandRulesHelp,
    WorkspacePaths,
    WorkspacePathsHelp,
    ForbiddenPaths,
    ForbiddenPathsHelp,
    RiskLevel,
    RiskLevelHelp,
    Redirects,
    RedirectsHelp,
    ShellSyntax,
    ShellSyntaxHelp,
    EmptyCommand,
    EmptyCommandHelp,
    HeredocHelp,
    GitCommitHelp,
    JqHelp,
    Metacharacters,
    MetacharactersHelp,
    DangerousVars,
    DangerousVarsHelp,
    Substitution,
    SubstitutionHelp,
    SyntaxRedirection,
    SyntaxRedirectionHelp,
    NewlineInjection,
    NewlineInjectionHelp,
    IfsInjection,
    IfsInjectionHelp,
    ProcessEnvironment,
    ProcessEnvironmentHelp,
    Malformed,
    MalformedHelp,
    ObfuscatedFlags,
    ObfuscatedFlagsHelp,
    Backslash,
    BackslashHelp,
    BraceExpansion,
    BraceExpansionHelp,
    UnicodeWhitespace,
    UnicodeWhitespaceHelp,
    HashComment,
    HashCommentHelp,
    CommentQuote,
    CommentQuoteHelp,
    QuotedNewline,
    QuotedNewlineHelp,
    ZshDangerous,
    ZshDangerousHelp,
    ControlChars,
    ControlCharsHelp,
    SandboxEnabled,
    SandboxEnabledHelp,
    SandboxBackend,
    SandboxBackendHelp,
    BackendDescription,
    FirejailArgs,
    FirejailArgsHelp,
    FirejailExample,
    MaxMemory,
    MaxMemoryHelp,
    MaxCpuTime,
    MaxCpuTimeHelp,
    MaxSubprocesses,
    MaxSubprocessesHelp,
    MemoryMonitoring,
    MemoryMonitoringHelp,
    AuditLog,
    AuditLogHelp,
    AuditPath,
    AuditPathHelp,
    AuditMaxSize,
    AuditMaxSizeHelp,
    AuditSignature,
    AuditSignatureHelp,
    SignEvents,
    OtpHelp,
    OtpMethod,
    OtpMethodHelp,
    OtpTtlHelp,
    OtpCache,
    OtpCacheHelp,
    OtpActions,
    OtpActionsHelp,
    ShellExample,
    OtpDomains,
    OtpDomainsHelp,
    DomainExample,
    OtpCategories,
    OtpCategoriesHelp,
    PaymentsExample,
    Estop,
    EstopHelp,
    EstopFile,
    EstopFileHelp,
    ResumeOtp,
    ResumeOtpHelp,
    SyscallAnomaly,
    SyscallAnomalyHelp,
    StrictMode,
    StrictModeHelp,
    UnknownSyscall,
    UnknownSyscallHelp,
    DenyThreshold,
    DenyThresholdHelp,
    EventThreshold,
    EventThresholdHelp,
    AlertThreshold,
    AlertThresholdHelp,
    AlertCooldown,
    AlertCooldownHelp,
    SyscallLog,
    SyscallLogHelp,
    SyscallBaseline,
    SyscallBaselineHelp,
    OpenatExample,
    CanaryHelp,
    SemanticGuard,
    SemanticGuardHelp,
    SemanticCollection,
    SemanticCollectionHelp,
    SemanticThreshold,
    SemanticThresholdHelp,
    SwitchPriorityHelp,
    PolicyIntro,
    SyntaxIntro,
    Sandbox,
    SandboxIntro,
    Resources,
    ResourcesIntro,
    AuditIntro,
    OtpIntro,
    EstopIntro,
    SyscallIntro,
    Guard,
    GuardIntro,
    SecurityConfig,
    SecurityIntro,
    SecurityHelpTitle,
    SandboxResources,
    Audit,
    OtpEstop,
    Syscalls,
}
pub fn text(locale: DesktopLocale, key: Text) -> &'static str {
    let values = match key {
        Text::Add => ("添加", "新增", "Add"),
        Text::Delete => ("删除", "刪除", "Delete"),
        Text::ConfirmItem => ("确认删除该列表项？", "確定刪除此項目？", "Delete this list item?"),
        Text::Cancel => ("取消", "取消", "Cancel"),
        Text::ConfirmDelete => ("确认删除", "確認刪除", "Confirm deletion"),
        Text::On => ("开启", "開啟", "On"),
        Text::Inactive => ("已保存，当前不生效", "已儲存，目前未生效", "Saved; currently inactive"),
        Text::AutoDetect => ("自动检测", "自動偵測", "Auto-detect"),
        Text::ForceEnable => ("强制启用", "強制啟用", "Force enable"),
        Text::ForceDisable => ("强制禁用", "強制停用", "Force disable"),
        Text::NoSandbox => ("禁止沙箱", "禁止沙箱", "Disable sandbox"),
        Text::AutoSelect => ("自动选择", "自動選擇", "Automatic selection"),
        Text::LandlockHelp => (
            "使用 Linux Landlock 约束文件系统与进程权限，适合支持 Landlock 的 Linux 环境。",
            "使用 Linux Landlock 限制檔案系統與程序權限，適用於支援 Landlock 的 Linux 環境。",
            "Use Linux Landlock to restrict filesystem and process permissions on supported Linux systems.",
        ),
        Text::FirejailHelp => (
            "使用 Firejail 进行沙箱隔离，可额外传入命令行参数定制策略。",
            "使用 Firejail 隔離，可傳入額外命令列參數自訂策略。",
            "Use Firejail isolation with optional command-line arguments to customize policy.",
        ),
        Text::BubblewrapHelp => (
            "使用 Bubblewrap 构造轻量级容器隔离，适合更细粒度的命名空间控制。",
            "使用 Bubblewrap 建立輕量容器隔離，提供細緻的命名空間控制。",
            "Use Bubblewrap container isolation for fine-grained namespace control.",
        ),
        Text::DockerBoundaryHelp => (
            "使用 Docker 容器作为执行边界，适合已有容器环境的部署方式。",
            "使用 Docker 容器作為執行邊界，適用於已有容器環境的部署。",
            "Use Docker containers as the execution boundary in container-based deployments.",
        ),
        Text::NoSandboxHelp => (
            "不使用任何沙箱。仅在你明确接受更低隔离级别时使用。",
            "不使用沙箱。僅於明確接受較低隔離程度時使用。",
            "Use no sandbox. Choose only if you explicitly accept reduced isolation.",
        ),
        Text::AutoSandboxHelp => (
            "由运行时自动选择可用的最优沙箱实现；无法确定时会回退到安全允许的路径。",
            "由執行環境自動選擇可用的最佳沙箱；無法確定時退回安全允許的路徑。",
            "Automatically select the best available sandbox; fall back to a permitted safe path if uncertain.",
        ),
        Text::BashSecurity => ("Bash 安全检查", "Bash 安全檢查", "Bash security checks"),
        Text::BashHelp => (
            "统一控制命令、路径、风险、重定向与 Shell 语法审计；完全访问始终绕过。",
            "統一控制命令、路徑、風險、重新導向與 Shell 語法稽核；完整存取一律略過。",
            "Control command, path, risk, redirection and Shell syntax auditing; full access always bypasses checks.",
        ),
        Text::Policy => ("策略检查", "策略檢查", "Policy checks"),
        Text::PolicyHelp => (
            "关闭后保留各单项值，但命令、路径、风险和重定向策略均不生效。",
            "關閉後保留個別設定，但命令、路徑、風險與重新導向策略不生效。",
            "Disabling retains individual settings but deactivates command, path, risk and redirection policies.",
        ),
        Text::ReadOnlyConstraint => ("只读约束", "唯讀限制", "Read-only constraints"),
        Text::ReadOnlyConstraintHelp => (
            "阻止只读会话执行写操作。",
            "阻止唯讀工作階段執行寫入。",
            "Prevent writes in read-only sessions.",
        ),
        Text::DangerousShell => {
            ("危险 Shell 结构", "危險 Shell 結構", "Dangerous Shell structures")
        }
        Text::DangerousShellHelp => (
            "检查危险的 Shell 组合结构。",
            "檢查危險的 Shell 組合結構。",
            "Check dangerous Shell combinations.",
        ),
        Text::CommandRules => ("命令规则", "命令規則", "Command rules"),
        Text::CommandRulesHelp => (
            "应用允许命令与规则引擎约束。",
            "套用允許命令與規則引擎限制。",
            "Apply allowed-command and rule-engine constraints.",
        ),
        Text::WorkspacePaths => ("工作区路径", "工作區路徑", "Workspace paths"),
        Text::WorkspacePathsHelp => (
            "限制命令访问工作区外路径。",
            "限制命令存取工作區外的路徑。",
            "Restrict command access outside the workspace.",
        ),
        Text::ForbiddenPaths => ("禁止路径", "禁止路徑", "Forbidden paths"),
        Text::ForbiddenPathsHelp => (
            "检查明确禁止访问的路径。",
            "檢查明確禁止存取的路徑。",
            "Check explicitly forbidden paths.",
        ),
        Text::RiskLevel => ("风险等级", "風險等級", "Risk level"),
        Text::RiskLevelHelp => (
            "应用高、中风险命令策略。",
            "套用高、中風險命令策略。",
            "Apply high- and medium-risk command policies.",
        ),
        Text::Redirects => ("重定向", "重新導向", "Redirection"),
        Text::RedirectsHelp => (
            "检查 Shell 输入输出重定向。",
            "檢查 Shell 輸入輸出重新導向。",
            "Check Shell input/output redirection.",
        ),
        Text::ShellSyntax => ("Shell 语法检测", "Shell 語法偵測", "Shell syntax checks"),
        Text::ShellSyntaxHelp => (
            "关闭后保留各 validator 值，但所有 Shell 语法检测均不生效。",
            "關閉後保留各 validator 設定，但所有 Shell 語法偵測不生效。",
            "Disabling retains validator settings but deactivates all Shell syntax checks.",
        ),
        Text::EmptyCommand => ("空命令", "空命令", "Empty command"),
        Text::EmptyCommandHelp => {
            ("拒绝空白或空命令。", "拒絕空白或空命令。", "Reject blank or empty commands.")
        }
        Text::HeredocHelp => {
            ("检查 heredoc 结构。", "檢查 heredoc 結構。", "Check heredoc structures.")
        }
        Text::GitCommitHelp => (
            "检查 Git 提交命令结构。",
            "檢查 Git 提交命令結構。",
            "Check Git commit command structures.",
        ),
        Text::JqHelp => {
            ("检查 jq 命令结构。", "檢查 jq 命令結構。", "Check jq command structures.")
        }
        Text::Metacharacters => ("Shell 元字符", "Shell 元字元", "Shell metacharacters"),
        Text::MetacharactersHelp => (
            "检查敏感 Shell 元字符。",
            "檢查敏感 Shell 元字元。",
            "Check sensitive Shell metacharacters.",
        ),
        Text::DangerousVars => ("危险变量", "危險變數", "Dangerous variables"),
        Text::DangerousVarsHelp => {
            ("检查危险变量引用。", "檢查危險變數引用。", "Check dangerous variable references.")
        }
        Text::Substitution => ("命令替换", "命令替換", "Command substitution"),
        Text::SubstitutionHelp => {
            ("检查命令替换结构。", "檢查命令替換結構。", "Check command substitution structures.")
        }
        Text::SyntaxRedirection => ("语法重定向", "語法重新導向", "Syntax redirection"),
        Text::SyntaxRedirectionHelp => {
            ("检查重定向语法。", "檢查重新導向語法。", "Check redirection syntax.")
        }
        Text::NewlineInjection => ("换行注入", "換行注入", "Newline injection"),
        Text::NewlineInjectionHelp => {
            ("检查命令换行注入。", "檢查命令換行注入。", "Check command newline injection.")
        }
        Text::IfsInjection => ("IFS 注入", "IFS 注入", "IFS injection"),
        Text::IfsInjectionHelp => ("检查 IFS 注入。", "檢查 IFS 注入。", "Check IFS injection."),
        Text::ProcessEnvironment => ("进程环境", "程序環境", "Process environment"),
        Text::ProcessEnvironmentHelp => {
            ("检查 /proc 环境访问。", "檢查 /proc 環境存取。", "Check /proc environment access.")
        }
        Text::Malformed => ("异常命令", "異常命令", "Malformed commands"),
        Text::MalformedHelp => {
            ("检查格式异常的命令。", "檢查格式異常的命令。", "Check malformed commands.")
        }
        Text::ObfuscatedFlags => ("混淆参数", "混淆參數", "Obfuscated flags"),
        Text::ObfuscatedFlagsHelp => {
            ("检查混淆命令参数。", "檢查混淆命令參數。", "Check obfuscated command arguments.")
        }
        Text::Backslash => ("反斜杠", "反斜線", "Backslash"),
        Text::BackslashHelp => {
            ("检查可疑反斜杠。", "檢查可疑反斜線。", "Check suspicious backslashes.")
        }
        Text::BraceExpansion => ("花括号扩展", "大括號展開", "Brace expansion"),
        Text::BraceExpansionHelp => {
            ("检查花括号扩展。", "檢查大括號展開。", "Check brace expansion.")
        }
        Text::UnicodeWhitespace => ("Unicode 空白", "Unicode 空白", "Unicode whitespace"),
        Text::UnicodeWhitespaceHelp => (
            "检查 Unicode 空白字符。",
            "檢查 Unicode 空白字元。",
            "Check Unicode whitespace characters.",
        ),
        Text::HashComment => ("哈希注释", "井字註解", "Hash comments"),
        Text::HashCommentHelp => {
            ("检查哈希注释结构。", "檢查井字註解結構。", "Check hash comment structures.")
        }
        Text::CommentQuote => ("注释引号同步", "註解引號同步", "Comment/quote synchronization"),
        Text::CommentQuoteHelp => (
            "检查注释与引号状态不同步。",
            "檢查註解與引號狀態不同步。",
            "Check mismatched comment and quote states.",
        ),
        Text::QuotedNewline => ("引号内换行", "引號內換行", "Quoted newlines"),
        Text::QuotedNewlineHelp => (
            "拒绝引号内换行；默认关闭以支持多行提交消息。",
            "拒絕引號內換行；預設關閉以支援多行提交訊息。",
            "Reject quoted newlines; disabled by default to support multiline commit messages.",
        ),
        Text::ZshDangerous => ("Zsh 危险结构", "Zsh 危險結構", "Dangerous Zsh structures"),
        Text::ZshDangerousHelp => {
            ("检查 Zsh 危险结构。", "檢查 Zsh 危險結構。", "Check dangerous Zsh structures.")
        }
        Text::ControlChars => ("控制字符", "控制字元", "Control characters"),
        Text::ControlCharsHelp => (
            "检查命令中的控制字符。",
            "檢查命令中的控制字元。",
            "Check control characters in commands.",
        ),
        Text::SandboxEnabled => ("沙箱启用", "沙箱啟用", "Sandbox activation"),
        Text::SandboxEnabledHelp => (
            "选择自动检测、强制启用或强制禁用沙箱。",
            "選擇自動偵測、強制啟用或強制停用沙箱。",
            "Choose automatic detection, forced activation or forced deactivation.",
        ),
        Text::SandboxBackend => ("沙箱后端", "沙箱後端", "Sandbox backend"),
        Text::SandboxBackendHelp => (
            "根据启用策略收窄可选值，避免互相矛盾的配置。",
            "根據啟用策略限制選項，避免互相矛盾的設定。",
            "Restrict choices to match the activation policy and avoid conflicting settings.",
        ),
        Text::BackendDescription => ("后端说明", "後端說明", "Backend description"),
        Text::FirejailArgs => ("Firejail 参数", "Firejail 參數", "Firejail arguments"),
        Text::FirejailArgsHelp => (
            "仅在使用 Firejail 后端时生效，每次添加一个参数。",
            "僅於 Firejail 後端生效，每次新增一個參數。",
            "Applies only to Firejail; add one argument at a time.",
        ),
        Text::FirejailExample => ("例如 --private", "例如 --private", "For example --private"),
        Text::MaxMemory => ("最大内存", "最大記憶體", "Maximum memory"),
        Text::MaxMemoryHelp => (
            "限制执行进程可使用的最大内存。",
            "限制執行程序可用的最大記憶體。",
            "Limit the execution process memory.",
        ),
        Text::MaxCpuTime => ("最大 CPU 时长", "最大 CPU 時間", "Maximum CPU time"),
        Text::MaxCpuTimeHelp => (
            "限制单次执行可消耗的 CPU 时间。",
            "限制單次執行可消耗的 CPU 時間。",
            "Limit CPU time per execution.",
        ),
        Text::MaxSubprocesses => ("最大子进程数", "最大子程序數", "Maximum subprocesses"),
        Text::MaxSubprocessesHelp => (
            "限制执行过程中允许派生的子进程数量。",
            "限制執行時允許衍生的子程序數量。",
            "Limit subprocesses spawned during execution.",
        ),
        Text::MemoryMonitoring => ("内存监控", "記憶體監控", "Memory monitoring"),
        Text::MemoryMonitoringHelp => (
            "持续监控执行进程的内存占用。",
            "持續監控執行程序的記憶體使用量。",
            "Continuously monitor execution process memory usage.",
        ),
        Text::AuditLog => ("审计日志", "稽核日誌", "Audit log"),
        Text::AuditLogHelp => (
            "记录安全相关动作和事件。",
            "記錄安全相關動作與事件。",
            "Record security actions and events.",
        ),
        Text::AuditPath => ("审计日志路径", "稽核日誌路徑", "Audit log path"),
        Text::AuditPathHelp => (
            "指定审计日志文件的写入位置。",
            "指定稽核日誌檔案位置。",
            "Choose the audit log file location.",
        ),
        Text::AuditMaxSize => ("审计大小上限", "稽核大小上限", "Maximum audit size"),
        Text::AuditMaxSizeHelp => (
            "限制单个审计日志文件的最大大小。",
            "限制單個稽核日誌檔案的最大大小。",
            "Limit individual audit log file size.",
        ),
        Text::AuditSignature => ("审计签名", "稽核簽章", "Audit signing"),
        Text::AuditSignatureHelp => (
            "为审计事件添加签名，提高完整性。",
            "為稽核事件新增簽章以提高完整性。",
            "Sign audit events to improve integrity.",
        ),
        Text::SignEvents => ("签名事件", "簽署事件", "Sign events"),
        Text::OtpHelp => (
            "启用敏感动作的二次验证。",
            "啟用敏感動作的二次驗證。",
            "Enable secondary verification for sensitive actions.",
        ),
        Text::OtpMethod => ("OTP 方法", "OTP 方法", "OTP method"),
        Text::OtpMethodHelp => (
            "支持 totp、pairing、cli-prompt。",
            "支援 totp、pairing、cli-prompt。",
            "Supports totp, pairing and cli-prompt.",
        ),
        Text::OtpTtlHelp => (
            "单个 OTP 令牌的有效时长。",
            "單個 OTP 權杖的有效時間。",
            "Lifetime of an individual OTP token.",
        ),
        Text::OtpCache => ("OTP 缓存时长", "OTP 快取時間", "OTP cache lifetime"),
        Text::OtpCacheHelp => (
            "验证结果的缓存有效期。",
            "驗證結果的快取有效期。",
            "Lifetime of cached verification results.",
        ),
        Text::OtpActions => ("OTP 动作白名单", "OTP 動作白名單", "OTP action allowlist"),
        Text::OtpActionsHelp => (
            "需要 OTP 验证的敏感动作，每次添加一个。",
            "需要 OTP 驗證的敏感動作，每次新增一個。",
            "Sensitive actions requiring OTP; add one at a time.",
        ),
        Text::ShellExample => ("例如 shell", "例如 shell", "For example shell"),
        Text::OtpDomains => ("OTP 域名规则", "OTP 網域規則", "OTP domain rules"),
        Text::OtpDomainsHelp => (
            "需要 OTP 验证的网络域名规则，每次添加一个。",
            "需要 OTP 驗證的網域規則，每次新增一個。",
            "Domain rules requiring OTP; add one at a time.",
        ),
        Text::DomainExample => {
            ("例如 admin.example.com", "例如 admin.example.com", "For example admin.example.com")
        }
        Text::OtpCategories => ("OTP 域名分类", "OTP 網域分類", "OTP domain categories"),
        Text::OtpCategoriesHelp => (
            "按域名分类启用 OTP 门控，每次添加一个分类。",
            "依網域分類啟用 OTP 門控，每次新增一個分類。",
            "Enable OTP gating by domain category; add one category at a time.",
        ),
        Text::PaymentsExample => ("例如 payments", "例如 payments", "For example payments"),
        Text::Estop => ("紧急停止", "緊急停止", "Emergency stop"),
        Text::EstopHelp => (
            "启用紧急停止状态机。",
            "啟用緊急停止狀態機。",
            "Enable the emergency-stop state machine.",
        ),
        Text::EstopFile => ("E-Stop 状态文件", "E-Stop 狀態檔案", "E-Stop state file"),
        Text::EstopFileHelp => (
            "指定紧急停止状态文件位置。",
            "指定緊急停止狀態檔案位置。",
            "Choose the emergency-stop state file location.",
        ),
        Text::ResumeOtp => ("恢复需 OTP", "恢復需 OTP", "Require OTP to resume"),
        Text::ResumeOtpHelp => (
            "从 E-Stop 恢复时要求额外 OTP 验证。",
            "從 E-Stop 恢復時要求額外 OTP 驗證。",
            "Require additional OTP verification when resuming from E-Stop.",
        ),
        Text::SyscallAnomaly => {
            ("系统调用异常检测", "系統呼叫異常偵測", "Syscall anomaly detection")
        }
        Text::SyscallAnomalyHelp => (
            "启用系统调用层面的异常行为检测。",
            "啟用系統呼叫層級的異常行為偵測。",
            "Enable anomaly detection at the syscall level.",
        ),
        Text::StrictMode => ("严格模式", "嚴格模式", "Strict mode"),
        Text::StrictModeHelp => (
            "使用更严格的异常检测规则。",
            "使用更嚴格的異常偵測規則。",
            "Use stricter anomaly detection rules.",
        ),
        Text::UnknownSyscall => ("未知系统调用告警", "未知系統呼叫警示", "Unknown syscall alerts"),
        Text::UnknownSyscallHelp => (
            "遇到未知系统调用时立即告警。",
            "遇到未知系統呼叫時立即警示。",
            "Alert immediately on unknown syscalls.",
        ),
        Text::DenyThreshold => ("每分钟拒绝阈值", "每分鐘拒絕閾值", "Denials per minute"),
        Text::DenyThresholdHelp => (
            "单位时间内拒绝事件的上限。",
            "單位時間內拒絕事件的上限。",
            "Maximum denied events per time interval.",
        ),
        Text::EventThreshold => ("每分钟总事件阈值", "每分鐘總事件閾值", "Events per minute"),
        Text::EventThresholdHelp => (
            "单位时间内允许的系统调用事件总数。",
            "單位時間內允許的系統呼叫事件總數。",
            "Total allowed syscall events per time interval.",
        ),
        Text::AlertThreshold => ("每分钟告警阈值", "每分鐘警示閾值", "Alerts per minute"),
        Text::AlertThresholdHelp => (
            "限制单位时间内的告警触发数量。",
            "限制單位時間內觸發的警示數量。",
            "Limit alerts triggered per time interval.",
        ),
        Text::AlertCooldown => ("告警冷却", "警示冷卻", "Alert cooldown"),
        Text::AlertCooldownHelp => (
            "两次告警之间的最小间隔。",
            "兩次警示之間的最短間隔。",
            "Minimum interval between alerts.",
        ),
        Text::SyscallLog => ("系统调用日志路径", "系統呼叫日誌路徑", "Syscall log path"),
        Text::SyscallLogHelp => (
            "指定系统调用异常日志的写入位置。",
            "指定系統呼叫異常日誌位置。",
            "Choose the syscall anomaly log location.",
        ),
        Text::SyscallBaseline => ("系统调用基线", "系統呼叫基準", "Syscall baseline"),
        Text::SyscallBaselineHelp => (
            "允许的系统调用白名单基线，每次添加一个。",
            "允許的系統呼叫白名單基準，每次新增一個。",
            "Allowed syscall baseline; add one syscall at a time.",
        ),
        Text::OpenatExample => ("例如 openat", "例如 openat", "For example openat"),
        Text::CanaryHelp => (
            "启用蜜罐令牌与外泄检测。",
            "啟用誘捕權杖與外洩偵測。",
            "Enable canary tokens and exfiltration detection.",
        ),
        Text::SemanticGuard => ("语义注入防护", "語意注入防護", "Semantic injection protection"),
        Text::SemanticGuardHelp => (
            "启用语义层面的注入攻击检测。",
            "啟用語意層級的注入攻擊偵測。",
            "Enable semantic injection attack detection.",
        ),
        Text::SemanticCollection => ("语义集合", "語意集合", "Semantic collection"),
        Text::SemanticCollectionHelp => (
            "指定用于语义分析的数据集合。",
            "指定語意分析使用的資料集合。",
            "Choose the collection used for semantic analysis.",
        ),
        Text::SemanticThreshold => ("语义阈值", "語意閾值", "Semantic threshold"),
        Text::SemanticThresholdHelp => (
            "语义防护的触发阈值。",
            "語意防護的觸發閾值。",
            "Threshold for triggering semantic protection.",
        ),
        Text::SwitchPriorityHelp => (
            "优先级为总开关 → 分组开关 → 单项开关；关闭父级不会改写已保存的子项。",
            "優先順序為總開關 → 群組開關 → 個別開關；關閉父級不會改寫已儲存的子項。",
            "Priority: master → group → individual switches; disabling a parent preserves saved child settings.",
        ),
        Text::PolicyIntro => (
            "命令、路径、风险与重定向策略。",
            "命令、路徑、風險與重新導向策略。",
            "Command, path, risk and redirection policies.",
        ),
        Text::SyntaxIntro => (
            "按 validator 独立控制 Shell 语法检查。",
            "依 validator 獨立控制 Shell 語法檢查。",
            "Control Shell syntax checks per validator.",
        ),
        Text::Sandbox => ("沙箱", "沙箱", "Sandbox"),
        Text::SandboxIntro => (
            "控制是否启用沙箱以及选择具体后端。",
            "控制是否啟用沙箱並選擇後端。",
            "Enable the sandbox and choose its backend.",
        ),
        Text::Resources => ("资源限制", "資源限制", "Resource limits"),
        Text::ResourcesIntro => (
            "约束进程的内存、CPU 与子进程数量。",
            "限制程序的記憶體、CPU 與子程序數量。",
            "Limit process memory, CPU and subprocess count.",
        ),
        Text::AuditIntro => (
            "记录并限制安全事件日志。",
            "記錄並限制安全事件日誌。",
            "Record and limit security event logs.",
        ),
        Text::OtpIntro => (
            "对敏感动作启用二次验证。",
            "對敏感動作啟用二次驗證。",
            "Enable secondary verification for sensitive actions.",
        ),
        Text::EstopIntro => (
            "配置 E-Stop 状态与恢复要求。",
            "設定 E-Stop 狀態與恢復要求。",
            "Configure E-Stop state and recovery requirements.",
        ),
        Text::SyscallIntro => (
            "监控系统调用行为并限制告警节奏。",
            "監控系統呼叫行為並限制警示頻率。",
            "Monitor syscalls and limit alert frequency.",
        ),
        Text::Guard => ("防护", "防護", "Protection"),
        Text::GuardIntro => (
            "配置 Canary Token 与语义注入防护。",
            "設定 Canary Token 與語意注入防護。",
            "Configure Canary Token and semantic injection protection.",
        ),
        Text::SecurityConfig => ("安全配置", "安全設定", "Security configuration"),
        Text::SecurityIntro => (
            "配置沙箱、资源限制、审计与安全门控策略。",
            "設定沙箱、資源限制、稽核與安全門控策略。",
            "Configure sandboxing, resource limits, auditing and security gating.",
        ),
        Text::SecurityHelpTitle => {
            ("Security 配置帮助", "Security 設定說明", "Security configuration help")
        }
        Text::SandboxResources => ("沙箱与资源", "沙箱與資源", "Sandbox and resources"),
        Text::Audit => ("审计", "稽核", "Audit"),
        Text::OtpEstop => ("OTP 与急停", "OTP 與急停", "OTP and emergency stop"),
        Text::Syscalls => ("系统调用", "系統呼叫", "Syscalls"),
    };
    match locale {
        DesktopLocale::SimplifiedChinese => values.0,
        DesktopLocale::TraditionalChinese => values.1,
        DesktopLocale::English => values.2,
    }
}

pub fn help(locale: DesktopLocale) -> String {
    let content = match locale {
        DesktopLocale::SimplifiedChinese => {
            "安全配置说明\n\n一、作用\n- security 用于配置沙箱、资源限制、审计、OTP 门控、紧急停止与系统调用异常检测。\n\n二、关键字段\n- bash.enabled: 总开关，默认 false；完全访问始终绕过。\n- bash.policy.enabled / bash.validators.enabled: 分组开关。\n- bash.policy.* / bash.validators.*: 单项开关；有效状态为总开关 && 分组开关 && 单项开关。\n- sandbox.enabled: auto/true/false。\n- sandbox.backend: auto | landlock | firejail | bubblewrap | docker | none。\n- resources.*: 进程资源上限。\n- audit.*: 审计日志行为。\n- otp.*: 敏感动作二次验证。\n- estop.*: 紧急停止状态机。\n- syscall_anomaly.*: 系统调用异常检测。\n- canary_tokens / semantic_guard*: 注入与外泄防护。\n\n三、示例\n{\n  \"security\": {\n    \"bash\": {\n      \"enabled\": true,\n      \"policy\": {\"enabled\": true, \"read_only\": true, \"redirects\": true},\n      \"validators\": {\"enabled\": true, \"heredoc\": true, \"quoted_newline\": false}\n    },\n    \"sandbox\": {\"enabled\": null, \"backend\": \"auto\", \"firejail_args\": []},\n    \"resources\": {\n      \"max_memory_mb\": 512,\n      \"max_cpu_time_seconds\": 60,\n      \"max_subprocesses\": 10,\n      \"memory_monitoring\": true\n    },\n    \"audit\": {\"enabled\": true, \"log_path\": \"audit.log\", \"max_size_mb\": 100, \"sign_events\": false}\n  }\n}\n"
        }
        DesktopLocale::TraditionalChinese => {
            "安全設定說明\nsecurity 設定沙箱、資源限制、稽核、OTP、緊急停止與系統呼叫偵測。\nbash.enabled：總開關，預設 false；完整存取一律略過。\nbash.policy.enabled / bash.validators.enabled：群組開關。\nbash.policy.* / bash.validators.*：個別開關；有效狀態為總開關 && 群組開關 && 個別開關。\nsandbox.enabled：auto/true/false。\nsandbox.backend：auto / landlock / firejail / bubblewrap / docker / none。\nresources.*：程序資源上限。\naudit.*：稽核日誌。\notp.*：敏感動作二次驗證。\nestop.*：緊急停止狀態機。\nsyscall_anomaly.*：系統呼叫異常偵測。\ncanary_tokens / semantic_guard*：注入與外洩防護。"
        }
        DesktopLocale::English => {
            "Security configuration help\nsecurity configures sandboxing, resource limits, auditing, OTP, emergency stop and syscall detection.\nbash.enabled: master switch, default false; full access always bypasses checks.\nbash.policy.enabled / bash.validators.enabled: group switches.\nbash.policy.* / bash.validators.*: individual switches; effective state is master && group && individual.\nsandbox.enabled: auto/true/false.\nsandbox.backend: auto / landlock / firejail / bubblewrap / docker / none.\nresources.*: process resource limits.\naudit.*: audit logging.\notp.*: secondary verification for sensitive actions.\nestop.*: emergency-stop state machine.\nsyscall_anomaly.*: syscall anomaly detection.\ncanary_tokens / semantic_guard*: injection and exfiltration protection."
        }
    };
    if locale == DesktopLocale::SimplifiedChinese {
        content.to_owned()
    } else {
        format!(
            "{content}\n\n{}",
            "{\n  \"security\": {\n    \"bash\": {\n      \"enabled\": true,\n      \"policy\": {\"enabled\": true, \"read_only\": true, \"redirects\": true},\n      \"validators\": {\"enabled\": true, \"heredoc\": true, \"quoted_newline\": false}\n    },\n    \"sandbox\": {\"enabled\": null, \"backend\": \"auto\", \"firejail_args\": []},\n    \"resources\": {\n      \"max_memory_mb\": 512,\n      \"max_cpu_time_seconds\": 60,\n      \"max_subprocesses\": 10,\n      \"memory_monitoring\": true\n    },\n    \"audit\": {\"enabled\": true, \"log_path\": \"audit.log\", \"max_size_mb\": 100, \"sign_events\": false}\n  }\n}"
        )
    }
}
