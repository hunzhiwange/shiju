use super::*;

/// 项目工作区的界面呈现模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkspaceMode {
    /// 完整代码工作区。
    #[default]
    Code,
    /// 聚焦会话的简洁工作区。
    Work,
}

/// 工作模式右侧资源区的呈现模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkResourceMode {
    /// 展示当前项目与会话摘要。
    #[default]
    Overview,
    /// 展示当前项目的工作空间文件。
    WorkspaceFiles,
    /// 展示内置浏览器承载面。
    Browser,
}

/// 工作模式左栏的项目范围过滤。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkSidebarFilterScope {
    /// 显示所有最近项目中的会话。
    #[default]
    All,
    /// 只显示当前打开项目中的会话。
    CurrentProject,
}

/// 工作模式左栏的会话运行状态过滤。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkSidebarFilterStatus {
    /// 不限制运行状态。
    #[default]
    All,
    /// 当前正在处理请求。
    InProgress,
    /// 没有运行中请求和待处理队列。
    Completed,
    /// 当前未运行、但队列中仍有待处理请求。
    Pending,
}

/// 工作模式左栏的更新时间过滤。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkSidebarFilterTime {
    /// 不限制更新时间。
    #[default]
    All,
    /// 最近 24 小时内更新。
    LastDay,
    /// 最近 7 天内更新。
    LastSevenDays,
    /// 最近 30 天内更新。
    LastThirtyDays,
}

pub const WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS: usize = 10;
pub const WORK_SIDEBAR_LOAD_MORE_STEP: usize = 5;

/// 工作模式左栏当前展示的列表。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkSidebarTab {
    /// 最近任务。
    #[default]
    Tasks,
    /// 最近空间。
    Spaces,
}

impl WorkspaceMode {
    /// 返回持久化到应用配置中的稳定标识。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Work => "work",
        }
    }

    /// 解析单个配置值，非法值保持显式失败。
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "code" => Some(Self::Code),
            "work" => Some(Self::Work),
            _ => None,
        }
    }

    /// 从可选配置读取模式；缺失或非法值使用安全的代码模式。
    pub fn from_config(value: Option<&str>) -> Self {
        value.and_then(Self::parse).unwrap_or_default()
    }
}

/// 模型弹出窗口悬停状态
///
/// 表示在模型选择弹出窗口中的悬停项，
/// 可以是具体的模型或纯文本提示。
#[derive(Debug, Clone)]
pub enum ModelPopoverHover {
    /// 悬停在特定模型上
    Model {
        /// 提供者标识符
        provider_id: String,
        /// 模型标识符
        model_id: String,
        /// 悬停锚点，用于 tooltip 绝对定位
        anchor: Option<crate::app::components::model_hover_tooltip::HoverAnchor>,
    },
    /// 悬停在文本上
    Text {
        /// 提示文本
        text: String,
        /// 悬停锚点，用于 tooltip 绝对定位
        anchor: Option<crate::app::components::model_hover_tooltip::HoverAnchor>,
    },
}

