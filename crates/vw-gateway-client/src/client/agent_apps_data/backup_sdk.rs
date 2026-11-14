//! Executor-owned backups. IDs travel in JSON, never as paths or URL segments.
use super::super::GatewayClient;
use super::DataMigrationError;
use serde::{Deserialize, Serialize};
use vw_api_types::agent_apps::data_management::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataBackupPolicy {
    /// Zero disables scheduled capture; administrator authorization is still required.
    pub interval_hours: u32,
    pub retain_count: u32,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDataBackupRequest {
    pub target: DataManagementTarget,
    pub backup_id: DataBackupId,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataBackupPreview {
    pub backup: DataBackupSummary,
    pub requires_confirmation: bool,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataRestoreStatus {
    Queued,
    Running,
    Completed,
    Failed,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataRestoreResult {
    pub committed: Option<bool>,
    pub projection: Option<String>,
    pub retention_failed: Option<bool>,
    pub code: Option<String>,
}
/// Completion may still require refreshing projections or reporting retention failure.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataRestoreProgress {
    pub target: DataManagementTarget,
    pub operation_id: String,
    pub status: DataRestoreStatus,
    pub rollback_backup_id: Option<DataBackupId>,
    pub result: Option<DataRestoreResult>,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataTargetHealth {
    Healthy,
    Corrupt,
    VersionUnsupported,
    Unavailable,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataBackupRestoreStatus {
    RequiresOwnerValidation,
    VersionUnsupported,
    Corrupt,
}
// Diagnosis returns the owner's snake_case metadata, not DataBackupSummary.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DataRecoveryBackup {
    pub id: String,
    pub kind: DataArtifactKind,
    pub format_version: u32,
    pub size_bytes: u64,
    pub created_at_ms: i64,
    pub restore_status: DataBackupRestoreStatus,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataRecoveryDiagnosis {
    pub target_health: DataTargetHealth,
    pub backups: Vec<DataRecoveryBackup>,
}
/// Statistics describe the requested execution host. No client-local path is implied.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DataBackupLocation {
    pub id: String,
    pub size_bytes: u64,
    pub item_count: u64,
    pub last_modified: Option<std::time::SystemTime>,
    pub deletable: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDataBackupLocationRequest {
    pub target: DataManagementTarget,
    pub category_id: String,
    pub confirmed: bool,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataBackupLocationDeletion {
    pub freed_bytes: u64,
    pub undeleted_count: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PolicyRequest<'a> {
    target: &'a DataManagementTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    settings: Option<&'a DataBackupPolicy>,
}

impl GatewayClient {
    /// No implicit retry. On ambiguous restore failure query the operation identity.
    pub async fn data_backup_create(
        &self,
        input: &CreateDataBackupRequest,
        token: Option<&str>,
    ) -> Result<Option<DataBackupSummary>, DataMigrationError> {
        self.data_migration_json("backups/create", input, token).await
    }
    pub async fn data_backup_list(
        &self,
        input: &ListDataBackupsRequest,
        token: Option<&str>,
    ) -> Result<Vec<DataBackupSummary>, DataMigrationError> {
        self.data_migration_json("backups/list", input, token).await
    }
    pub async fn data_backup_rename(
        &self,
        input: &RenameDataBackupRequest,
        token: Option<&str>,
    ) -> Result<DataBackupSummary, DataMigrationError> {
        self.data_migration_json("backups/rename", input, token).await
    }
    /// HTTP 409 is a conflict; the API does not distinguish protection from stale revision.
    pub async fn data_backup_delete(
        &self,
        input: &DeleteDataBackupRequest,
        token: Option<&str>,
    ) -> Result<(), DataMigrationError> {
        self.data_migration_json("backups/delete", input, token).await
    }
    pub async fn data_backup_restore_preview(
        &self,
        input: &PreviewDataBackupRequest,
        token: Option<&str>,
    ) -> Result<DataBackupPreview, DataMigrationError> {
        self.data_migration_json("backups/restore/preview", input, token).await
    }
    pub async fn data_backup_restore(
        &self,
        input: &RestoreDataBackupRequest,
        token: Option<&str>,
    ) -> Result<DataRestoreProgress, DataMigrationError> {
        self.data_migration_json("backups/restore", input, token).await
    }
    pub async fn data_backup_restore_operation(
        &self,
        input: &GetDataOperationRequest,
        token: Option<&str>,
    ) -> Result<DataRestoreProgress, DataMigrationError> {
        self.data_migration_json("backups/restore/get", input, token).await
    }
    pub async fn data_backup_diagnose(
        &self,
        input: &ListDataBackupsRequest,
        token: Option<&str>,
    ) -> Result<DataRecoveryDiagnosis, DataMigrationError> {
        self.data_migration_json("backups/diagnose", input, token).await
    }
    pub async fn data_backup_locations(
        &self,
        input: &ListDataBackupsRequest,
        token: Option<&str>,
    ) -> Result<Vec<DataBackupLocation>, DataMigrationError> {
        self.data_migration_json("locations/list", input, token).await
    }
    /// A successful HTTP response can still contain undeleted items.
    pub async fn data_backup_delete_location(
        &self,
        input: &DeleteDataBackupLocationRequest,
        token: Option<&str>,
    ) -> Result<DataBackupLocationDeletion, DataMigrationError> {
        self.data_migration_json("locations/delete", input, token).await
    }
    pub async fn data_backup_policy(
        &self,
        target: &DataManagementTarget,
        token: Option<&str>,
    ) -> Result<DataBackupPolicy, DataMigrationError> {
        self.data_migration_json("policy", &PolicyRequest { target, settings: None }, token).await
    }
    pub async fn data_backup_set_policy(
        &self,
        target: &DataManagementTarget,
        settings: &DataBackupPolicy,
        token: Option<&str>,
    ) -> Result<DataBackupPolicy, DataMigrationError> {
        self.data_migration_json(
            "policy",
            &PolicyRequest { target, settings: Some(settings) },
            token,
        )
        .await
    }
}

impl GatewayClient {
    pub async fn data_backup_location_path(&self, target: &DataManagementTarget, category_id: &str, token: Option<&str>) -> Result<String, DataMigrationError> {
        self.data_migration_json("locations/resolve", &serde_json::json!({"target": target, "categoryId": category_id}), token).await
    }
}
