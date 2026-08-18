//! # VibeWindow 环境标志模块
//!
//! 本模块提供 VibeWindow 运行时的环境变量配置和特性开关管理。
//!
//! ## 主要功能
//!
//! - **特性开关控制**：通过环境变量启用或禁用特定功能
//! - **配置路径管理**：指定配置文件和目录的自定义路径
//! - **实验性功能管理**：控制实验性特性的启用状态
//! - **性能调优**：设置超时时间、令牌限制等性能参数
//!
//! ## 设计原则
//!
//! 所有配置项均通过环境变量读取，使用 `Lazy` 实现惰性求值，确保：
//! - 环境变量只读取一次（首次访问时）
//! - 后续访问直接使用缓存值，避免重复解析
//!
//! ## 使用示例
//!
//! ```ignore
//! // 检查是否禁用自动更新
//! if *VIBEWINDOW_DISABLE_AUTOUPDATE {
//!     println!("自动更新已禁用");
//! }
//!
//! // 获取自定义配置路径
//! if let Some(config_path) = VIBEWINDOW_CONFIG.as_ref() {
//!     println!("使用配置文件: {}", config_path);
//! }
//! ```

use std::env;
use std::sync::LazyLock;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
mod tests;

/// 从环境变量中读取字符串值
///
/// 尝试获取指定环境变量的值，如果变量不存在或无法转换为有效字符串，则返回 `None`。
///
/// # 参数
///
/// - `key`: 环境变量名称
///
/// # 返回值
///
/// - `Some(String)`: 环境变量存在且可转换为字符串
/// - `None`: 环境变量不存在或转换失败
///
/// # 示例
///
/// ```ignore
/// let path = env_string("VIBEWINDOW_CONFIG");
/// // 如果环境变量 VIBEWINDOW_CONFIG="/etc/vibewindow/config.toml"
/// // 则返回 Some("/etc/vibewindow/config.toml")
/// ```
fn env_string(key: &str) -> Option<String> {
    env::var_os(key).map(|v| v.to_string_lossy().to_string())
}

/// 检查环境变量是否为真值
///
/// 判断指定环境变量的值是否表示"真"。支持的真值包括：
/// - `"true"`（不区分大小写）
/// - `"1"`
///
/// # 参数
///
/// - `key`: 环境变量名称
///
/// # 返回值
///
/// - `true`: 环境变量值为 "true" 或 "1"（不区分大小写）
/// - `false`: 环境变量不存在或值不是真值
///
/// # 示例
///
/// ```ignore
/// // 假设 VIBEWINDOW_AUTO_SHARE=true
/// if truthy("VIBEWINDOW_AUTO_SHARE") {
///     println!("自动分享已启用");
/// }
/// ```
fn truthy(key: &str) -> bool {
    let Some(value) = env_string(key) else {
        return false;
    };
    let value = value.to_ascii_lowercase();
    // 支持多种真值格式：布尔值 true 或数字 1
    value == "true" || value == "1"
}

/// 从环境变量中读取正整数
///
/// 尝试将环境变量的值解析为正整数（u64）。只有正整数才会被返回。
///
/// # 参数
///
/// - `key`: 环境变量名称
///
/// # 返回值
///
/// - `Some(u64)`: 环境变量存在且可解析为正整数
/// - `None`: 环境变量不存在、不是数字、或为非正整数
///
/// # 示例
///
/// ```ignore
/// // 假设 VIBEWINDOW_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS=30000
/// let timeout = number("VIBEWINDOW_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS");
/// // 返回 Some(30000)
/// ```
fn number(key: &str) -> Option<u64> {
    let value = env_string(key)?;
    let parsed = value.parse::<i64>().ok()?;
    // 只接受正整数，避免无效的超时配置
    if parsed > 0 { Some(parsed as u64) } else { None }
}

// ============================================================================
// 基础功能开关
// ============================================================================

/// 自动分享功能开关
///
/// 控制是否启用自动分享功能。当设置为 `true` 或 `1` 时启用。
pub static VIBEWINDOW_AUTO_SHARE: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_AUTO_SHARE"));

/// Git Bash 路径配置
///
/// 指定 Git Bash 的自定义安装路径，用于在 Windows 等系统上执行 Shell 命令。
pub static VIBEWINDOW_GIT_BASH_PATH: LazyLock<Option<String>> =
    LazyLock::new(|| env_string("VIBEWINDOW_GIT_BASH_PATH"));

