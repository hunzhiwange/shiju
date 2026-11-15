//! 浏览器后端类型定义模块
//!
//! 本模块提供浏览器自动化工具的后端类型系统，用于配置和解析不同的浏览器实现方案。
//!
//! # 主要功能
//!
//! - 定义用户可配置的浏览器后端类型（`BrowserBackendKind`）
//! - 定义已解析的浏览器后端类型（`ResolvedBackend`）
//! - 提供后端名称解析和错误消息生成工具函数
//!
//! # 后端类型
//!
//! - **AgentBrowser**: 基于 agent 的浏览器实现
//! - **RustNative**: 纯 Rust 原生实现的浏览器
//! - **ComputerUse**: 计算机使用（Computer Use）模式
//! - **Auto**: 自动选择最佳后端

/// 用户可配置的浏览器后端类型
///
/// 该枚举表示用户在配置中可以选择的浏览器后端类型，包括自动选择选项。
/// 通过 `parse` 方法可以从字符串解析，支持多种格式（如 kebab-case 和 snake_case）。
///
/// # 变体
///
/// - `AgentBrowser`: 基于 agent 的浏览器实现
/// - `RustNative`: 纯 Rust 原生实现的浏览器自动化
/// - `ComputerUse`: 计算机使用模式（通常用于 AI 代理直接控制计算机）
/// - `Auto`: 自动选择最佳可用的后端
///
/// # 示例
///
/// ```rust
/// use tool_browser::backend::BrowserBackendKind;
///
/// let backend = BrowserBackendKind::parse("native")?;
/// assert_eq!(backend, BrowserBackendKind::RustNative);
///
/// let backend = BrowserBackendKind::parse("agent-browser")?;
/// assert_eq!(backend, BrowserBackendKind::AgentBrowser);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserBackendKind {
    /// 基于 agent 的浏览器实现
    AgentBrowser,
    /// 纯 Rust 原生实现的浏览器自动化
    RustNative,
    /// 计算机使用模式（Computer Use）
    ComputerUse,
    /// 自动选择最佳可用的后端
    Auto,
}

/// 已解析的浏览器后端类型
///
/// 该枚举表示已经完成选择决策的具体浏览器后端类型。
/// 与 `BrowserBackendKind` 不同，该枚举不包含 `Auto` 选项，
/// 因为在实际使用前必须解析为具体的后端实现。
///
/// # 变体
///
/// - `AgentBrowser`: 基于 agent 的浏览器实现
/// - `RustNative`: 纯 Rust 原生实现的浏览器自动化
/// - `ComputerUse`: 计算机使用模式（Computer Use）
///
/// # 使用场景
///
/// 当配置中指定 `BrowserBackendKind::Auto` 时，运行时会根据系统环境
/// 和可用性选择一个具体的 `ResolvedBackend`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedBackend {
    /// 基于 agent 的浏览器实现
    AgentBrowser,
    /// 纯 Rust 原生实现的浏览器自动化
    RustNative,
    /// 计算机使用模式（Computer Use）
    ComputerUse,
}

