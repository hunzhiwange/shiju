//! 原生图标库 SDK，复用网关认证、租户上下文与错误映射。

use vw_api_types::agent_apps::{
    AddCustomIconRequest, AddCustomIconResult, CustomIconEntry, RenameCustomIconRequest,
};

use super::GatewayClient;

impl GatewayClient {
    /// 读取当前租户的图标列表；断线恢复时重新调用以获取最新列表。
    pub async fn custom_icons_list(&self) -> Result<Vec<CustomIconEntry>, String> {
        self.get_json("/v1/agent-apps/custom-icons", &[]).await
    }
}

impl GatewayClient {
    /// 新增图标，重复内容通过 existed 返回；写入失败直接交由调用方处理。
    pub async fn custom_icon_add(
        &self,
        request: &AddCustomIconRequest,
    ) -> Result<AddCustomIconResult, String> {
        self.post_json("/v1/agent-apps/custom-icons", &[], request).await
    }

    /// 修改指定图标名称，ID 始终作为单一路径段编码。
    pub async fn custom_icon_rename(
        &self,
        id: &str,
        request: &RenameCustomIconRequest,
    ) -> Result<CustomIconEntry, String> {
        self.patch_json(&custom_icon_path(id), &[], request).await
    }

    /// 删除指定图标，接受服务端的 204 空响应。
    pub async fn custom_icon_delete(&self, id: &str) -> Result<(), String> {
        self.delete_empty(&custom_icon_path(id), &[]).await
    }
}

fn custom_icon_path(id: &str) -> String {
    let mut encoded = String::new();
    for byte in id.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    format!("/v1/agent-apps/custom-icons/{encoded}")
}
