//! 浏览器自动化工具
//!
//! 支持多种后端的浏览器自动化工具。默认使用 Vercel 的 agent-browser CLI，
//! 也可通过配置启用 Rust 原生后端。支持 Computer-Use 级别的操作系统操作。
//!
//! ## 架构设计
//!
//! 本模块采用可插拔后端架构，允许根据不同场景选择最适合的浏览器自动化方案：
//! - **agent-browser 后端**（默认）：使用 Vercel 提供的 agent-browser CLI 工具
//! - **Rust 原生后端**（可选）：通过 `--features browser-native` 编译特性启用
//!
//! ## 核心功能
//!
//! - 浏览器操作自动化（导航、点击、输入等）
//! - Computer-Use 操作（操作系统级别的鼠标、键盘控制）
//! - 多会话管理与状态隔离
//! - 安全策略与域名白名单控制
//!
//! ## 模块组织
//!
//! - `actions`: 浏览器动作定义与解析
//! - `agent_browser`: agent-browser CLI 后端实现
//! - `backend`: 后端抽象与选择逻辑
//! - `computer_use`: Computer-Use 操作客户端
//! - `helpers`: 辅助函数（主机验证、域名规范化等）
//! - `tool`: 工具 trait 实现
//! - `native_backend`: Rust 原生浏览器后端（可选特性）

mod actions;
mod agent_browser;
mod backend;
mod computer_use;
mod helpers;
pub(crate) mod installation;
mod tool;

#[cfg(feature = "browser-native")]
mod native_backend;

use crate::app::agent::security::SecurityPolicy;
use std::sync::Arc;

pub use computer_use::ComputerUseConfig;

/// 为测试创建 Computer-Use 客户端
///
/// 这是一个仅在测试模块中可用的辅助函数，用于从 BrowserTool 实例
/// 提取并创建 ComputerUseClient，方便在测试中验证 Computer-Use 功能。
///
/// # 参数
///
/// * `tool` - BrowserTool 工具实例的引用
///
/// # 返回值
///
/// 返回一个新创建的 `ComputerUseClient` 实例，配置与传入的工具相同
///
/// # 使用场景
///
/// 主要用于单元测试和集成测试中，验证 Computer-Use 客户端的初始化
/// 和配置传递是否正确。
#[cfg(test)]
pub(crate) fn computer_use_client_for_tests(tool: &BrowserTool) -> computer_use::ComputerUseClient {
    computer_use::ComputerUseClient::new(
        tool.security.clone(),
        tool.allowed_domains.clone(),
        tool.session_name.clone(),
        tool.computer_use.clone(),
    )
}

/// 浏览器自动化工具
///
/// 使用可插拔后端架构的浏览器自动化工具实现。支持多种浏览器操作，
/// 包括页面导航、元素交互、截图等，以及 Computer-Use 级别的操作系统操作。
///
/// # 架构说明
///
/// 工具采用策略模式，根据配置选择不同的后端实现：
/// - **agent-browser**: 基于 CLI 的默认后端，适合大多数场景
/// - **native**: Rust 原生后端，提供更好的性能和集成度（需启用 `browser-native` 特性）
///
/// # 字段说明
///
/// * `security` - 安全策略，控制浏览器操作的安全边界
/// * `allowed_domains` - 允许访问的域名白名单
/// * `session_name` - 可选的会话名称，用于多会话隔离
/// * `backend` - 后端类型标识（"agent-browser" 或 "native"）
/// * `native_headless` - 原生后端是否使用无头模式
/// * `native_webdriver_url` - WebDriver 服务 URL（原生后端使用）
/// * `native_chrome_path` - Chrome 可执行文件路径（可选）
/// * `computer_use` - Computer-Use 配置，用于操作系统级别操作
/// * `native_state` - 原生后端的状态管理（仅 `browser-native` 特性）
///
/// # 安全性
///
/// - 所有网络请求受 `security` 策略和 `allowed_domains` 约束
/// - Computer-Use 操作具有独立的安全控制机制
/// - 私有网络地址访问需要显式配置
///
/// # 示例
///
/// ```ignore
/// use std::sync::Arc;
/// use crate::app::agent::security::SecurityPolicy;
/// use crate::app::agent::tools::browser::{BrowserTool, BrowserBackendKind};
///
/// let security = Arc::new(SecurityPolicy::default());
/// let tool = BrowserTool::new(
///     security,
///     vec!["example.com".to_string()],
///     None,
///     "agent-browser".to_string(),
///     true,
///     "http://localhost:4444".to_string(),
///     None,
///     ComputerUseConfig::default(),
/// );
/// ```
pub struct BrowserTool {
    security: Arc<SecurityPolicy>,
    allowed_domains: Vec<String>,
    session_name: Option<String>,
    backend: String,
    native_headless: bool,
    native_webdriver_url: String,
    native_chrome_path: Option<String>,
    computer_use: ComputerUseConfig,
    #[cfg(feature = "browser-native")]
    native_state: tokio::sync::Mutex<native_backend::NativeBrowserState>,
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
