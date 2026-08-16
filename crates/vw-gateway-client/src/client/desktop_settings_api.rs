use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::GatewayClient;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopSkillCatalogEntryDto {
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub resource_count: usize,
    pub installed: bool,
    pub enabled: bool,
    pub source: String,
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_pack: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopSkillDetailDto {
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub installed: bool,
    pub enabled: bool,
    pub source: String,
    pub source_path: Option<String>,
    pub document_name: String,
    pub document_content: String,
    pub can_install: bool,
    pub can_toggle: bool,
    pub can_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopSkillPathDto {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopPluginCatalogEntryDto {
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub component_count: usize,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopPluginCapabilitiesDto {
    pub skills: usize,
    pub mcp_servers: usize,
    pub hooks: usize,
    pub interface: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopPluginDetailDto {
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub source_path: Option<String>,
    pub manifest_path: String,
    pub capabilities: DesktopPluginCapabilitiesDto,
    pub can_toggle: bool,
    pub can_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopPluginPathDto {
    pub path: String,
}

/// One statically audited hook declaration from an installed plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookCatalogEntryDto {
    pub plugin_id: String,
    pub event: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: String,
    pub mode: String,
    pub hook_hash: String,
    pub trusted: bool,
}

/// Trust mutation request containing no executable declaration fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookMutationRequestDto {
    pub plugin_id: String,
    pub hook_hash: String,
}

/// Result of trusting or revoking one current hook declaration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookMutationResponseDto {
    pub plugin_id: String,
    pub hook_hash: String,
    pub trusted: bool,
}

/// Bounded command audit record without tool arguments, output, or error text.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookAuditEntryDto {
    pub timestamp: String,
    pub tool_name: String,
    pub duration_ms: u64,
    pub success: bool,
}

/// 外部应用探测结果的精简表示。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExternalAppsStateDto {
    /// 当前桌面平台标识。
    pub platform: Option<String>,
    /// 外部应用 ID 与是否可用的状态列表。
    pub apps: Vec<(String, bool)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopServiceCommandResponseDto {
    pub command: String,
    pub output: String,
}

impl GatewayClient {
    /// List statically audited plugin hook declarations in stable order.
    pub async fn hooks_get(&self) -> Result<Vec<HookCatalogEntryDto>, String> {
        self.get_json("/v1/hooks", &[]).await
    }

    /// Trust one current hook declaration by server-verified hash.
    pub async fn hook_trust(
        &self,
        plugin_id: &str,
        hook_hash: &str,
    ) -> Result<HookMutationResponseDto, String> {
        self.post_json(
            "/v1/hooks/trust",
            &[],
            &HookMutationRequestDto {
                plugin_id: plugin_id.to_string(),
                hook_hash: hook_hash.to_string(),
            },
        )
        .await
    }

    /// Revoke one current hook declaration by server-verified hash.
    pub async fn hook_revoke(
        &self,
        plugin_id: &str,
        hook_hash: &str,
    ) -> Result<HookMutationResponseDto, String> {
        self.post_json(
            "/v1/hooks/revoke",
            &[],
            &HookMutationRequestDto {
                plugin_id: plugin_id.to_string(),
                hook_hash: hook_hash.to_string(),
            },
        )
        .await
    }

    /// Read bounded command audit records containing only safe fields.
    pub async fn hook_audit_get(&self) -> Result<Vec<HookAuditEntryDto>, String> {
        self.get_json("/v1/hooks/audit", &[]).await
    }

    /// 读取 Skills 目录页可展示的技能列表。
    pub async fn skills_get(
        &self,
        project_path: Option<&str>,
    ) -> Result<Vec<DesktopSkillCatalogEntryDto>, String> {
        let mut query = Vec::new();
        if let Some(project_path) = project_path.map(str::trim).filter(|value| !value.is_empty()) {
            query.push(("project_path".to_string(), project_path.to_string()));
        }
        self.get_json("/v1/skills", &query).await
    }

    /// 兼容旧桌面调用名；实际请求走前端中立的 `/v1/skills`。
    pub async fn desktop_skills_get(
        &self,
        project_path: Option<&str>,
    ) -> Result<Vec<DesktopSkillCatalogEntryDto>, String> {
        self.skills_get(project_path).await
    }

    /// 读取指定技能的文档与可执行操作。
    pub async fn skill_detail_get(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
    ) -> Result<DesktopSkillDetailDto, String> {
        let mut query = vec![("skill_id".to_string(), skill_id.to_string())];
        if let Some(project_path) = project_path.map(str::trim).filter(|value| !value.is_empty()) {
            query.push(("project_path".to_string(), project_path.to_string()));
        }
        self.get_json("/v1/skills/detail", &query).await
    }

    /// 兼容旧桌面调用名；实际请求走前端中立的 `/v1/skills/detail`。
    pub async fn desktop_skill_detail_get(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
    ) -> Result<DesktopSkillDetailDto, String> {
        self.skill_detail_get(project_path, skill_id).await
    }

    /// 在当前项目下创建新的技能 scaffold。
    pub async fn skill_create(&self, project_path: &str) -> Result<String, String> {
        let response: DesktopSkillPathDto = self
            .post_json(
                "/v1/skills/create",
                &[],
                &serde_json::json!({
                    "project_path": project_path,
                }),
            )
            .await?;
        Ok(response.path)
    }

    /// 兼容旧桌面调用名；实际请求走前端中立的 `/v1/skills/create`。
    pub async fn desktop_skill_create(&self, project_path: &str) -> Result<String, String> {
        self.skill_create(project_path).await
    }

    /// 将内置技能安装到当前项目 skills 目录。
    pub async fn skill_install_builtin(
        &self,
        project_path: &str,
        skill_id: &str,
    ) -> Result<String, String> {
        let response: DesktopSkillPathDto = self
            .post_json(
                "/v1/skills/install-built-in",
                &[],
                &serde_json::json!({
                    "project_path": project_path,
                    "skill_id": skill_id,
                }),
            )
            .await?;
        Ok(response.path)
    }

    /// 兼容旧桌面调用名；实际请求走前端中立的 `/v1/skills/install-built-in`。
    pub async fn desktop_skill_install_builtin(
        &self,
        project_path: &str,
        skill_id: &str,
    ) -> Result<String, String> {
        self.skill_install_builtin(project_path, skill_id).await
    }

    /// 设置本地技能启用状态。
    pub async fn skill_set_enabled(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
        enabled: bool,
    ) -> Result<String, String> {
        let response: DesktopSkillPathDto = self
            .post_json(
                "/v1/skills/set-enabled",
                &[],
                &serde_json::json!({
                    "project_path": project_path,
                    "skill_id": skill_id,
                    "enabled": enabled,
                }),
            )
            .await?;
        Ok(response.path)
    }

    /// 兼容旧桌面调用名；实际请求走前端中立的 `/v1/skills/set-enabled`。
    pub async fn desktop_skill_set_enabled(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
        enabled: bool,
    ) -> Result<String, String> {
        self.skill_set_enabled(project_path, skill_id, enabled).await
    }

    /// 删除本地技能目录。
    pub async fn skill_delete(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
    ) -> Result<String, String> {
        let response: DesktopSkillPathDto = self
            .post_json(
                "/v1/skills/delete",
                &[],
                &serde_json::json!({
                    "project_path": project_path,
                    "skill_id": skill_id,
                }),
            )
            .await?;
        Ok(response.path)
    }

    /// 兼容旧桌面调用名；实际请求走前端中立的 `/v1/skills/delete`。
    pub async fn desktop_skill_delete(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
    ) -> Result<String, String> {
        self.skill_delete(project_path, skill_id).await
    }

    /// 读取已发现插件目录。
    pub async fn plugins_get(&self) -> Result<Vec<DesktopPluginCatalogEntryDto>, String> {
        self.get_json("/v1/plugins", &[]).await
    }

    /// 读取指定插件详情。
    pub async fn plugin_detail_get(
        &self,
        plugin_id: &str,
    ) -> Result<DesktopPluginDetailDto, String> {
        self.get_json("/v1/plugins/detail", &[("plugin_id".to_string(), plugin_id.to_string())])
            .await
    }

    /// 设置本地插件启用状态。
    pub async fn plugin_set_enabled(
        &self,
        plugin_id: &str,
        enabled: bool,
    ) -> Result<String, String> {
        let response: DesktopPluginPathDto = self
            .post_json(
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
        let response: DesktopPluginPathDto = self
            .post_json(
                "/v1/plugins/delete",
                &[],
                &serde_json::json!({
                    "plugin_id": plugin_id,
                }),
            )
            .await?;
        Ok(response.path)
    }

    /// 读取桌面端外部应用可用性状态。
    pub async fn desktop_external_apps_get(&self) -> Result<ExternalAppsStateDto, String> {
        let value: Value = self.get_json("/v1/desktop/external-apps", &[]).await?;
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
        let _: Value = self
            .post_json(
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

    /// 在系统文件管理器中定位并高亮指定路径。
    pub async fn desktop_external_path_reveal(&self, path: &str) -> Result<(), String> {
        let _: Value = self
            .post_json(
                "/v1/desktop/external-path/reveal",
                &[],
                &serde_json::json!({
                    "path": path,
                }),
            )
            .await?;
        Ok(())
    }

    /// 通过 gateway 调用本机 CLI service 生命周期命令。
    pub async fn desktop_service_command(
        &self,
        command: &str,
    ) -> Result<DesktopServiceCommandResponseDto, String> {
        self.post_json(
            &format!("/v1/desktop/service/{}", command.trim().to_ascii_lowercase()),
            &[],
            &serde_json::json!({}),
        )
        .await
    }

    /// 从全局配置中读取桌面系统设置并反序列化。
    pub async fn desktop_system_settings_get<T>(&self) -> Result<Option<T>, String>
    where
        T: DeserializeOwned,
    {
        let Some(value) = self.global_config_get_path(&["app_ui", "system_settings"]).await? else {
            return Ok(None);
        };
        serde_json::from_value(value).map(Some).map_err(|err| err.to_string())
    }

    /// 以补丁方式写入桌面系统设置。
    pub async fn desktop_system_settings_patch<T>(&self, patch: &T) -> Result<(), String>
    where
        T: Serialize,
    {
        let value = serde_json::to_value(patch).map_err(|err| err.to_string())?;
        self.global_config_patch_path(&["app_ui", "system_settings"], value).await
    }

    /// 读取桌面偏好设置。
    pub async fn desktop_preferences_get(&self) -> Result<Value, String> {
        self.get_json("/v1/desktop/preferences", &[]).await
    }

    /// 更新桌面偏好设置并返回最新值。
    pub async fn desktop_preferences_patch(&self, patch: &Value) -> Result<Value, String> {
        self.patch_json("/v1/desktop/preferences", &[], patch).await
    }

    /// 读取某类桌面工具的持久化内容。
    pub async fn desktop_tool_content_get(&self, tool_type: &str) -> Result<String, String> {
        let value: Value =
            self.get_json(&format!("/v1/desktop/tool-content/{tool_type}"), &[]).await?;
        Ok(value.get("content").and_then(Value::as_str).unwrap_or_default().to_string())
    }

    /// 写入某类桌面工具的持久化内容。
    pub async fn desktop_tool_content_put(
        &self,
        tool_type: &str,
        content: &str,
    ) -> Result<(), String> {
        let _: Value = self
            .put_json(
                &format!("/v1/desktop/tool-content/{tool_type}"),
                &[],
                &serde_json::json!({ "content": content }),
            )
            .await?;
        Ok(())
    }

    /// 读取项目级桌面偏好设置。
    pub async fn desktop_project_preferences_get(
        &self,
        project_path: &str,
    ) -> Result<Option<(String, bool, Option<String>)>, String> {
        let query = vec![("project_path".to_string(), project_path.to_string())];
        let Some(value) =
            self.get_json_with_404::<Value>("/v1/desktop/project-preferences", &query).await?
        else {
            return Ok(None);
        };
        let model = value.get("model").and_then(Value::as_str).unwrap_or_default().to_string();
        let auto_model = value.get("auto_model").and_then(Value::as_bool).unwrap_or(false);
        let acp_agent = value.get("acp_agent").and_then(Value::as_str).map(ToString::to_string);
        if model.trim().is_empty() { Ok(None) } else { Ok(Some((model, auto_model, acp_agent))) }
    }

    /// 写入项目级桌面偏好设置。
    pub async fn desktop_project_preferences_put(
        &self,
        project_path: &str,
        model: &str,
        auto_model: bool,
        acp_agent: Option<&str>,
    ) -> Result<(), String> {
        let query = vec![("project_path".to_string(), project_path.to_string())];
        let _: Value = self
            .put_json(
                "/v1/desktop/project-preferences",
                &query,
                &serde_json::json!({
                    "model": model,
                    "auto_model": auto_model,
                    "acp_agent": acp_agent,
                }),
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "desktop_settings_api_tests.rs"]
mod desktop_settings_api_tests;
