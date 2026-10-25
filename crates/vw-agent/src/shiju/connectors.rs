//! 通过识矩固定数据网关访问当前用户连接器，禁止传入目标用户或公司身份。
use super::{ShijuAccountClient, ShijuAccountError, decode_response, endpoint};
use reqwest::Method;
use serde::Serialize;
use serde::de::DeserializeOwned;
use vw_gateway_client::{
    ShijuConnector, ShijuConnectorAuthorizeRequest, ShijuConnectorBinding, ShijuConnectorChallenge,
    ShijuConnectorPoll, ShijuConnectorSaveRequest,
};

const CONNECTORS_PATH: &str = "data/api/v1/me/account-binding-connectors";
const BINDINGS_PATH: &str = "data/api/v1/me/account-bindings";
const AUTHORIZATIONS_PATH: &str = "data/api/v1/me/account-binding-authorizations";
const REQUEST_FAILED: &str = "SHIJU connector request failed";

impl ShijuAccountClient {
    pub(crate) async fn connectors(
        &self,
        credential: &str,
    ) -> Result<Vec<ShijuConnector>, ShijuAccountError> {
        let mut connectors: Vec<ShijuConnector> =
            self.connector_request::<(), _>(Method::GET, CONNECTORS_PATH, credential, None).await?;
        // 即使上游意外返回密码，也不允许它进入桌面列表或调试日志。
        for connector in &mut connectors {
            if let Some(binding) = &mut connector.binding {
                binding.binding_config.retain(|name, _| {
                    connector
                        .account_binding_variables
                        .iter()
                        .any(|variable| variable.name == *name && variable.kind != "password")
                });
            }
        }
        Ok(connectors)
    }

    pub(crate) async fn save_connector_binding(
        &self,
        credential: &str,
        request: &ShijuConnectorSaveRequest,
    ) -> Result<ShijuConnectorBinding, ShijuAccountError> {
        let connector = self
            .connectors(credential)
            .await?
            .into_iter()
            .find(|connector| connector.company_platform_id == request.company_platform_id)
            .ok_or(invalid_input("SHIJU connector is not available"))?;
        if request.third_party_account_id.chars().count() > 200
            || request.third_party_account_name.chars().count() > 200
        {
            return Err(invalid_input("SHIJU connector account is too long"));
        }
        if request.binding_config.keys().any(|key| {
            !connector.account_binding_variables.iter().any(|variable| variable.name == *key)
        }) {
            return Err(invalid_input("SHIJU connector variable is not supported"));
        }
        for variable in &connector.account_binding_variables {
            let value = request.binding_config.get(&variable.name);
            if value.is_some_and(|value| !value.is_string())
                || (variable.required
                    && value
                        .and_then(|value| value.as_str())
                        .is_none_or(|value| value.trim().is_empty()))
            {
                return Err(invalid_input(
                    "SHIJU connector required variable is missing or invalid",
                ));
            }
        }
        let mut binding: ShijuConnectorBinding =
            self.connector_request(Method::PUT, BINDINGS_PATH, credential, Some(request)).await?;
        binding.binding_config.retain(|name, _| {
            connector
                .account_binding_variables
                .iter()
                .any(|variable| variable.name == *name && variable.kind != "password")
        });
        Ok(binding)
    }

    pub(crate) async fn authorize_connector(
        &self,
        credential: &str,
        request: &ShijuConnectorAuthorizeRequest,
    ) -> Result<ShijuConnectorChallenge, ShijuAccountError> {
        if !self.connectors(credential).await?.iter().any(|connector| {
            connector.company_platform_id == request.company_platform_id
                && connector.authorization_available
        }) {
            return Err(invalid_input("SHIJU connector authorization is not available"));
        }
        let challenge: ShijuConnectorChallenge = self
            .connector_request(Method::POST, AUTHORIZATIONS_PATH, credential, Some(request))
            .await?;
        validate_flow_id(&challenge.flow_id)?;
        let url = reqwest::Url::parse(&challenge.authorization_url).map_err(|_| {
            ShijuAccountError::Message("SHIJU connector authorization URL is invalid")
        })?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || challenge.expires_in_seconds == 0
        {
            return Err(ShijuAccountError::Message(
                "SHIJU connector authorization URL or lifetime is invalid",
            ));
        }
        Ok(challenge)
    }

    pub(crate) async fn poll_connector(
        &self,
        credential: &str,
        flow_id: &str,
    ) -> Result<ShijuConnectorPoll, ShijuAccountError> {
        validate_flow_id(flow_id)?;
        self.connector_request::<(), _>(
            Method::GET,
            &format!("{AUTHORIZATIONS_PATH}/{flow_id}"),
            credential,
            None,
        )
        .await
    }

    async fn connector_request<B: Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        credential: &str,
        body: Option<&B>,
    ) -> Result<T, ShijuAccountError> {
        if credential.trim().is_empty() {
            return Err(ShijuAccountError::Message("SHIJU credential cannot be empty"));
        }
        let mut request = self
            .http
            .request(method, endpoint(&self.gateway_base, path)?)
            .header("X-Api-Key", credential);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response =
            request.send().await.map_err(|_| ShijuAccountError::Message(REQUEST_FAILED))?;
        decode_response(response, REQUEST_FAILED).await
    }
}

fn invalid_input(message: &'static str) -> ShijuAccountError {
    ShijuAccountError::Upstream {
        status: reqwest::StatusCode::BAD_REQUEST,
        message: message.to_string(),
    }
}

fn validate_flow_id(flow_id: &str) -> Result<(), ShijuAccountError> {
    if !(32..=128).contains(&flow_id.len())
        || !flow_id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(invalid_input("SHIJU connector flow ID is invalid"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "connectors_tests.rs"]
mod tests;
