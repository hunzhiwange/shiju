//! Provider 认证模块
//!
//! 本模块提供 Provider 的认证功能，支持多种认证方式：
//! - OAuth 授权流程
//! - API 密钥认证
//!
//! 主要功能：
//! - 查询可用的认证方法
//! - 发起授权请求
//! - 处理 OAuth 回调
//! - 设置 API 密钥
//!
//! # 认证流程
//!
//! 1. 通过 `methods()` 查询 provider 支持的认证方式
//! 2. 通过 `authorize()` 发起授权请求，获取授权信息
//! 3. 用户完成授权后，通过 `callback()` 处理回调
//! 4. 或者直接通过 `api()` 设置 API 密钥

use agent_auth as auth;
use serde::{Deserialize, Serialize};

/// 认证方法枚举
///
/// 定义 Provider 支持的认证方式，目前支持：
/// - OAuth：基于 OAuth 协议的授权流程
/// - Api：基于 API 密钥的简单认证
///
/// # 序列化格式
///
/// 使用 `type` 字段作为标签进行序列化，字段名转换为小写：
/// ```json
/// {"type": "oauth", "label": "GitHub OAuth"}
/// {"type": "api", "label": "API Key"}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Method {
    /// OAuth 授权方式
    ///
    /// # 字段
    /// - `label`: 认证方法的显示标签，用于 UI 展示
    Oauth { label: String },

    /// API 密钥认证方式
    ///
    /// # 字段
    /// - `label`: 认证方法的显示标签，用于 UI 展示
    Api { label: String },
}

/// 授权信息结构体
///
/// 包含发起授权请求所需的信息，用于引导用户完成授权流程。
///
/// # 字段
///
/// - `url`: 授权页面的 URL，用户需要访问此 URL 完成授权
/// - `method`: 授权方法名称，用于展示给用户
/// - `instructions`: 授权说明文本，指导用户如何完成授权
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authorization {
    /// 授权页面的 URL
    pub url: String,
    /// 授权方法名称
    pub method: String,
    /// 授权说明文本
    pub instructions: String,
}

/// 认证错误枚举
///
/// 定义认证过程中可能发生的各种错误情况。
#[derive(Debug)]
pub enum Error {
    /// 不支持的认证方式
    ///
    /// 当请求的认证方式不在支持列表中时返回此错误
    Unsupported,

    /// 未找到待处理的 OAuth 授权
    ///
    /// 当尝试处理回调时，没有找到对应的授权会话时返回此错误
    MissingOauth,

    /// 缺少 OAuth code
    ///
    /// OAuth 回调时缺少必需的 authorization code 参数
    MissingCode,

    /// OAuth 回调失败
    ///
    /// 处理 OAuth 回调时发生错误
    CallbackFailed,

    /// IO 错误
    ///
    /// 文件系统或网络 IO 操作失败时返回此错误
    Io(std::io::Error),
}

/// 实现 Error 的显示格式
///
/// 为每种错误提供用户友好的中文错误信息
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unsupported => write!(f, "unsupported auth method"),
            Error::MissingOauth => write!(f, "missing oauth authorization"),
            Error::MissingCode => write!(f, "missing code"),
            Error::CallbackFailed => write!(f, "oauth callback failed"),
            Error::Io(e) => write!(f, "{}", e),
        }
    }
}

/// 实现 Error trait
///
/// 使 Error 可以作为标准错误类型使用
impl std::error::Error for Error {}

/// 从 std::io::Error 转换为认证错误
///
/// 允许将 IO 错误自动转换为认证错误
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

/// 查询所有 Provider 的可用认证方法
///
/// 返回一个 HashMap，其中键为 Provider ID，值为该 Provider 支持的认证方法列表。
///
/// # 返回值
///
/// 返回 `HashMap<String, Vec<Method>>`，其中：
/// - 键：Provider 的唯一标识符
/// - 值：该 Provider 支持的认证方法列表
///
/// # 示例
///
/// ```rust,no_run
/// use vibe_agent::app::agent::provider::auth;
///
/// async fn example() {
///     let methods = auth::methods().await;
///     // methods 是一个 HashMap，包含每个 provider 支持的认证方式
///     for (provider_id, auth_methods) in methods {
///         println!("Provider: {}", provider_id);
///         for method in auth_methods {
///             println!("  - {:?}", method);
///         }
///     }
/// }
/// ```
///
/// # 注意
///
/// 当前实现返回空的 HashMap，实际功能可能在后续版本中实现
pub async fn methods() -> std::collections::HashMap<String, Vec<Method>> {
    std::collections::HashMap::new()
}

