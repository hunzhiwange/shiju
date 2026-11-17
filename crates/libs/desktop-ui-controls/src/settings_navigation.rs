/// 系统设置标签页枚举
///
/// 定义系统设置面板中所有可用的配置分类标签页。
/// 每个变体对应一个独立的配置模块，用户可通过点击标签页切换查看。
///
/// # 变体说明
///
/// - `General` - 常规设置，包含界面、终端与编辑器配置项
/// - `DialogueFlow` - 对话流设置，控制对话流程行为
/// - `Editor` - 兼容旧入口，选择时转入常规设置的编辑器标签
/// - `Projects` - 项目设置，管理项目相关配置
/// - `Providers` - 提供商设置，配置 AI 服务提供商
/// - `Models` - 模型设置，管理可用 AI 模型
/// - `ModelRoutes` - 模型路由设置，配置模式到模型的映射
/// - `QueryClassification` - 查询模型分类设置，配置匹配规则与模型路由目标
/// - `Heartbeat` - 心跳设置，配置心跳检测参数
/// - `Cron` - Cron 设置，配置定时任务
/// - `Agents` - Agents 设置，配置 researcher / coder / reviewer 等委托代理
/// - `Reliability` - 可靠性设置，配置容错与恢复机制
/// - `Channels` - 通道设置，配置 CLI 与外部消息通道
/// - `Memory` - 记忆设置，配置记忆后端和向量检索参数
/// - `Runtime` - 运行时设置，配置执行环境与隔离选项
/// - `Acp` - ACP 设置，配置外部智能体后端
/// - `Mcp` - MCP 设置，配置全局 MCP 服务器
/// - `Autonomy` - 自治设置，配置代理自治能力
/// - `Security` - 安全设置，配置安全策略
/// - `Observability` - 可观测性设置，配置日志与监控
/// - `Storage` - 存储设置，配置持久化存储 provider 与连接参数
/// - `Proxy` - 代理设置，配置网络代理
/// - `Tunnel` - 隧道设置，配置网关公网暴露 provider
/// - `Skills` - 技能设置，配置代理可用技能
/// - `Hooks` - 钩子设置，配置运行时 hooks 与内置钩子
/// - `WebSearch` - Web 搜索设置，配置搜索 provider、凭据与请求参数
/// - `HttpRequest` - HTTP 请求设置，配置 http_request 工具白名单与网络限制
/// - `Browser` - 浏览器设置，配置 browser/browser_open 工具与后端
/// - `HttpRequest` - HTTP 请求设置，配置 http_request 工具白名单与请求限制
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemTab {
    #[cfg(not(target_arch = "wasm32"))]
    ComputerControl,
    #[cfg(not(target_arch = "wasm32"))]
    Voice,
    /// 应用目录与安装诊断
    Apps,
    /// 授权中心的只读账号列表
    #[cfg(not(target_arch = "wasm32"))]
    Auth,
    /// 常规设置标签页
    General,
    /// 兼容旧主题入口，选择时转入常规设置的主题标签
    Themes,
    /// 编辑器设置标签页
    Editor,
    /// 项目设置标签页
    Projects,
    /// 当前网关所有项目的原生归档会话。
    ArchivedChats,
    /// Gateway-owned worktree settings.
    Worktrees,
    /// 提供商设置标签页
    Providers,
    /// 模型设置标签页
    Models,
    /// 定时任务配置标签页
    Cron,
    /// 智能体配置标签页
    Agents,
    /// 通道配置标签页
    Channels,
    /// 记忆配置标签页
    Memory,
    /// 运行时配置标签页
    Runtime,
    /// 自治配置标签页
    Autonomy,
    /// 安全配置标签页
    Security,
    /// 客户端网关连接标签页
    GatewayClient,
    /// 网关配置标签页
    Gateway,
    /// 可观测性配置标签页
    Observability,
    /// 代理配置标签页
    Proxy,
    /// 隧道配置标签页
    Tunnel,
    /// 钩子配置标签页
    Hooks,
    /// 网页搜索配置标签页
    WebSearch,
    /// 网络请求配置标签页
    HttpRequest,
    /// 浏览器配置标签页
    Browser,
}

impl SystemTab {
    pub fn label(self) -> &'static str {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            SystemTab::Voice => "语音设置",
            #[cfg(not(target_arch = "wasm32"))]
            SystemTab::ComputerControl => "电脑操控",
            SystemTab::Apps => "第三方智能体",
            #[cfg(not(target_arch = "wasm32"))]
            SystemTab::Auth => "授权中心",
            SystemTab::General => "常规设置",
            SystemTab::Themes => "主题",
            SystemTab::Editor => "编辑器",
            SystemTab::Projects => "项目管理",
            SystemTab::ArchivedChats => "已归档的聊天",
            SystemTab::Worktrees => "工作树",
            SystemTab::Providers => "提供商",
            SystemTab::Models => "模型",
            SystemTab::Cron => "定时任务",
            SystemTab::Agents => "智能体",
            SystemTab::Channels => "通道配置",
            SystemTab::Memory => "记忆配置",
            SystemTab::Runtime => "运行时配置",
            SystemTab::Autonomy => "自治配置",
            SystemTab::Security => "安全配置",
            SystemTab::GatewayClient => "客服端网关",
            SystemTab::Gateway => "服务端网关",
            SystemTab::Observability => "可观测性",
            SystemTab::Proxy => "代理配置",
            SystemTab::Tunnel => "隧道配置",
            SystemTab::Hooks => "钩子配置",
            SystemTab::WebSearch => "网络配置",
            SystemTab::HttpRequest => "网络请求配置",
            SystemTab::Browser => "浏览器配置",
        }
    }
    /// 按侧边栏分组顺序列出一级设置页。
    pub fn all() -> impl Iterator<Item = SystemTab> {
        GROUPS.iter().flat_map(|(_, tabs)| tabs.iter().copied())
    }
}