/// 自定义配置文件路径
///
/// 指定 VibeWindow 配置文件的完整路径，覆盖默认配置路径。
pub static VIBEWINDOW_CONFIG: LazyLock<Option<String>> =
    LazyLock::new(|| env_string("VIBEWINDOW_CONFIG"));

/// 内联配置内容
///
/// 直接通过环境变量提供配置内容，而不是从文件读取。适用于容器化部署场景。
pub static VIBEWINDOW_CONFIG_CONTENT: LazyLock<Option<String>> =
    LazyLock::new(|| env_string("VIBEWINDOW_CONFIG_CONTENT"));

/// 禁用自动更新
///
/// 控制是否禁用 VibeWindow 的自动更新功能。
pub static VIBEWINDOW_DISABLE_AUTOUPDATE: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_DISABLE_AUTOUPDATE"));

/// 禁用自动清理
///
/// 控制是否禁用过期资源的自动清理（prune）功能。
pub static VIBEWINDOW_DISABLE_PRUNE: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_DISABLE_PRUNE"));

/// 禁用终端标题修改
///
/// 控制是否允许 VibeWindow 修改终端窗口标题。
pub static VIBEWINDOW_DISABLE_TERMINAL_TITLE: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_DISABLE_TERMINAL_TITLE"));

/// 权限配置
///
/// 指定自定义的权限策略配置，用于控制代理的操作权限范围。
pub static VIBEWINDOW_PERMISSION: LazyLock<Option<String>> =
    LazyLock::new(|| env_string("VIBEWINDOW_PERMISSION"));

// ============================================================================
// 插件和扩展管理
// ============================================================================

/// 禁用默认插件
///
/// 控制是否在启动时禁用所有默认插件的加载。
pub static VIBEWINDOW_DISABLE_DEFAULT_PLUGINS: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_DISABLE_DEFAULT_PLUGINS"));

/// 禁用 LSP 下载
///
/// 控制是否禁用语言服务器协议（LSP）相关组件的自动下载。
pub static VIBEWINDOW_DISABLE_LSP_DOWNLOAD: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_DISABLE_LSP_DOWNLOAD"));

/// 启用实验性模型
///
/// 控制是否启用实验性的 AI 模型支持。
pub static VIBEWINDOW_ENABLE_EXPERIMENTAL_MODELS: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_ENABLE_EXPERIMENTAL_MODELS"));

/// 禁用自动压缩
///
/// 控制是否禁用对话历史的自动压缩功能。
pub static VIBEWINDOW_DISABLE_AUTOCOMPACT: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_DISABLE_AUTOCOMPACT"));

// ============================================================================
// Claude Code 集成控制
// ============================================================================

/// 禁用 Claude Code 集成（主开关）
///
/// 完全禁用 Claude Code 相关的所有功能，包括提示和技能。
pub static VIBEWINDOW_DISABLE_CLAUDE_CODE: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_DISABLE_CLAUDE_CODE"));

/// 禁用 Claude Code 提示
///
/// 禁用 Claude Code 相关的提示功能。
/// 如果 `VIBEWINDOW_DISABLE_CLAUDE_CODE` 已启用，则此选项自动生效。
pub static VIBEWINDOW_DISABLE_CLAUDE_CODE_PROMPT: LazyLock<bool> = LazyLock::new(|| {
    // 优先检查主开关，其次检查提示专用开关
    *VIBEWINDOW_DISABLE_CLAUDE_CODE || truthy("VIBEWINDOW_DISABLE_CLAUDE_CODE_PROMPT")
});

/// 禁用 Claude Code 技能
///
/// 禁用 Claude Code 相关的技能加载和执行。
/// 如果 `VIBEWINDOW_DISABLE_CLAUDE_CODE` 已启用，则此选项自动生效。
pub static VIBEWINDOW_DISABLE_CLAUDE_CODE_SKILLS: LazyLock<bool> = LazyLock::new(|| {
    // 优先检查主开关，其次检查技能专用开关
    *VIBEWINDOW_DISABLE_CLAUDE_CODE || truthy("VIBEWINDOW_DISABLE_CLAUDE_CODE_SKILLS")
});

