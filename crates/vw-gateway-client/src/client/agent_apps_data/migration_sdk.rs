//! Migration transport uses opaque authorized sources, never client filesystem paths.
use super::super::GatewayClient;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use vw_api_types::agent_apps::data_management::*;

/// Safe transport errors: no raw HTTP payloads, URLs or credentials are retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataMigrationError {
    NativeTransportRequired,
    InvalidCredential,
    Transport,
    Timeout,
    Decode,
    ResponseTooLarge,
    Http { status: u16, error: Option<DataManagementError> },
}
impl std::fmt::Display for DataMigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http { status, .. } => write!(f, "data migration HTTP {status}"),
            _ => f.write_str(match self {
                Self::NativeTransportRequired => "data migration requires native transport",
                Self::InvalidCredential => "invalid data migration credential",
                Self::Transport => "data migration transport failed",
                Self::Timeout => "data migration request timed out; query operation before resubmitting",
                Self::Decode => "invalid data migration response",
                Self::ResponseTooLarge => "data migration response too large",
                Self::Http { .. } => unreachable!(),
            }),
        }
    }
}
impl std::error::Error for DataMigrationError {}

/// Actual export API response; exporting does not return an operation record.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataExportTransfer {
    pub transfer_id: DataTransferId,
    pub expires_in_seconds: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadRequest<'a> {
    target: &'a DataManagementTarget,
    transfer_id: &'a DataTransferId,
}

impl GatewayClient {
    /// Inspect an administrator-authorized source handle. No upload route exists yet.
    pub async fn data_migration_identify(&self, input: &IdentifyDataSourceRequest, token: Option<&str>) -> Result<DataSourceSummary, DataMigrationError> {
        self.data_migration_json("sources/identify", input, token).await
    }

    /// Preview only; this grants no write authority.
    pub async fn data_migration_preview(&self, input: &PreviewDataImportRequest, token: Option<&str>) -> Result<DataImportPreview, DataMigrationError> {
        self.data_migration_json("import/preview", input, token).await
    }

    /// Send exactly once. On ambiguous failure query with the same operation identity.
    pub async fn data_migration_execute(&self, input: &ExecuteDataImportRequest, token: Option<&str>) -> Result<DataOperationProgress, DataMigrationError> {
        self.data_migration_json("import/execute", input, token).await
    }

    /// Preserve committed, rollback, partial results and cancellation intent verbatim.
    pub async fn data_migration_operation(&self, input: &GetDataOperationRequest, token: Option<&str>) -> Result<DataOperationProgress, DataMigrationError> {
        self.data_migration_json("operations/get", input, token).await
    }

    /// Request cancellation; the server decides the commit boundary.
    pub async fn data_migration_cancel(&self, input: &CancelDataOperationRequest, token: Option<&str>) -> Result<DataOperationProgress, DataMigrationError> {
        self.data_migration_json("operations/cancel", input, token).await
    }

    /// Explicitly retry projection warnings, without replaying the import.
    pub async fn data_migration_retry_projections(&self, input: &GetDataOperationRequest, token: Option<&str>) -> Result<DataOperationProgress, DataMigrationError> {
        self.data_migration_json("operations/retry-projections", input, token).await
    }

    /// Create a portable export handle; unsupported categories remain HTTP errors.
    pub async fn data_migration_export(&self, input: &ExportDataRequest, token: Option<&str>) -> Result<DataExportTransfer, DataMigrationError> {
        self.data_migration_json("export", input, token).await
    }

    /// Download the portable archive into memory, without writing client paths.
    pub async fn data_migration_download(&self, target: &DataManagementTarget, transfer_id: &DataTransferId, token: Option<&str>) -> Result<Vec<u8>, DataMigrationError> {
        self.data_migration_bytes("export/download", &DownloadRequest { target, transfer_id }, token).await
    }

    async fn data_migration_json<B: Serialize, T: DeserializeOwned>(&self, route: &str, input: &B, token: Option<&str>) -> Result<T, DataMigrationError> {
        let bytes = self.data_migration_bytes(route, input, token).await?;
        serde_json::from_slice(&bytes).map_err(|_| DataMigrationError::Decode)
    }

    async fn data_migration_bytes<B: Serialize>(&self, route: &str, input: &B, token: Option<&str>) -> Result<Vec<u8>, DataMigrationError> {
        use DataMigrationError as Error;
        let transport = self.native_transport().map_err(|_| Error::NativeTransportRequired)?;
        let mut request = transport.post(format!("{}/v1/agent-apps/data/{route}", self.endpoint.base_url())).json(input);
        if let Some(token) = token {
            let mut value = reqwest::header::HeaderValue::from_str(token).map_err(|_| Error::InvalidCredential)?;
            value.set_sensitive(true);
            request = request.header("x-agent-apps-token", value);
        }
        // Native transport disables redirects and retries and applies a bounded timeout.
        let mut response = crate::http::apply_request_context(request, &self.endpoint)
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
fn transport_error(error: reqwest::Error) -> DataMigrationError {
    if error.is_timeout() { DataMigrationError::Timeout } else { DataMigrationError::Transport }
}