/// 应用标签页
///
/// 表示应用中的一个标签页，包含屏幕类型和项目路径等信息。
#[derive(Debug, Clone, PartialEq)]
pub struct AppTab {
    /// 标签页的唯一标识符
    pub id: String,
    /// 标签页标题
    pub title: String,
    /// 标签页对应的屏幕
    pub screen: Screen,
    /// 关联的项目路径（可选）
    pub project_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopBarGatewayTab {
    Gateway,
    Mcp,
    Lsp,
    Plugins,
}

impl Default for TopBarGatewayTab {
    fn default() -> Self {
        Self::Gateway
    }
}

/// 外部打开应用枚举
///
/// 定义可以使用外部打开功能的应用程序类型，
/// 包括编辑器、终端和文件管理器等。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExternalOpenApp {
    /// macOS Finder 文件管理器
    Finder,
    /// Visual Studio Code 编辑器
    VSCode,
    /// Cursor 编辑器
    Cursor,
    /// Trae 编辑器
    Trae,
    /// Windsurf 编辑器
    Windsurf,
    /// Kiro 编辑器
    Kiro,
    /// Zed 编辑器
    Zed,
    /// TextMate 编辑器
    TextMate,
    /// Antigravity 编辑器
    Antigravity,
    /// macOS Terminal 终端
    Terminal,
    /// iTerm2 终端
    ITerm2,
    /// Ghostty 终端
    Ghostty,
    /// Xcode IDE
    Xcode,
    /// Android Studio IDE
    AndroidStudio,
    /// PowerShell 终端
    PowerShell,
    /// Sublime Text 编辑器
    SublimeText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimePlatform {
    MacOs,
    Windows,
    Linux,
}

impl RuntimePlatform {
    pub(crate) fn from_gateway_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "macos" | "darwin" | "mac" => Some(Self::MacOs),
            "windows" | "win32" => Some(Self::Windows),
            "linux" => Some(Self::Linux),
            _ => None,
        }
    }

    pub(crate) fn current_target() -> Option<Self> {
        #[cfg(target_os = "macos")]
        {
            Some(Self::MacOs)
        }

        #[cfg(windows)]
        {
            return Some(Self::Windows);
        }

        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "macos"), not(windows)))]
        {
            return Some(Self::Linux);
        }

        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    }

    pub(crate) const fn file_manager_label(self) -> &'static str {
        match self {
            Self::MacOs => "Finder",
            Self::Windows => "File Explorer",
            Self::Linux => "File Manager",
        }
    }
}

impl ExternalOpenApp {
    /// 获取应用的字符串标识符
    ///
    /// # 返回值
    ///
    /// 返回应用的小写字符串标识符
    pub fn as_str(&self) -> &'static str {
        match self {
            ExternalOpenApp::Finder => "finder",
            ExternalOpenApp::VSCode => "vscode",
            ExternalOpenApp::Cursor => "cursor",
            ExternalOpenApp::Trae => "trae",
            ExternalOpenApp::Windsurf => "windsurf",
            ExternalOpenApp::Kiro => "kiro",
            ExternalOpenApp::Zed => "zed",
            ExternalOpenApp::TextMate => "textmate",
            ExternalOpenApp::Antigravity => "antigravity",
            ExternalOpenApp::Terminal => "terminal",
            ExternalOpenApp::ITerm2 => "iterm2",
            ExternalOpenApp::Ghostty => "ghostty",
            ExternalOpenApp::Xcode => "xcode",
            ExternalOpenApp::AndroidStudio => "android-studio",
            ExternalOpenApp::PowerShell => "powershell",
            ExternalOpenApp::SublimeText => "sublime-text",
        }
    }

    /// 获取应用的显示标签（中文）
    ///
    /// # 返回值
    ///
    /// 返回应用的中文显示名称
    pub fn label(&self) -> &'static str {
        match self {
            ExternalOpenApp::Finder => "文件管理器",
            ExternalOpenApp::VSCode => "VS Code",
            ExternalOpenApp::Cursor => "Cursor",
            ExternalOpenApp::Trae => "Trae",
            ExternalOpenApp::Windsurf => "Windsurf",
            ExternalOpenApp::Kiro => "Kiro",
            ExternalOpenApp::Zed => "Zed",
            ExternalOpenApp::TextMate => "TextMate",
            ExternalOpenApp::Antigravity => "Antigravity",
            ExternalOpenApp::Terminal => "Terminal",
            ExternalOpenApp::ITerm2 => "iTerm2",
            ExternalOpenApp::Ghostty => "Ghostty",
            ExternalOpenApp::Xcode => "Xcode",
            ExternalOpenApp::AndroidStudio => "Android Studio",
            ExternalOpenApp::PowerShell => "PowerShell",
            ExternalOpenApp::SublimeText => "Sublime Text",
        }
    }
}

/// 约定式提交类型枚举
///
/// 定义 Git 约定式提交的消息类型，
/// 遵循 Conventional Commits 规范。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConventionalCommitType {
    /// 新功能（feat）
    Feat,
    /// Bug 修复（fix）
    Fix,
    /// 文档变更（docs）
    Docs,
    /// 代码格式调整（style）
    Style,
    /// 代码重构（refactor）
    Refactor,
    /// 性能优化（perf）
    Perf,
    /// 测试相关（test）
    Test,
    /// 构建系统（build）
    Build,
    /// CI/CD 配置（ci）
    Ci,
    /// 其他修改（chore）
    Chore,
    /// 回退提交（revert）
    Revert,
    /// 初始化（init）
    Init,
    /// 配置变更（config）
    Config,
    /// 发布（release）
    Release,
    /// 部署（deploy）
    Deploy,
    /// 合并（merge）
    Merge,
    /// 进行中（wip）
    Wip,
    /// 拼写错误（typo）
    Typo,
    /// 国际化（locale）
    Locale,
}