/// 发起授权请求
///
/// 为指定的 Provider 和认证方法发起授权请求，返回授权信息。
///
/// # 参数
///
/// - `provider_id`: Provider 的唯一标识符
/// - `method`: 认证方法的索引，对应 `methods()` 返回列表中的位置
///
/// # 返回值
///
/// 返回 `Result<Option<Authorization>, Error>`：
/// - `Ok(Some(Authorization))`: 授权请求成功，包含授权信息
/// - `Ok(None)`: 该 provider 不需要授权或授权方法不存在
/// - `Err(Error)`: 授权请求失败
///
/// # 错误
///
/// 可能返回以下错误：
/// - `Error::Unsupported`: 不支持的认证方式
/// - `Error::Io`: IO 操作失败
///
/// # 示例
///
/// ```rust,no_run
/// use vibe_agent::app::agent::provider::auth;
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     // 发起第一个认证方法的授权
///     if let Some(auth_info) = auth::authorize("github", 0).await? {
///         println!("请访问: {}", auth_info.url);
///         println!("说明: {}", auth_info.instructions);
///     }
///     Ok(())
/// }
/// ```
///
/// # 注意
///
/// 当前实现始终返回 `Ok(None)`，实际功能可能在后续版本中实现
pub async fn authorize(_provider_id: &str, _method: usize) -> Result<Option<Authorization>, Error> {
    Ok(None)
}

/// 处理 OAuth 回调
///
/// 当用户完成 OAuth 授权后，OAuth 提供商会重定向到回调 URL，
/// 此函数用于处理该回调并完成认证流程。
///
/// # 参数
///
/// - `provider_id`: Provider 的唯一标识符
/// - `method`: 认证方法的索引，对应 `methods()` 返回列表中的位置
/// - `code`: OAuth 提供商返回的 authorization code，可选
///
/// # 返回值
///
/// 返回 `Result<(), Error>`：
/// - `Ok(())`: 回调处理成功，认证完成
/// - `Err(Error)`: 回调处理失败
///
/// # 错误
///
/// 可能返回以下错误：
/// - `Error::Unsupported`: 不支持的认证方式
/// - `Error::MissingOauth`: 未找到对应的授权会话
/// - `Error::MissingCode`: 缺少必需的 authorization code
/// - `Error::CallbackFailed`: 回调处理失败
///
/// # 示例
///
/// ```rust,no_run
/// use vibe_agent::app::agent::provider::auth;
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     // 处理 OAuth 回调，code 从回调 URL 中获取
///     let code = "abc123def456";
///     auth::callback("github", 0, Some(code)).await?;
///     println!("认证成功！");
///     Ok(())
/// }
/// ```
///
/// # 注意
///
/// 当前实现始终返回 `Error::Unsupported`，实际功能可能在后续版本中实现
pub async fn callback(
    _provider_id: &str,
    _method: usize,
    _code: Option<&str>,
) -> Result<(), Error> {
    Err(Error::Unsupported)
}

/// 设置 Provider 的 API 密钥
///
/// 为指定的 Provider 设置 API 密钥，用于基于密钥的认证方式。
/// 密钥会被安全地存储在系统的认证存储中。
///
/// # 参数
///
/// - `provider_id`: Provider 的唯一标识符
/// - `key`: API 密钥字符串
///
/// # 返回值
///
/// 返回 `Result<(), Error>`：
/// - `Ok(())`: API 密钥设置成功
/// - `Err(Error)`: 设置失败
///
/// # 错误
///
/// 主要可能返回 `Error::Io`，当密钥存储操作失败时。
///
/// # 示例
///
/// ```rust,no_run
/// use vibe_agent::app::agent::provider::auth;
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     // 设置 GitHub 的 API 密钥
///     let api_key = "ghp_xxxxxxxxxxxxxxxxxxxx";
///     auth::api("github", api_key).await?;
///     println!("API 密钥设置成功！");
///     Ok(())
/// }
/// ```
///
/// # 安全性
///
/// API 密钥会被存储在系统的安全存储中，请确保：
/// - 不要在日志或输出中打印密钥
/// - 使用强密钥并定期更换
/// - 不要将密钥提交到版本控制系统
pub async fn api(provider_id: &str, key: &str) -> Result<(), Error> {
    // 调用底层认证模块设置 API 密钥信息
    // 将密钥包装为 ApiInfo 结构体并存储
    auth::set(provider_id, &auth::Info::Api(auth::ApiInfo { key: key.to_string() }))?;
    Ok(())
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod auth_tests;