pub fn is_network_tab(tab: SystemTab) -> bool {
    matches!(
        tab,
        SystemTab::WebSearch
            | SystemTab::HttpRequest
            | SystemTab::Browser
            | SystemTab::Proxy
            | SystemTab::Tunnel
    )
}

impl std::fmt::Display for SystemTab {
    /// 格式化标签页为中文名称
    ///
    /// 将每个标签页变体转换为对应的中文显示名称，
    /// 用于在 UI 界面中渲染标签页标题。
    ///
    /// # 参数
    ///
    /// - `f` - 格式化器引用
    ///
    /// # 返回值
    ///
    /// 返回 `fmt::Result`，表示格式化操作的结果。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

pub fn system_tab_keywords(tab: SystemTab) -> &'static [&'static str] {
    match tab {
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::Voice => &["voice", "microphone", "dictation", "语音", "听写", "音色"],
        SystemTab::Apps => &["apps", "应用", "安装", "catalog", "第三方智能体", "third-party agents", "cc-switch"],
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::Auth => &["auth", "accounts", "账号", "授权", "凭据", "第三方智能体", "third-party agents", "cc-switch"],
        SystemTab::General => &["general", "app", "basic", "基础", "themes", "主题"],
        SystemTab::Themes => {
            &["themes", "gallery", "palette", "custom", "builtin", "画廊", "配色", "色板"]
        }
        SystemTab::Editor => &["editor", "code", "编辑", "主题与显示", "字体大小", "行高"],
        SystemTab::Projects => &["projects", "workspace", "工作区", "Worktree", "历史项目"],
        SystemTab::Worktrees => &["worktree", "工作树", "清理", "环境"],
        SystemTab::ArchivedChats => &["archived", "chats", "归档", "聊天", "历史项目"],
        SystemTab::Providers => {
            &["providers", "api key", "model", "模型", "密钥", "base url", "catalog"]
        }
        SystemTab::Models => &["models", "llm", "ai", "provider"],
        SystemTab::Cron => &["cron", "schedule", "task", "自动化"],
        SystemTab::Agents => &["agents", "subagent", "智能体"],
        SystemTab::Channels => &["channels", "message", "通道"],
        SystemTab::Memory => &["memory", "记忆"],
        SystemTab::Runtime => &["runtime", "native", "docker", "wasm", "运行时"],
        SystemTab::Autonomy => &["autonomy", "agent run", "自治"],
        SystemTab::Security => &["security", "policy", "安全"],
        SystemTab::GatewayClient => &["gateway client", "connection", "客户端"],
        SystemTab::Gateway => &["gateway server", "api", "服务端"],
        SystemTab::Observability => &["observability", "log", "monitor", "OTel", "trace"],
        SystemTab::Proxy => &["proxy", "代理", "HTTP", "HTTPS", "ALL_PROXY", "NO_PROXY"],
        SystemTab::Tunnel => &["tunnel", "public gateway", "tailscale", "funnel", "tailnet"],
        SystemTab::Hooks => &["hooks", "tool", "command", "钩子"],
        SystemTab::WebSearch => &["network", "网络"],
        SystemTab::HttpRequest => &["http request", "network request"],
        SystemTab::Browser => &["browser", "computer use"],
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::ComputerControl => {
            &["computer control", "电脑操控", "Chrome", "Excel", "Word", "锁屏", "站点", "授权"]
        }
    }
}

pub const GROUPS: &[(&str, &[SystemTab])] = {
    use SystemTab::*;
    &[
        (
            "应用与项目",
            &[
                General,
                #[cfg(not(target_arch = "wasm32"))]
                ComputerControl,
                #[cfg(not(target_arch = "wasm32"))]
                Voice,
                Projects,
                Worktrees,
                ArchivedChats,
            ],
        ),
        ("模型与智能体", &[Models, Providers, Agents, Memory]),
        (
            "第三方智能体",
            &[
                Apps,
            ],
        ),
        ("网络与集成", &[GatewayClient, Gateway, WebSearch]),
        ("运行与安全", &[Cron, Security, Autonomy, Hooks, Observability]),
    ]
};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GeneralSettingsTab {
    #[default]
    Interface,
    Terminal,
    Editor,
    Themes,
    Companion,
}

#[cfg(test)]
#[path = "settings_navigation_tests.rs"]
mod tests;

/// Pages within the third-party area; native agent settings retain their own targets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThirdPartyPage {
    #[default]
    Agents,
    Mcp,
    Skills,
    Sessions,
    Authorization,
    Usage,
    ManagementTools,
}
impl ThirdPartyPage {
    pub const ALL: &'static [(Self, &'static str)] = &[
        (Self::Agents, "智能体管理"), (Self::Mcp, "MCP"), (Self::Skills, "Skills"),
        (Self::Sessions, "会话"), (Self::Authorization, "授权中心"),
        (Self::Usage, "用量与额度"), (Self::ManagementTools, "管理工具"),
    ];
}
