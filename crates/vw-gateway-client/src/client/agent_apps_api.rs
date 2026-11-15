//! 原生图标库 SDK，复用网关认证、租户上下文与错误映射。

use vw_api_app_identity::{
    AddCustomIconRequest,
    AddCustomIconResult,
    CustomIconEntry,
    RenameCustomIconRequest,
};

use super::GatewayClient;

impl GatewayClient {
    /// 读取当前租户的图标列表；断线恢复时重新调用以获取最新列表。
    pub async fn custom_icons_list(&self) -> Result<Vec<CustomIconEntry>, String> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone())
            .custom_icons_list().await
    }
    /// 新增图标，重复内容通过 existed 返回；写入失败直接交由调用方处理。
    pub async fn custom_icon_add(
        &self,
        request: &AddCustomIconRequest,
    ) -> Result<AddCustomIconResult, String> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone())
            .custom_icon_add(request).await
    }
    /// 修改指定图标名称，ID 始终作为单一路径段编码。
    pub async fn custom_icon_rename(
        &self,
        id: &str,
        request: &RenameCustomIconRequest,
    ) -> Result<CustomIconEntry, String> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone())
            .custom_icon_rename(id, request).await
    }
    /// 删除指定图标，接受服务端的 204 空响应。
    pub async fn custom_icon_delete(&self, id: &str) -> Result<(), String> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone())
            .custom_icon_delete(id).await
    }
}
