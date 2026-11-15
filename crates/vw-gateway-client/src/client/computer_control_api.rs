//! 计算机控制兼容门面。
use super::GatewayClient;
use gateway_computer_control::{ComputerControlApiError, GatewayComputerControlClient};
use vw_api_computer_control::*;

impl GatewayClient {
    pub async fn computer_control_status(&self) -> Result<ComputerControlStatus, ComputerControlApiError> {
        GatewayComputerControlClient::new(self.transport.clone()).computer_control_status().await
    }
    pub async fn computer_control_approvals(&self) -> Result<ControlApprovals, ComputerControlApiError> {
        GatewayComputerControlClient::new(self.transport.clone()).computer_control_approvals().await
    }
    pub async fn computer_control_save(&self, body: &ControlSettingsUpdate) -> Result<ControlVersionResponse, ComputerControlApiError> {
        GatewayComputerControlClient::new(self.transport.clone()).computer_control_save(body).await
    }
    pub async fn computer_control_revoke(&self, body: &ControlGrantRevocation) -> Result<ControlVersionResponse, ComputerControlApiError> {
        GatewayComputerControlClient::new(self.transport.clone()).computer_control_revoke(body).await
    }
    pub async fn computer_control_pair(&self, body: &ControlPairingConfirmation) -> Result<ControlPairingResponse, ComputerControlApiError> {
        GatewayComputerControlClient::new(self.transport.clone()).computer_control_pair(body).await
    }
    pub async fn computer_control_stop(&self) -> Result<(), ComputerControlApiError> {
        GatewayComputerControlClient::new(self.transport.clone()).computer_control_stop().await
    }
}