impl BrowserBackendKind {
    /// 从字符串解析浏览器后端类型
    ///
    /// 该方法支持多种输入格式，会自动处理大小写、连字符和下划线。
    ///
    /// # 支持的格式
    ///
    /// - `"agent_browser"`, `"agentbrowser"`, `"agent-browser"` → `AgentBrowser`
    /// - `"rust_native"`, `"native"` → `RustNative`
    /// - `"computer_use"`, `"computeruse"` → `ComputerUse`
    /// - `"auto"` → `Auto`
    ///
    /// # 参数
    ///
    /// * `raw` - 原始输入字符串，可以包含不同的大小写和分隔符格式
    ///
    /// # 返回值
    ///
    /// - `Ok(BrowserBackendKind)` - 成功解析的后端类型
    /// - `Err` - 输入字符串无法识别为有效的后端类型
    ///
    /// # 示例
    ///
    /// ```rust
    /// use tool_browser::backend::BrowserBackendKind;
    ///
    /// // 支持不同格式
    /// assert_eq!(BrowserBackendKind::parse("native")?, BrowserBackendKind::RustNative);
    /// assert_eq!(BrowserBackendKind::parse("Native")?, BrowserBackendKind::RustNative);
    /// assert_eq!(BrowserBackendKind::parse("rust_native")?, BrowserBackendKind::RustNative);
    /// assert_eq!(BrowserBackendKind::parse("Rust-Native")?, BrowserBackendKind::RustNative);
    ///
    /// // 无效输入会返回错误
    /// assert!(BrowserBackendKind::parse("invalid").is_err());
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        // 标准化输入：去除空白、转换为小写、统一使用下划线
        let key = raw.trim().to_ascii_lowercase().replace('-', "_");
        match key.as_str() {
            "agent_browser" | "agentbrowser" => Ok(Self::AgentBrowser),
            "rust_native" | "native" => Ok(Self::RustNative),
            "computer_use" | "computeruse" => Ok(Self::ComputerUse),
            "auto" => Ok(Self::Auto),
            _ => anyhow::bail!(
                "Unsupported browser backend '{raw}'. Use 'agent_browser', 'rust_native', 'computer_use', or 'auto'"
            ),
        }
    }

    /// 获取后端类型的标准字符串表示
    ///
    /// 返回用于配置文件和日志输出的标准化名称（snake_case 格式）。
    ///
    /// # 返回值
    ///
    /// 返回静态字符串引用，表示该后端类型的标准名称：
    /// - `AgentBrowser` → `"agent_browser"`
    /// - `RustNative` → `"rust_native"`
    /// - `ComputerUse` → `"computer_use"`
    /// - `Auto` → `"auto"`
    ///
    /// # 示例
    ///
    /// ```rust
    /// use tool_browser::backend::BrowserBackendKind;
    ///
    /// assert_eq!(BrowserBackendKind::RustNative.as_str(), "rust_native");
    /// assert_eq!(BrowserBackendKind::Auto.as_str(), "auto");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AgentBrowser => "agent_browser",
            Self::RustNative => "rust_native",
            Self::ComputerUse => "computer_use",
            Self::Auto => "auto",
        }
    }
}

/// 获取已解析后端的标准名称字符串
///
/// 将 `ResolvedBackend` 转换为标准化的字符串表示，用于日志、错误消息和配置输出。
///
/// # 参数
///
/// * `backend` - 已解析的浏览器后端类型
///
/// # 返回值
///
/// 返回静态字符串引用，表示该后端的标准名称：
/// - `ResolvedBackend::AgentBrowser` → `"agent_browser"`
/// - `ResolvedBackend::RustNative` → `"rust_native"`
/// - `ResolvedBackend::ComputerUse` → `"computer_use"`
///
/// # 示例
///
/// ```rust
/// use tool_browser::backend::{ResolvedBackend, backend_name};
///
/// assert_eq!(backend_name(ResolvedBackend::RustNative), "rust_native");
/// assert_eq!(backend_name(ResolvedBackend::AgentBrowser), "agent_browser");
/// ```
pub fn backend_name(backend: ResolvedBackend) -> &'static str {
    match backend {
        ResolvedBackend::AgentBrowser => "agent_browser",
        ResolvedBackend::RustNative => "rust_native",
        ResolvedBackend::ComputerUse => "computer_use",
    }
}

/// 生成操作不可用的错误消息
///
/// 当某个浏览器操作在特定后端上不可用时，使用此函数生成标准化的错误消息。
///
/// # 参数
///
/// * `action` - 不可用的操作名称（如 "click", "screenshot" 等）
/// * `backend` - 当前使用的浏览器后端类型
///
/// # 返回值
///
/// 返回格式化的错误消息字符串，格式为：`"Action '{action}' is unavailable for backend '{backend_name}'"`
///
/// # 示例
///
/// ```rust
/// use tool_browser::backend::{ResolvedBackend, unavailable_action_for_backend_error};
///
/// let error_msg = unavailable_action_for_backend_error("screenshot", ResolvedBackend::AgentBrowser);
/// assert_eq!(error_msg, "Action 'screenshot' is unavailable for backend 'agent_browser'");
/// ```
pub fn unavailable_action_for_backend_error(action: &str, backend: ResolvedBackend) -> String {
    format!("Action '{action}' is unavailable for backend '{}'", backend_name(backend))
}
#[cfg(test)]
#[path = "backend_tests.rs"]
mod backend_tests;
