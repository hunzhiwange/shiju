use crate::GatewayClient;
use gateway_app_data::*;
use vw_api_app_operations::data_management::*;

impl GatewayClient {
    pub async fn data_migration_identify(&self, input: &IdentifyDataSourceRequest, token: Option<&str>) -> Result<DataSourceSummary, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_migration_identify(input, token).await
    }

    pub async fn data_migration_preview(&self, input: &PreviewDataImportRequest, token: Option<&str>) -> Result<DataImportPreview, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_migration_preview(input, token).await
    }

    pub async fn data_migration_execute(&self, input: &ExecuteDataImportRequest, token: Option<&str>) -> Result<DataOperationProgress, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_migration_execute(input, token).await
    }

    pub async fn data_migration_operation(&self, input: &GetDataOperationRequest, token: Option<&str>) -> Result<DataOperationProgress, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_migration_operation(input, token).await
    }

    pub async fn data_migration_cancel(&self, input: &CancelDataOperationRequest, token: Option<&str>) -> Result<DataOperationProgress, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_migration_cancel(input, token).await
    }

    pub async fn data_migration_retry_projections(&self, input: &GetDataOperationRequest, token: Option<&str>) -> Result<DataOperationProgress, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_migration_retry_projections(input, token).await
    }

    pub async fn data_migration_export(&self, input: &ExportDataRequest, token: Option<&str>) -> Result<DataExportTransfer, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_migration_export(input, token).await
    }

    pub async fn data_migration_download(&self, target: &DataManagementTarget, transfer_id: &DataTransferId, token: Option<&str>) -> Result<Vec<u8>, DataMigrationError> {
        gateway_app_data::GatewayAppDataClient::new(self.transport.clone())
            .data_migration_download(target, transfer_id, token).await
    }
}
