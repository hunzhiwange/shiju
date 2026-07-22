//! 认证信息访问与 OAuth 服务入口。
//!
//! 本模块封装本地认证存储路径解析，并把共享认证模型暴露给 agent
//! 运行时。公开函数只读写认证仓库，不记录或打印令牌内容。

/// OpenAI OAuth 相关流程。
pub mod openai_oauth;
#[cfg(test)]
#[path = "openai_oauth_tests.rs"]
mod openai_oauth_tests;

pub use vw_shared::auth::{ApiInfo, Info, OAUTH_DUMMY_KEY, OauthInfo, WellKnownInfo};

use crate::app::agent::global;
use std::collections::HashMap;
use std::path::PathBuf;

/// 解析认证信息文件路径。
///
/// 路径由全局 home/data 目录派生，保持认证文件和运行时状态目录一致。
fn filepath() -> PathBuf {
    let paths = global::paths();
    vw_shared::auth::store::resolve_filepath(&paths.home, &paths.data)
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;

/// 读取指定 provider 的认证信息。
///
/// # 参数
///
/// - `provider_id`: provider 的稳定标识，例如模型供应商或集成名称。
///
/// # 返回值
///
/// 找到时返回认证信息；不存在或读取失败时返回 `None`，具体语义由共享
/// 存储层保持兼容。
pub fn get(provider_id: &str) -> Option<Info> {
    vw_shared::auth::store::get_from(&filepath(), provider_id)
}

/// 读取全部认证信息。
///
/// # 返回值
///
/// 返回以 provider id 为 key 的认证信息映射。该函数不暴露额外权限，
/// 只读取共享认证存储层已经保存的数据。
pub fn all() -> HashMap<String, Info> {
    vw_shared::auth::store::all_from(&filepath())
}

/// 写入指定 provider 的认证信息。
///
/// # 参数
///
/// - `key`: provider 的稳定标识。
/// - `info`: 要保存的认证信息。
///
/// # 错误处理
///
/// 文件创建、写入或序列化失败时返回底层 `std::io::Error`。
pub fn set(key: &str, info: &Info) -> Result<(), std::io::Error> {
    vw_shared::auth::store::set_to(&filepath(), key, info)
}

/// 删除指定 provider 的认证信息。
///
/// # 参数
///
/// - `key`: provider 的稳定标识。
///
/// # 错误处理
///
/// 删除或写回认证存储失败时返回底层 `std::io::Error`。
pub fn remove(key: &str) -> Result<(), std::io::Error> {
    vw_shared::auth::store::remove_from(&filepath(), key)
}

/// 认证服务门面。
///
/// 该类型预留状态目录和密钥加密开关，用于统一承接不同 provider 的
/// OAuth token 获取流程。字段保持私有，避免调用方直接接触敏感状态。
pub struct AuthService {
    state_dir: PathBuf,
    secrets_encrypt: bool,
}

impl AuthService {
    /// 使用显式状态目录创建认证服务。
    ///
    /// # 参数
    ///
    /// - `state_dir`: OAuth 状态和凭据缓存所在目录。
    /// - `secrets_encrypt`: 是否启用密钥加密存储。
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `AuthService` 实例。
    pub fn new(state_dir: &std::path::Path, secrets_encrypt: bool) -> Self {
        Self { state_dir: state_dir.to_path_buf(), secrets_encrypt }
    }

    /// 从应用配置创建认证服务。
    ///
    /// # 参数
    ///
    /// - `_config`: 应用配置；当前兼容实现暂不读取其中字段。
    ///
    /// # 返回值
    ///
    /// 返回默认认证服务实例。
    pub fn from_config(_config: &crate::app::agent::config::Config) -> Self {
        Self { state_dir: std::path::PathBuf::new(), secrets_encrypt: false }
    }

    /// 获取可用的 Gemini access token。
    ///
    /// # 参数
    ///
    /// - `_profile_override`: 可选 profile 覆盖名。
    ///
    /// # 返回值
    ///
    /// 成功时返回可用 token；当前实现未接入 Gemini OAuth，固定返回 `None`。
    ///
    /// # 错误处理
    ///
    /// 未来接入刷新流程时，配置、刷新或解密失败会通过 `anyhow::Error` 返回。
    pub async fn get_valid_gemini_access_token(
        &self,
        _profile_override: Option<&str>,
    ) -> anyhow::Result<Option<String>> {
        Ok(None)
    }

    /// 获取可用的 OpenAI access token。
    ///
    /// # 参数
    ///
    /// - `_profile_override`: 可选 profile 覆盖名。
    ///
    /// # 返回值
    ///
    /// 成功时返回可用 token；当前实现未启用刷新逻辑，固定返回 `None`。
    ///
    /// # 错误处理
    ///
    /// 未来刷新或存储读取失败时通过 `anyhow::Error` 返回，避免静默吞掉
    /// 认证问题。
    pub async fn get_valid_openai_access_token(
        &self,
        _profile_override: Option<&str>,
    ) -> anyhow::Result<Option<String>> {
        Ok(None)
    }

    /// 获取 Gemini OAuth profile。
    ///
    /// # 参数
    ///
    /// - `_profile_override`: 可选 profile 覆盖名。
    ///
    /// # 返回值
    ///
    /// 当前实现未接入 Gemini profile，固定返回 `None`。
    ///
    /// # 错误处理
    ///
    /// 未来读取或解析 profile 失败时通过 `anyhow::Error` 返回。
    pub async fn get_gemini_profile(
        &self,
        _profile_override: Option<&str>,
    ) -> anyhow::Result<Option<()>> {
        Ok(None)
    }

    /// 获取指定 provider 的 OAuth profile。
    ///
    /// # 参数
    ///
    /// - `_provider`: provider 标识。
    /// - `_profile_override`: 可选 profile 覆盖名。
    ///
    /// # 返回值
    ///
    /// 当前兼容实现固定返回 `None`。
    ///
    /// # 错误处理
    ///
    /// 未来 profile 读取、解密或校验失败时通过 `anyhow::Error` 返回。
    pub async fn get_profile(
        &self,
        _provider: &str,
        _profile_override: Option<&str>,
    ) -> anyhow::Result<Option<OauthInfo>> {
        Ok(None)
    }
}