impl ConventionalCommitType {
    /// 获取所有提交类型的数组
    ///
    /// # 返回值
    ///
    /// 返回包含所有 `ConventionalCommitType` 变体的数组
    pub(crate) const fn all() -> [ConventionalCommitType; 19] {
        [
            ConventionalCommitType::Feat,
            ConventionalCommitType::Fix,
            ConventionalCommitType::Docs,
            ConventionalCommitType::Style,
            ConventionalCommitType::Refactor,
            ConventionalCommitType::Perf,
            ConventionalCommitType::Test,
            ConventionalCommitType::Build,
            ConventionalCommitType::Ci,
            ConventionalCommitType::Chore,
            ConventionalCommitType::Revert,
            ConventionalCommitType::Init,
            ConventionalCommitType::Config,
            ConventionalCommitType::Release,
            ConventionalCommitType::Deploy,
            ConventionalCommitType::Merge,
            ConventionalCommitType::Wip,
            ConventionalCommitType::Typo,
            ConventionalCommitType::Locale,
        ]
    }

    /// 将提交类型转换为字符串
    ///
    /// # 返回值
    ///
    /// 返回提交类型对应的小写字符串标识
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            ConventionalCommitType::Feat => "feat",
            ConventionalCommitType::Fix => "fix",
            ConventionalCommitType::Docs => "docs",
            ConventionalCommitType::Style => "style",
            ConventionalCommitType::Refactor => "refactor",
            ConventionalCommitType::Perf => "perf",
            ConventionalCommitType::Test => "test",
            ConventionalCommitType::Build => "build",
            ConventionalCommitType::Ci => "ci",
            ConventionalCommitType::Chore => "chore",
            ConventionalCommitType::Revert => "revert",
            ConventionalCommitType::Init => "init",
            ConventionalCommitType::Config => "config",
            ConventionalCommitType::Release => "release",
            ConventionalCommitType::Deploy => "deploy",
            ConventionalCommitType::Merge => "merge",
            ConventionalCommitType::Wip => "wip",
            ConventionalCommitType::Typo => "typo",
            ConventionalCommitType::Locale => "locale",
        }
    }

    /// 从前缀字符串解析提交类型
    ///
    /// # 参数
    ///
    /// - `s`：前缀字符串（如 "feat"、"fix" 等）
    ///
    /// # 返回值
    ///
    /// 如果匹配成功，返回对应的 `ConventionalCommitType`；
    /// 否则返回 `None`
    #[allow(dead_code)]
    pub(crate) fn from_prefix(s: &str) -> Option<ConventionalCommitType> {
        match s {
            "feat" => Some(ConventionalCommitType::Feat),
            "fix" => Some(ConventionalCommitType::Fix),
            "docs" => Some(ConventionalCommitType::Docs),
            "style" => Some(ConventionalCommitType::Style),
            "refactor" => Some(ConventionalCommitType::Refactor),
            "perf" => Some(ConventionalCommitType::Perf),
            "test" => Some(ConventionalCommitType::Test),
            "build" => Some(ConventionalCommitType::Build),
            "ci" => Some(ConventionalCommitType::Ci),
            "chore" => Some(ConventionalCommitType::Chore),
            "revert" => Some(ConventionalCommitType::Revert),
            "init" => Some(ConventionalCommitType::Init),
            "config" => Some(ConventionalCommitType::Config),
            "release" => Some(ConventionalCommitType::Release),
            "deploy" => Some(ConventionalCommitType::Deploy),
            "merge" => Some(ConventionalCommitType::Merge),
            "wip" => Some(ConventionalCommitType::Wip),
            "typo" => Some(ConventionalCommitType::Typo),
            "locale" => Some(ConventionalCommitType::Locale),
            _ => None,
        }
    }
}

impl std::fmt::Display for ConventionalCommitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
#[path = "presentation_tests.rs"]
mod presentation_tests;
