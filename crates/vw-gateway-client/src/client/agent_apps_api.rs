//! 原生图标库读取 SDK，复用网关认证、租户上下文与错误映射。

use vw_api_types::agent_apps::CustomIconEntry;

use super::GatewayClient;

impl GatewayClient {
    /// 读取当前租户的图标列表；断线恢复时重新调用以获取最新列表。
    pub async fn custom_icons_list(&self) -> Result<Vec<CustomIconEntry>, String> {
        self.get_json("/v1/agent-apps/custom-icons", &[]).await
    }
}
