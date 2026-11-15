//! PTY 兼容门面。

pub use gateway_pty::*;
use super::GatewayClient;

impl GatewayClient {
    pub async fn pty_list(&self, directory: Option<&str>) -> Result<Vec<PtyInfoDto>, String> {
        GatewayPtyClient::new(self.transport.clone()).pty_list(directory).await
    }

    pub async fn pty_create(&self, input: &PtyCreateRequest, directory: Option<&str>) -> Result<PtyInfoDto, String> {
        GatewayPtyClient::new(self.transport.clone()).pty_create(input, directory).await
    }

    pub async fn pty_get(&self, pty_id: &str, directory: Option<&str>) -> Result<PtyInfoDto, String> {
        GatewayPtyClient::new(self.transport.clone()).pty_get(pty_id, directory).await
    }

    pub async fn pty_update(&self, pty_id: &str, input: &PtyUpdateRequest, directory: Option<&str>) -> Result<PtyInfoDto, String> {
        GatewayPtyClient::new(self.transport.clone()).pty_update(pty_id, input, directory).await
    }

    pub async fn pty_delete(&self, pty_id: &str, directory: Option<&str>) -> Result<bool, String> {
        GatewayPtyClient::new(self.transport.clone()).pty_delete(pty_id, directory).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn pty_connect(&self, pty_id: &str, directory: Option<&str>, cursor: Option<i64>) -> Result<PtyConnection, String> {
        GatewayPtyClient::new(self.transport.clone()).pty_connect(pty_id, directory, cursor).await
    }

}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "pty_facade_tests.rs"]
mod pty_facade_tests;
