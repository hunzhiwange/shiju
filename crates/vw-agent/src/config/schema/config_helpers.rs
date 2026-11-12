//! 配置辅助工具模块
//!
//! 本模块提供配置管理操作的实用工具函数，包括错误消息生成和从外部源安全获取 API 密钥。
//!
//! # 主要功能
//!
//! - [`config_dir_creation_error`]: 为目录创建失败生成用户友好的错误消息
//! - [`read_codex_openai_api_key`]: 从 Codex 认证文件中读取 OpenAI API 密钥
//!
//! # 安全考虑
//!
//! 本模块在处理敏感信息（如 API 密钥）时遵循安全最佳实践：
//! - 不会在日志或错误消息中泄露密钥内容
//! - 读取失败时静默返回 None，避免暴露系统信息
//! - 对密钥进行验证和清理（去除空白字符、检查非空）

// 导入路径处理的标准库
use std::path::Path;

/// 为配置目录创建失败生成描述性错误消息
///
/// 创建包含失败路径的用户友好错误消息，并为服务部署场景（特别是 OpenRC 服务设置）提供指导。
///
/// # 参数
///
/// * `path` - 目录创建失败的路径
///
/// # 返回值
///
/// 返回格式化的错误消息字符串，包含：
/// - 失败的目录路径
/// - 针对 OpenRC 用户的服务部署指导
///
/// # 示例
///
/// ```
/// use std::path::Path;
/// use vibewindow::app::agent::config::schema::config_helpers::config_dir_creation_error;
///
/// let path = Path::new("/etc/vibewindow/config");
/// let error_msg = config_dir_creation_error(path);
/// assert!(error_msg.contains("/etc/vibewindow/config"));
/// assert!(error_msg.contains("OpenRC"));
/// ```
///
/// # 使用场景
///
/// 当配置系统尝试创建必要的目录结构失败时，应使用此函数生成错误消息。
/// 这在以下情况下特别有用：
/// - 首次运行时初始化配置目录
/// - 在系统服务环境中运行时（权限可能受限）
/// - 用户手动指定了非标准配置路径
pub(crate) fn config_dir_creation_error(path: &Path) -> String {
    // 格式化错误消息，包含失败路径和服务部署建议
    // 注意：错误消息使用英文，因为这是面向系统管理员的错误信息
    // 提及 OpenRC 是因为它是一个常见的 init 系统，在容器和服务器环境中广泛使用
    format!(
        "Failed to create config directory: {}. If running as an OpenRC service, \
         ensure this path is writable by user 'vibewindow'.",
        path.display()
    )
}

/// 从 Codex 认证文件读取 OpenAI API 密钥
///
/// 尝试从 `~/.codex/auth.json` 获取 OpenAI API 密钥，这是 Codex CLI 存储认证凭证的标准位置。
///
/// # 安全考虑
///
/// - 从用户主目录读取，遵循系统约定
/// - 验证密钥非空且经过适当修剪
/// - 任何错误时返回 `None`（文件未找到、解析错误、密钥缺失）以实现安全回退
/// - 不会在错误消息中泄露密钥内容或系统路径信息
///
/// # 返回值
///
/// * `Some(String)` - 如果找到并验证通过，返回修剪后的非空 API 密钥
/// * `None` - 如果文件不存在、格式错误或密钥缺失/为空
///
/// # 预期文件格式
///
/// 函数期望 `~/.codex/auth.json` 包含带有 `OPENAI_API_KEY` 字段的有效 JSON：
///
/// ```json
/// {
///   "OPENAI_API_KEY": "sk-..."
/// }
/// ```
///
/// # 错误处理
///
/// 此函数在以下情况下静默失败并返回 `None`：
/// - 无法确定用户主目录
/// - `~/.codex/auth.json` 文件不存在或不可读
/// - JSON 解析失败
/// - `OPENAI_API_KEY` 字段缺失或不是字符串
/// - API 密钥为空或仅包含空白字符
///
/// # 示例
///
/// ```no_run
/// use vibewindow::app::agent::config::schema::config_helpers::read_codex_openai_api_key;
///
/// match read_codex_openai_api_key() {
///     Some(key) => println!("找到 API 密钥：{} 个字符", key.len()),
///     None => println!("无可用 API 密钥"),
/// }
/// ```
///
/// # 实现细节
///
/// 函数按顺序执行以下操作：
/// 1. 获取运行时配置主目录
/// 2. 构建 Codex 认证文件路径
/// 3. 读取文件内容
/// 4. 解析 JSON
/// 5. 提取并验证 API 密钥
/// 6. 对密钥进行修剪并检查非空
///
/// 每个步骤使用 `?` 操作符进行短路求值，确保在任一步骤失败时优雅地返回 `None`。
pub(crate) fn read_codex_openai_api_key() -> Option<String> {
    let home = crate::config::runtime_home::config_home().ok()?;

    // 构建 Codex 的 auth.json 文件路径
    let auth_path = home.join(".codex").join("auth.json");

    // 读取文件内容；如果不可读则静默失败
    let raw = std::fs::read_to_string(auth_path).ok()?;

    // 解析 JSON；如果格式错误则静默失败
    let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;

    // 提取 OPENAI_API_KEY 字段，验证并返回
    // 链式调用说明：
    // 1. get("OPENAI_API_KEY") - 从 JSON 中获取指定字段
    // 2. and_then(as_str) - 尝试将值转换为字符串切片
    // 3. map(trim) - 去除字符串首尾空白字符
    // 4. filter(!is_empty) - 只保留非空字符串
    // 5. map(to_string) - 将 &str 转换为 String
    parsed
        .get("OPENAI_API_KEY")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}
