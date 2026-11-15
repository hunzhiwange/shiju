use crate::{DesktopPluginCatalogEntryDto, DesktopPluginDetailDto, DesktopPluginPreviewDto, DesktopPluginPathDto};
use serde_json::Value;
use crate::GatewayDesktopSettingsClient;

impl GatewayDesktopSettingsClient {
    /// 读取已发现插件目录。
    pub async fn plugins_get(&self) -> Result<Vec<DesktopPluginCatalogEntryDto>, String> {
        self.transport.get_json("/v1/plugins", &[]).await
    }

    /// Read state-bound plugin diagnostics from the running authenticated gateway.
    pub async fn plugins_status_get(&self) -> Result<Vec<DesktopPluginCatalogEntryDto>, String> {
        self.transport.get_json("/v1/plugins/status", &[]).await
    }

    /// 读取指定插件详情。
    pub async fn plugin_detail_get(
        &self,
        plugin_id: &str,
    ) -> Result<DesktopPluginDetailDto, String> {
        self.transport.get_json("/v1/plugins/detail", &[("plugin_id".to_string(), plugin_id.to_string())])
            .await
    }

    /// Prepare and audit a source without publishing it to the plugin store.
    pub async fn plugin_prepare_install(
        &self,
        source: &str,
    ) -> Result<DesktopPluginPreviewDto, String> {
        self.transport.post_json("/v1/plugins/prepare-install", &[], &serde_json::json!({"source":source}))
            .await
    }

    pub async fn plugin_install(&self, token: &str, approved: bool) -> Result<String, String> {
        let response: DesktopPluginPathDto = self.transport.post_json(
                "/v1/plugins/install",
                &[],
                &serde_json::json!({"token":token,"approved":approved}),
            )
            .await?;
        Ok(response.path)
    }

    pub async fn plugin_prepare_update(
        &self,
        plugin_id: &str,
        source: Option<&str>,
    ) -> Result<DesktopPluginPreviewDto, String> {
        self.transport.post_json(
            "/v1/plugins/prepare-update",
            &[],
            &serde_json::json!({"plugin_id":plugin_id,"source":source}),
        )
        .await
    }

    pub async fn plugin_update(&self, token: &str, approved: bool) -> Result<String, String> {
        let response: DesktopPluginPathDto = self.transport.post_json(
                "/v1/plugins/update",
                &[],
                &serde_json::json!({"token":token,"approved":approved}),
            )
            .await?;
        Ok(response.path)
    }

    /// Discard a pending preview and release its private staging files.
    pub async fn plugin_cancel(&self, token: &str) -> Result<(), String> {
        let _: Value =
            self.transport.post_json("/v1/plugins/cancel", &[], &serde_json::json!({"token":token})).await?;
        Ok(())
    }

    /// 设置本地插件启用状态。
    pub async fn plugin_set_enabled(
        &self,
        plugin_id: &str,
        enabled: bool,
    ) -> Result<String, String> {
        let response: DesktopPluginPathDto = self.transport.post_json(
                "/v1/plugins/set-enabled",
                &[],
                &serde_json::json!({
                    "plugin_id": plugin_id,
                    "enabled": enabled,
                }),
            )
            .await?;
        Ok(response.path)
    }

    /// 删除本地插件目录。
    pub async fn plugin_delete(&self, plugin_id: &str) -> Result<String, String> {
        let response: DesktopPluginPathDto = self.transport.post_json(
                "/v1/plugins/delete",
                &[],
                &serde_json::json!({
                    "plugin_id": plugin_id,
                }),
            )
            .await?;
        Ok(response.path)
    }
}
