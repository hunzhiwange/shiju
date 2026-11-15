//! JSON 请求兼容转发。
use serde::{Serialize, de::DeserializeOwned};
use super::GatewayClient;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use gateway_http::get_json_blocking;

impl GatewayClient {
    pub(super) async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<T, String> {
        self.transport.get_json(path, query).await
    }
    pub(super) async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
        body: &B,
    ) -> Result<T, String> {
        self.transport.post_json(path, query, body).await
    }
    pub(super) async fn patch_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
        body: &B,
    ) -> Result<T, String> {
        self.transport.patch_json(path, query, body).await
    }
    pub(super) async fn put_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
        body: &B,
    ) -> Result<T, String> {
        self.transport.put_json(path, query, body).await
    }
    pub(super) async fn delete_empty(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<(), String> {
        self.transport.delete_empty(path, query).await
    }
    pub(super) async fn delete_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
        body: &B,
    ) -> Result<T, String> {
        self.transport.delete_json(path, query, body).await
    }
    pub(super) async fn get_json_with_404<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<Option<T>, String> {
        self.transport.get_json_with_404(path, query).await
    }
}
