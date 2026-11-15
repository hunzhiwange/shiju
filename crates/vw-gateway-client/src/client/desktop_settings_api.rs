//! 桌面设置兼容门面。
use super::GatewayClient;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use vw_api_file::file::FileNodeKind;
pub use gateway_desktop_settings::*;

impl GatewayClient {
    /// 在网关主机安装浏览器依赖，沿用认证上下文并允许下载时间。
    pub async fn browser_install(&self, backend: &str) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .browser_install(backend).await
    }

    /// 安装可能需要下载较大的包，使用独立请求超时。
    pub async fn acp_install(&self, agent: &str) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .acp_install(agent).await
    }

    pub async fn acp_diagnostic(&self, id: &str, config: &Value) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .acp_diagnostic(id, config).await
    }

    pub async fn acp_diagnostic_cancel(&self, id: &str) -> Result<bool, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .acp_diagnostic_cancel(id).await
    }

    /// List statically audited plugin hook declarations in stable order.
    pub async fn hooks_get(&self) -> Result<Vec<HookCatalogEntryDto>, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .hooks_get().await
    }

    /// Trust one current hook declaration by server-verified hash.
    pub async fn hook_trust(
        &self,
        plugin_id: &str,
        hook_hash: &str,
    ) -> Result<HookMutationResponseDto, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .hook_trust(plugin_id, hook_hash).await
    }

    /// Revoke one current hook declaration by server-verified hash.
    pub async fn hook_revoke(
        &self,
        plugin_id: &str,
        hook_hash: &str,
    ) -> Result<HookMutationResponseDto, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .hook_revoke(plugin_id, hook_hash).await
    }

    /// Read bounded command audit records containing only safe fields.
    pub async fn hook_audit_get(&self) -> Result<Vec<HookAuditEntryDto>, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .hook_audit_get().await
    }

    /// 读取 Skills 目录页可展示的技能列表。
    pub async fn skills_get(
        &self,
        project_path: Option<&str>,
    ) -> Result<Vec<DesktopSkillCatalogEntryDto>, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .skills_get(project_path).await
    }

    /// 读取 desktop 专用技能目录。
    pub async fn desktop_skills_get(
        &self,
        project_path: Option<&str>,
    ) -> Result<Vec<DesktopSkillCatalogEntryDto>, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_skills_get(project_path).await
    }

    /// 读取指定技能的文档与可执行操作。
    pub async fn skill_detail_get(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
    ) -> Result<DesktopSkillDetailDto, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .skill_detail_get(project_path, skill_id).await
    }

    /// 读取 desktop 专用技能详情。
    pub async fn desktop_skill_detail_get(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
    ) -> Result<DesktopSkillDetailDto, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_skill_detail_get(project_path, skill_id).await
    }

    /// 在当前项目下创建新的技能 scaffold。
    pub async fn skill_create(&self, project_path: &str) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .skill_create(project_path).await
    }

    /// 创建 desktop 专用技能 scaffold。
    pub async fn desktop_skill_create(&self, project_path: &str) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_skill_create(project_path).await
    }

    /// 将内置技能安装到当前项目 skills 目录。
    pub async fn skill_install_builtin(
        &self,
        project_path: &str,
        skill_id: &str,
    ) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .skill_install_builtin(project_path, skill_id).await
    }

    /// 安装 desktop 专用内置技能。
    pub async fn desktop_skill_install_builtin(
        &self,
        project_path: &str,
        skill_id: &str,
    ) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_skill_install_builtin(project_path, skill_id).await
    }

    /// 设置本地技能启用状态。
    pub async fn skill_set_enabled(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
        enabled: bool,
    ) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .skill_set_enabled(project_path, skill_id, enabled).await
    }

    /// 设置 desktop 专用技能启用状态。
    pub async fn desktop_skill_set_enabled(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
        enabled: bool,
    ) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_skill_set_enabled(project_path, skill_id, enabled).await
    }

    /// 删除本地技能目录。
    pub async fn skill_delete(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
    ) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .skill_delete(project_path, skill_id).await
    }

    /// 删除 desktop 专用技能目录。
    pub async fn desktop_skill_delete(
        &self,
        project_path: Option<&str>,
        skill_id: &str,
    ) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_skill_delete(project_path, skill_id).await
    }

    /// 读取已发现插件目录。
    pub async fn plugins_get(&self) -> Result<Vec<DesktopPluginCatalogEntryDto>, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .plugins_get().await
    }

    /// Read state-bound plugin diagnostics from the running authenticated gateway.
    pub async fn plugins_status_get(&self) -> Result<Vec<DesktopPluginCatalogEntryDto>, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .plugins_status_get().await
    }

    /// 读取指定插件详情。
    pub async fn plugin_detail_get(
        &self,
        plugin_id: &str,
    ) -> Result<DesktopPluginDetailDto, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .plugin_detail_get(plugin_id).await
    }

    /// Prepare and audit a source without publishing it to the plugin store.
    pub async fn plugin_prepare_install(
        &self,
        source: &str,
    ) -> Result<DesktopPluginPreviewDto, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .plugin_prepare_install(source).await
    }

    pub async fn plugin_install(&self, token: &str, approved: bool) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .plugin_install(token, approved).await
    }

    pub async fn plugin_prepare_update(
        &self,
        plugin_id: &str,
        source: Option<&str>,
    ) -> Result<DesktopPluginPreviewDto, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .plugin_prepare_update(plugin_id, source).await
    }

    pub async fn plugin_update(&self, token: &str, approved: bool) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .plugin_update(token, approved).await
    }

    /// Discard a pending preview and release its private staging files.
    pub async fn plugin_cancel(&self, token: &str) -> Result<(), String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .plugin_cancel(token).await
    }

    /// 设置本地插件启用状态。
    pub async fn plugin_set_enabled(
        &self,
        plugin_id: &str,
        enabled: bool,
    ) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .plugin_set_enabled(plugin_id, enabled).await
    }

    /// 删除本地插件目录。
    pub async fn plugin_delete(&self, plugin_id: &str) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .plugin_delete(plugin_id).await
    }

    /// 读取桌面端外部应用可用性状态。
    pub async fn desktop_external_apps_get(&self) -> Result<ExternalAppsStateDto, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_external_apps_get().await
    }

    /// 调用指定外部应用打开目标路径。
    pub async fn desktop_external_app_open(&self, path: &str, target: &str) -> Result<(), String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_external_app_open(path, target).await
    }

    /// 判断网关主机上的绝对路径指向文件还是目录。
    pub async fn desktop_external_path_kind(&self, path: &str) -> Result<FileNodeKind, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_external_path_kind(path).await
    }

    /// 在系统文件管理器中定位并高亮指定路径。
    pub async fn desktop_external_path_reveal(&self, path: &str) -> Result<(), String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_external_path_reveal(path).await
    }

    /// 通过 gateway 调用本机 CLI service 生命周期命令。
    pub async fn desktop_service_command(
        &self,
        command: &str,
    ) -> Result<DesktopServiceCommandResponseDto, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_service_command(command).await
    }

    /// 执行经过类型约束的 service 生命周期命令。
    pub async fn desktop_service_command_typed(
        &self,
        command: DesktopServiceCommand,
    ) -> Result<DesktopServiceCommandResponseDto, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_service_command_typed(command).await
    }

    /// 从全局配置中读取桌面系统设置并反序列化。
    pub async fn desktop_system_settings_get<T>(&self) -> Result<Option<T>, String>
    where
        T: DeserializeOwned,
    {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_system_settings_get::<T>().await
    }

    /// 以补丁方式写入桌面系统设置。
    pub async fn desktop_system_settings_patch<T>(&self, patch: &T) -> Result<(), String>
    where
        T: Serialize,
    {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_system_settings_patch::<T>(patch).await
    }

    /// 读取桌面偏好设置。
    pub async fn desktop_preferences_get(&self) -> Result<Value, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_preferences_get().await
    }

    /// 更新桌面偏好设置并返回最新值。
    pub async fn desktop_preferences_patch(&self, patch: &Value) -> Result<Value, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_preferences_patch(patch).await
    }

    /// 读取某类桌面工具的持久化内容。
    pub async fn desktop_tool_content_get(&self, tool_type: &str) -> Result<String, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_tool_content_get(tool_type).await
    }

    /// 写入某类桌面工具的持久化内容。
    pub async fn desktop_tool_content_put(
        &self,
        tool_type: &str,
        content: &str,
    ) -> Result<(), String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_tool_content_put(tool_type, content).await
    }

    /// 读取项目级桌面偏好设置。
    pub async fn desktop_project_preferences_get(
        &self,
        project_path: &str,
    ) -> Result<Option<(String, bool, Option<String>)>, String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_project_preferences_get(project_path).await
    }

    /// 写入项目级桌面偏好设置。
    pub async fn desktop_project_preferences_put(
        &self,
        project_path: &str,
        model: &str,
        auto_model: bool,
        acp_agent: Option<&str>,
    ) -> Result<(), String> {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(self.transport.clone())
            .desktop_project_preferences_put(project_path, model, auto_model, acp_agent).await
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "desktop_settings_api_tests.rs"]
mod desktop_settings_api_tests;
