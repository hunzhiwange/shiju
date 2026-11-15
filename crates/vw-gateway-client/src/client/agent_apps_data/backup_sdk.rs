use crate::GatewayClient;
use gateway_app_data::*;
use vw_api_app_operations::data_management::*;

impl GatewayClient {
    pub async fn data_backup_create(
        &self,
        input: &CreateDataBackupRequest,
        token: Option<&str>,
    ) -> Result<Option<DataBackupSummary>, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_create(input, token).await
    }

    pub async fn data_backup_list(
        &self,
        input: &ListDataBackupsRequest,
        token: Option<&str>,
    ) -> Result<Vec<DataBackupSummary>, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_list(input, token).await
    }

    pub async fn data_backup_rename(
        &self,
        input: &RenameDataBackupRequest,
        token: Option<&str>,
    ) -> Result<DataBackupSummary, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_rename(input, token).await
    }

    pub async fn data_backup_delete(
        &self,
        input: &DeleteDataBackupRequest,
        token: Option<&str>,
    ) -> Result<(), DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_delete(input, token).await
    }

    pub async fn data_backup_restore_preview(
        &self,
        input: &PreviewDataBackupRequest,
        token: Option<&str>,
    ) -> Result<DataBackupPreview, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_restore_preview(input, token).await
    }

    pub async fn data_backup_restore(
        &self,
        input: &RestoreDataBackupRequest,
        token: Option<&str>,
    ) -> Result<DataRestoreProgress, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_restore(input, token).await
    }

    pub async fn data_backup_restore_operation(
        &self,
        input: &GetDataOperationRequest,
        token: Option<&str>,
    ) -> Result<DataRestoreProgress, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_restore_operation(input, token).await
    }

    pub async fn data_backup_diagnose(
        &self,
        input: &ListDataBackupsRequest,
        token: Option<&str>,
    ) -> Result<DataRecoveryDiagnosis, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_diagnose(input, token).await
    }

    pub async fn data_backup_locations(
        &self,
        input: &ListDataBackupsRequest,
        token: Option<&str>,
    ) -> Result<Vec<DataBackupLocation>, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_locations(input, token).await
    }

    pub async fn data_backup_delete_location(
        &self,
        input: &DeleteDataBackupLocationRequest,
        token: Option<&str>,
    ) -> Result<DataBackupLocationDeletion, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_delete_location(input, token).await
    }

    pub async fn data_backup_policy(
        &self,
        target: &DataManagementTarget,
        token: Option<&str>,
    ) -> Result<DataBackupPolicy, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_policy(target, token).await
    }

    pub async fn data_backup_set_policy(
        &self,
        target: &DataManagementTarget,
        settings: &DataBackupPolicy,
        token: Option<&str>,
    ) -> Result<DataBackupPolicy, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_set_policy(target, settings, token).await
    }

    pub async fn data_backup_location_path(&self, target: &DataManagementTarget, category_id: &str, token: Option<&str>) -> Result<String, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_backup_location_path(target, category_id, token).await
    }
}
