use crate::ui_types::Screen;
/// 项目工作区的界面呈现模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkspaceMode {
    /// 完整代码工作区。
    Code,
    /// 聚焦会话的简洁工作区。
    #[default]
    Work,
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

    /// 从可选配置读取模式；缺失或非法值使用默认的工作模式。
    pub fn from_config(value: Option<&str>) -> Self {
        value.and_then(Self::parse).unwrap_or_default()
    }
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
    pub fn from_gateway_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "macos" | "darwin" | "mac" => Some(Self::MacOs),
            "windows" | "win32" => Some(Self::Windows),
            "linux" => Some(Self::Linux),
            _ => None,
        }
    }

    pub fn current_target() -> Option<Self> {
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

    pub const fn file_manager_label(self) -> &'static str {
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

impl WorkspaceMode {
    pub fn chat_fullscreen(self, code: bool, work: bool) -> bool {
        match self { Self::Code => code, Self::Work => work }
    }
}
