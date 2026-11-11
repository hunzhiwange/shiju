//! Project-scoped Pi and OMP directories. Native paths and profile selection stay remote.
use super::{AggregationApiError as Error, GatewayClient};
use vw_api_types::agent_apps::aggregation::{
    AggregationQuery, OmpDirectoryState, OmpDirectoryWrite, PiDirectoryState, PiDirectoryWrite,
};

impl GatewayClient {
    /// Read Pi directory metadata without changing native defaults or login.
    pub async fn pi_directory_state(
        &self,
        query: &AggregationQuery,
    ) -> Result<PiDirectoryState, Error> {
        self.native_directory_request("pi", "config", reqwest::Method::POST, "/state", query).await
    }
    /// Apply one typed Pi entry change using the revision returned by the gateway.
    pub async fn pi_directory_entry(
        &self,
        id: &str,
        body: &PiDirectoryWrite,
    ) -> Result<PiDirectoryState, Error> {
        self.native_directory_request("pi", id, reqwest::Method::PUT, "/directory-entry", body)
            .await
    }
    /// Read OMP metadata including the profile selected by the execution host.
    pub async fn omp_directory_state(
        &self,
        query: &AggregationQuery,
    ) -> Result<OmpDirectoryState, Error> {
        self.native_directory_request("omp", "config", reqwest::Method::POST, "/state", query).await
    }
    /// Apply one typed OMP entry change; the supplied profile is a provider declaration.
    pub async fn omp_directory_entry(
        &self,
        id: &str,
        body: &OmpDirectoryWrite,
    ) -> Result<OmpDirectoryState, Error> {
        self.native_directory_request("omp", id, reqwest::Method::PUT, "/directory-entry", body)
            .await
    }
    async fn native_directory_request<B: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        app: &str,
        id: &str,
        method: reqwest::Method,
        action: &str,
        body: &B,
    ) -> Result<R, Error> {
        if id.is_empty()
            || id.len() > 128
            || !id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
        {
            return Err(Error::Transport("invalid provider identity".into()));
        }
        let request = self
            .dynamic_host_client
            .request(
                method,
                format!("{}/v1/app-providers/{app}/{id}{action}", self.endpoint.base_url()),
            )
            .json(body);
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|_| Error::Transport("native directory request failed".into()))?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|_| Error::Transport("native directory response read failed".into()))?;
        if !status.is_success() {
            return Err(Error::Http {
                status: status.as_u16(),
                body: serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
            });
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| Error::Decode("invalid native directory response".into()))
    }
}