/// 禁用外部技能
///
/// 禁用所有外部技能的加载和执行。
/// 如果 `VIBEWINDOW_DISABLE_CLAUDE_CODE_SKILLS` 已启用，则此选项自动生效。
pub static VIBEWINDOW_DISABLE_EXTERNAL_SKILLS: LazyLock<bool> = LazyLock::new(|| {
    // 层级依赖：Claude Code 技能禁用时，外部技能也被禁用
    *VIBEWINDOW_DISABLE_CLAUDE_CODE_SKILLS || truthy("VIBEWINDOW_DISABLE_EXTERNAL_SKILLS")
});

// ============================================================================
// 调试和测试工具
// ============================================================================

/// 模拟版本控制系统
///
/// 用于测试目的，模拟 VCS（如 Git）的行为而不需要真实的版本库。
pub static VIBEWINDOW_FAKE_VCS: LazyLock<Option<String>> =
    LazyLock::new(|| env_string("VIBEWINDOW_FAKE_VCS"));

/// 服务器密码
///
/// 指定 VibeWindow 内置服务器的认证密码。
pub static VIBEWINDOW_SERVER_PASSWORD: LazyLock<Option<String>> =
    LazyLock::new(|| env_string("VIBEWINDOW_SERVER_PASSWORD"));

/// 服务器用户名
///
/// 指定 VibeWindow 内置服务器的认证用户名。
pub static VIBEWINDOW_SERVER_USERNAME: LazyLock<Option<String>> =
    LazyLock::new(|| env_string("VIBEWINDOW_SERVER_USERNAME"));

// ============================================================================
// 实验性功能（通用）
// ============================================================================

/// 实验性功能总开关
///
/// 启用所有实验性功能的基础开关。单独的实验性功能可能有各自独立的开关。
pub static VIBEWINDOW_EXPERIMENTAL: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_EXPERIMENTAL"));

/// 实验性文件监视器
///
/// 启用实验性的文件系统监视器实现。
pub static VIBEWINDOW_EXPERIMENTAL_FILEWATCHER: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_EXPERIMENTAL_FILEWATCHER"));

/// 禁用实验性文件监视器
///
/// 显式禁用实验性文件监视器，即使总开关已启用。
pub static VIBEWINDOW_EXPERIMENTAL_DISABLE_FILEWATCHER: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_EXPERIMENTAL_DISABLE_FILEWATCHER"));

/// 实验性图标发现
///
/// 启用实验性的应用程序图标发现功能。
/// 如果 `VIBEWINDOW_EXPERIMENTAL` 总开关已启用，则此功能自动启用。
pub static VIBEWINDOW_EXPERIMENTAL_ICON_DISCOVERY: LazyLock<bool> =
    LazyLock::new(|| *VIBEWINDOW_EXPERIMENTAL || truthy("VIBEWINDOW_EXPERIMENTAL_ICON_DISCOVERY"));

/// 禁用选择时自动复制
///
/// 控制是否禁用文本选择时自动复制到剪贴板的实验性功能。
pub static VIBEWINDOW_EXPERIMENTAL_DISABLE_COPY_ON_SELECT: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_EXPERIMENTAL_DISABLE_COPY_ON_SELECT"));

/// 启用 EXA 搜索
///
/// 启用 EXA 搜索引擎集成。可通过以下任一方式启用：
/// - 直接设置 `VIBEWINDOW_ENABLE_EXA`
/// - 启用实验性总开关 `VIBEWINDOW_EXPERIMENTAL`
/// - 设置 `VIBEWINDOW_EXPERIMENTAL_EXA`
pub static VIBEWINDOW_ENABLE_EXA: LazyLock<bool> = LazyLock::new(|| {
    // 多种方式可以启用 EXA 搜索功能
    truthy("VIBEWINDOW_ENABLE_EXA")
        || *VIBEWINDOW_EXPERIMENTAL
        || truthy("VIBEWINDOW_EXPERIMENTAL_EXA")
});

// ============================================================================
// 性能调优参数
// ============================================================================

/// Bash 命令默认超时时间（毫秒）
///
/// 设置 Bash 命令执行的默认超时时间，单位为毫秒。
/// 必须设置为正整数才有效。
pub static VIBEWINDOW_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS: LazyLock<Option<u64>> =
    LazyLock::new(|| number("VIBEWINDOW_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS"));

