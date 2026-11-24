//! 当前用户连接器账号绑定与授权契约；敏感载荷不参与调试输出。
use super::GatewayShijuConnectorClient;
use gateway_shiju_account::{GatewayShijuAccountClient, ShijuCredential};
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
    #[serde(default)]
    pub third_party_account_id: String,
    #[serde(default)]
    pub third_party_account_name: String,
    #[serde(default)]
    pub binding_config: Map<String, Value>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuConnector {
    #[serde(deserialize_with = "deserialize_connector_id")]
    pub platform_id: u64,
    #[serde(default)]
    pub platform_code: String,
    pub platform_name: String,
    #[serde(deserialize_with = "deserialize_connector_id")]
    pub company_platform_id: u64,
    #[serde(default)]
    pub authorization_available: bool,
    #[serde(default, deserialize_with = "nullable_variables")]
    pub account_binding_variables: Vec<ShijuConnectorVariable>,
    pub binding: Option<ShijuConnectorBinding>,
}

pub fn deserialize_connector_id<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<u64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Id {
        Number(u64),
        String(String),
    }

    match Id::deserialize(deserializer)? {
        Id::Number(id) => Ok(id),
        Id::String(id) => id.parse().map_err(serde::de::Error::custom),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShijuConnectorCatalogScope {
    System,
    Custom,
}

impl ShijuConnectorCatalogScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShijuConnectorCatalogStatus {
    Unavailable,
    Disabled,
    NoBinding,
    NoAccess,
    Bindable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuConnectorCatalogEntry {
    pub platform_id: u64,
    pub name: String,
    pub code: String,
    pub description: String,
    pub status: ShijuConnectorCatalogStatus,
    pub account: Option<ShijuConnector>,
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

impl GatewayShijuConnectorClient {
    pub async fn shiju_connector_catalog(
        &self,
        credential: &ShijuCredential,
        scope: ShijuConnectorCatalogScope,
    ) -> Result<Vec<ShijuConnectorCatalogEntry>, String> {
        GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_sensitive_request::<(), _>(
                reqwest::Method::GET,
                "/v1/shiju/connectors/catalog",
                &[("scope".to_string(), scope.as_str().to_string())],
                None,
                Some(credential),
            )
            .await
    }

    pub async fn shiju_connectors(
        &self,
        credential: &ShijuCredential,
    ) -> Result<Vec<ShijuConnector>, String> {
        GatewayShijuAccountClient::new(self.transport.clone()).shiju_account_sensitive_request::<(), _>(
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
        GatewayShijuAccountClient::new(self.transport.clone()).shiju_account_sensitive_request(
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
        GatewayShijuAccountClient::new(self.transport.clone()).shiju_account_sensitive_request(
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
        GatewayShijuAccountClient::new(self.transport.clone()).shiju_account_sensitive_request(
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
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Variables {
        Many(Vec<ShijuConnectorVariable>),
        One(ShijuConnectorVariable),
    }

    Ok(match Option::<Variables>::deserialize(deserializer)? {
        Some(Variables::Many(variables)) => variables,
        Some(Variables::One(variable)) => vec![variable],
        None => Vec::new(),
    })
}
#[cfg(test)]
#[path = "shiju_connector_api_tests.rs"]
mod tests;
