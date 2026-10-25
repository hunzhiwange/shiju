//! 当前用户连接器账号绑定与授权契约；敏感载荷不参与调试输出。
use super::{GatewayClient, ShijuCredential};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuConnectorVariable {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuConnectorBinding {
    pub third_party_account_id: String,
    pub third_party_account_name: String,
    #[serde(default)]
    pub binding_config: Map<String, Value>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuConnector {
    pub platform_id: u64,
    pub platform_code: String,
    pub platform_name: String,
    pub company_platform_id: u64,
    pub authorization_available: bool,
    #[serde(default, deserialize_with = "nullable_variables")]
    pub account_binding_variables: Vec<ShijuConnectorVariable>,
    pub binding: Option<ShijuConnectorBinding>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShijuConnectorSaveRequest {
    pub company_platform_id: u64,
    pub third_party_account_id: String,
    pub third_party_account_name: String,
    pub binding_config: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShijuConnectorAuthorizeRequest {
    pub company_platform_id: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ShijuConnectorChallenge {
    pub flow_id: String,
    pub authorization_url: String,
    pub poll_interval_seconds: u64,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShijuConnectorAuthorizationStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShijuConnectorPollRequest {
    pub flow_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShijuConnectorPoll {
    pub status: ShijuConnectorAuthorizationStatus,
}

macro_rules! redacted_debug {
    ($($ty:ty),+ $(,)?) => {$ (
        impl std::fmt::Debug for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($ty), "(<redacted>)"))
            }
        }
    )+};
}
redacted_debug!(
    ShijuConnectorBinding,
    ShijuConnectorSaveRequest,
    ShijuConnectorChallenge,
    ShijuConnectorPollRequest
);

impl GatewayClient {
    pub async fn shiju_connectors(
        &self,
        credential: &ShijuCredential,
    ) -> Result<Vec<ShijuConnector>, String> {
        self.shiju_account_sensitive_request::<(), _>(
            reqwest::Method::GET,
            "/v1/shiju/connectors",
            &[],
            None,
            Some(credential),
        )
        .await
    }
    pub async fn shiju_connector_save(
        &self,
        credential: &ShijuCredential,
        request: &ShijuConnectorSaveRequest,
    ) -> Result<ShijuConnectorBinding, String> {
        self.shiju_account_sensitive_request(
            reqwest::Method::PUT,
            "/v1/shiju/connectors/binding",
            &[],
            Some(request),
            Some(credential),
        )
        .await
    }
    pub async fn shiju_connector_authorize(
        &self,
        credential: &ShijuCredential,
        company_platform_id: u64,
    ) -> Result<ShijuConnectorChallenge, String> {
        self.shiju_account_sensitive_request(
            reqwest::Method::POST,
            "/v1/shiju/connectors/authorizations",
            &[],
            Some(&ShijuConnectorAuthorizeRequest { company_platform_id }),
            Some(credential),
        )
        .await
    }
    pub async fn shiju_connector_poll(
        &self,
        credential: &ShijuCredential,
        flow_id: &str,
    ) -> Result<ShijuConnectorPoll, String> {
        self.shiju_account_sensitive_request(
            reqwest::Method::POST,
            "/v1/shiju/connectors/authorizations/poll",
            &[],
            Some(&ShijuConnectorPollRequest { flow_id: flow_id.to_string() }),
            Some(credential),
        )
        .await
    }
}

fn nullable_variables<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ShijuConnectorVariable>, D::Error> {
    Ok(Option::<Vec<ShijuConnectorVariable>>::deserialize(deserializer)?.unwrap_or_default())
}
#[cfg(test)]
#[path = "shiju_connector_api_tests.rs"]
mod tests;