/// 输出令牌最大数量
///
/// 限制单次 AI 响应的最大输出令牌数量，用于控制成本和响应长度。
pub static VIBEWINDOW_EXPERIMENTAL_OUTPUT_TOKEN_MAX: LazyLock<Option<u64>> =
    LazyLock::new(|| number("VIBEWINDOW_EXPERIMENTAL_OUTPUT_TOKEN_MAX"));

// ============================================================================
// 实验性功能（高级）
// ============================================================================

/// 实验性输出格式化
///
/// 启用实验性的输出格式化功能。
/// 如果 `VIBEWINDOW_EXPERIMENTAL` 总开关已启用，则此功能自动启用。
pub static VIBEWINDOW_EXPERIMENTAL_OXFMT: LazyLock<bool> =
    LazyLock::new(|| *VIBEWINDOW_EXPERIMENTAL || truthy("VIBEWINDOW_EXPERIMENTAL_OXFMT"));

/// 实验性 LSP TypeScript 支持
///
/// 启用实验性的 LSP TypeScript 类型检查集成。
pub static VIBEWINDOW_EXPERIMENTAL_LSP_TY: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_EXPERIMENTAL_LSP_TY"));

/// 禁用文件时间检查
///
/// 控制是否禁用基于文件修改时间的缓存验证。
pub static VIBEWINDOW_DISABLE_FILETIME_CHECK: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_DISABLE_FILETIME_CHECK"));

/// 实验性 Markdown 渲染
///
/// 启用实验性的 Markdown 渲染引擎。
pub static VIBEWINDOW_EXPERIMENTAL_MARKDOWN: LazyLock<bool> =
    LazyLock::new(|| truthy("VIBEWINDOW_EXPERIMENTAL_MARKDOWN"));

// ============================================================================
// 模型配置
// ============================================================================

/// 自定义模型列表路径
///
/// 指定本地模型列表文件的路径，用于离线或自定义模型配置。
pub static VIBEWINDOW_MODELS_PATH: LazyLock<Option<String>> =
    LazyLock::new(|| env_string("VIBEWINDOW_MODELS_PATH"));

// ============================================================================
// 公共函数接口
// ============================================================================

/// 检查是否禁用项目级配置
///
/// 判断是否禁用从项目目录读取配置文件的功能。
/// 启用后，VibeWindow 将只使用全局配置。
///
/// # 返回值
///
/// - `true`: 禁用项目级配置
/// - `false`: 允许读取项目级配置（默认行为）
///
/// # 环境变量
///
/// 读取 `VIBEWINDOW_DISABLE_PROJECT_CONFIG` 环境变量
pub fn vibewindow_disable_project_config() -> bool {
    truthy("VIBEWINDOW_DISABLE_PROJECT_CONFIG")
}

/// 获取自定义配置目录
///
/// 获取 VibeWindow 配置文件的自定义目录路径。
/// 如果未设置，则使用默认的系统配置目录。
///
/// # 返回值
///
/// - `Some(String)`: 自定义配置目录路径
/// - `None`: 使用默认配置目录
///
/// # 环境变量
///
/// 读取 `VIBEWINDOW_CONFIG_DIR` 环境变量
pub fn vibewindow_config_dir() -> Option<String> {
    env_string("VIBEWINDOW_CONFIG_DIR")
}

/// 获取客户端类型标识
///
/// 获取当前 VibeWindow 客户端的类型标识，用于区分不同的运行环境
///（如 CLI、桌面应用、Web 界面等）。
///
/// # 返回值
///
/// 返回客户端类型字符串，默认为 `"cli"`
///
/// # 环境变量
///
/// 读取 `VIBEWINDOW_CLIENT` 环境变量
///
/// # 示例
///
/// ```ignore
/// let client = vibewindow_client();
/// match client.as_str() {
///     "cli" => println!("命令行界面"),
///     "desktop" => println!("桌面应用"),
///     "web" => println!("Web 界面"),
///     _ => println!("未知客户端: {}", client),
/// }
/// ```
pub fn vibewindow_client() -> String {
    env_string("VIBEWINDOW_CLIENT").unwrap_or_else(|| "cli".to_string())
}
