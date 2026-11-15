//! Integration HTTP handlers; the host supplies existing authentication and settings services.
#![cfg(not(target_arch = "wasm32"))]
pub mod handlers;

use agent_config::runtime::Config;
use axum::{Json, http::{HeaderMap, StatusCode}};
use std::collections::BTreeMap;

/// Existing host services used by the integration routes.
pub trait IntegrationsHost: Send + Sync + 'static {
    type State: Clone + Send + Sync + 'static;
    type Settings: serde::Serialize;
    fn config(state: &Self::State) -> &parking_lot::Mutex<Config>;
    fn require_auth(state: &Self::State, headers: &HeaderMap)
        -> Result<(), (StatusCode, Json<serde_json::Value>)>;
    fn build_integration_settings_payload(config: &Config) -> Self::Settings;
    fn config_revision(config: &Config) -> String;
    fn apply_integration_credentials_update(config: &Config, id: &str,
        fields: &BTreeMap<String, String>) -> Result<Config, String>;
}

/// 集成凭据更新请求体
///
/// 用于更新特定集成（如第三方服务）的凭据字段，支持乐观并发控制。
///
/// # 并发控制
///
/// 使用 `revision` 字段实现乐观并发控制，确保在多个并发更新时
/// 不会意外覆盖其他客户端的修改。
///
/// # 示例
///
/// ```json
/// {
///     "revision": "v3",
///     "fields": {
///         "api_key": "sk-xxxx",
///         "endpoint": "https://api.example.com"
///     }
/// }
/// ```
#[derive(serde::Deserialize)]
pub struct IntegrationCredentialsUpdateBody {
    /// 当前配置的修订版本标识
    ///
    /// 可选参数，用于乐观并发控制。
    /// 如果服务器上的当前版本与此不匹配，更新可能会被拒绝。
    pub revision: Option<String>,

    /// 要更新的凭据字段键值对
    ///
    /// 键为字段名称，值为新的凭据值。
    /// 使用 `BTreeMap` 确保字段的序列化顺序稳定。
    /// 默认为空 map，可通过 `#[serde(default)]` 处理缺失字段。
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
}
