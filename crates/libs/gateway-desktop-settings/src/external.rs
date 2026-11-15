use crate::ExternalAppsStateDto;
use serde_json::Value;
use vw_api_file::file::FileNodeKind;
use crate::GatewayDesktopSettingsClient;

impl GatewayDesktopSettingsClient {
    /// 读取桌面端外部应用可用性状态。
    pub async fn desktop_external_apps_get(&self) -> Result<ExternalAppsStateDto, String> {
        let value: Value = self.transport.get_json("/v1/desktop/external-apps", &[]).await?;
        let platform = value.get("platform").and_then(Value::as_str).map(ToString::to_string);
        let apps = value
            .get("apps")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let id = item.get("id").and_then(Value::as_str)?;
                        let available =
                            item.get("available").and_then(Value::as_bool).unwrap_or(false);
                        Some((id.to_string(), available))
                    })
                    .collect::<Vec<(String, bool)>>()
            })
            .unwrap_or_default();
        Ok(ExternalAppsStateDto { platform, apps })
    }

    /// 调用指定外部应用打开目标路径。
    pub async fn desktop_external_app_open(&self, path: &str, target: &str) -> Result<(), String> {
        let _: Value = self.transport.post_json(
                "/v1/desktop/external-apps/open",
                &[],
                &serde_json::json!({
                    "path": path,
                    "target": target,
                }),
            )
            .await?;
        Ok(())
    }

    /// 判断网关主机上的绝对路径指向文件还是目录。
    pub async fn desktop_external_path_kind(&self, path: &str) -> Result<FileNodeKind, String> {
        self.transport.post_json("/v1/desktop/external-path/kind", &[], &serde_json::json!({ "path": path }))
            .await
    }

    /// 在系统文件管理器中定位并高亮指定路径。
    pub async fn desktop_external_path_reveal(&self, path: &str) -> Result<(), String> {
        let _: Value = self.transport.post_json(
                "/v1/desktop/external-path/reveal",
                &[],
                &serde_json::json!({
                    "path": path,
                }),
            )
            .await?;
        Ok(())
    }
}
