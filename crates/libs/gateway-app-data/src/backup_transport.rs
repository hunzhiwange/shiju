//! Transport for authorized host-local backup operations.
use crate::GatewayAppDataClient;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use vw_api_app_operations::data_management::*;

/// Safe transport errors: no raw HTTP payloads, URLs or credentials are retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataBackupError {
    NativeTransportRequired,
    InvalidCredential,
    Transport,
    Timeout,
    Decode,
    ResponseTooLarge,
    Http { status: u16, error: Option<DataManagementError> },
}
impl std::fmt::Display for DataBackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http { status, .. } => write!(f, "backup HTTP {status}"),
            _ => f.write_str(match self {
                Self::NativeTransportRequired => "backup requires native transport",
                Self::InvalidCredential => "invalid backup credential",
                Self::Transport => "backup transport failed",
                Self::Timeout => "backup request timed out; query operation before resubmitting",
                Self::Decode => "invalid backup response",
                Self::ResponseTooLarge => "backup response too large",
                Self::Http { .. } => unreachable!(),
            }),
        }
    }
}
impl std::error::Error for DataBackupError {}

impl GatewayAppDataClient {
    pub(super) async fn backup_json<B: Serialize, T: DeserializeOwned>(&self, route: &str, input: &B, token: Option<&str>) -> Result<T, DataBackupError> {
        let bytes = self.backup_bytes(route, input, token).await?;
        serde_json::from_slice(&bytes).map_err(|_| DataBackupError::Decode)
    }

    // Browser fetch cannot enforce the backup transport's no-redirect policy.
    #[cfg(target_arch = "wasm32")]
    async fn backup_bytes<B: Serialize>(
        &self,
        _route: &str,
        _input: &B,
        _token: Option<&str>,
    ) -> Result<Vec<u8>, DataBackupError> {
        Err(DataBackupError::NativeTransportRequired)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn backup_bytes<B: Serialize>(
        &self,
        route: &str,
        input: &B,
        _token: Option<&str>,
    ) -> Result<Vec<u8>, DataBackupError> {
        use DataBackupError as Error;
        let transport = self.transport.native_transport().map_err(|_| Error::NativeTransportRequired)?;
        let request = transport.post(format!("{}/v1/agent-apps/data/{route}", self.transport.endpoint().base_url())).json(input);
        // Native transport disables redirects and retries and applies a bounded timeout.
        let mut response = gateway_http::apply_request_context(request, self.transport.endpoint())
            .send().await.map_err(transport_error)?;
        let status = response.status();
        let limit = if status.is_success() { 64 * 1024 * 1024 } else { 16 * 1024 };
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if bytes.len() + chunk.len() > limit { return Err(Error::ResponseTooLarge); }
            bytes.extend_from_slice(&chunk);
        }
        if !status.is_success() {
            return Err(Error::Http { status: status.as_u16(), error: serde_json::from_slice(&bytes).ok() });
        }
        Ok(bytes)
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn transport_error(error: reqwest::Error) -> DataBackupError {
    if error.is_timeout() { DataBackupError::Timeout } else { DataBackupError::Transport }
}
