//! 技能、插件和外部应用网关操作。
use super::*;

pub async fn gateway_external_apps_async(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<ExternalAppsStateDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_external_apps_get().await
}

pub async fn gateway_external_open_async(endpoint: gateway_endpoint::GatewayEndpoint, path: &str, target: &str) -> Result<(), String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_external_app_open(path, target).await
}

pub async fn gateway_external_reveal_async(endpoint: gateway_endpoint::GatewayEndpoint, path: &str) -> Result<(), String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_external_path_reveal(path).await
}

pub async fn gateway_skills_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: Option<&str>,
) -> Result<Vec<DesktopSkillCatalogEntryDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).skills_get(project_path).await
}

pub async fn gateway_skill_detail_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: Option<&str>,
    skill_id: &str,
) -> Result<DesktopSkillDetailDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).skill_detail_get(project_path, skill_id).await
}

pub async fn gateway_skill_create_async(endpoint: gateway_endpoint::GatewayEndpoint, project_path: &str) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).skill_create(project_path).await
}

pub async fn gateway_skill_install_builtin_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: &str,
    skill_id: &str,
) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).skill_install_builtin(project_path, skill_id).await
}

pub async fn gateway_skill_set_enabled_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: Option<&str>,
    skill_id: &str,
    enabled: bool,
) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).skill_set_enabled(project_path, skill_id, enabled).await
}

pub async fn gateway_skill_delete_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: Option<&str>,
    skill_id: &str,
) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).skill_delete(project_path, skill_id).await
}

pub async fn gateway_skill_market_sources_async(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<Vec<SkillMarketSourceDto>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_sources().await
}

pub async fn gateway_skill_market_recommended_async(endpoint: gateway_endpoint::GatewayEndpoint,
    source: &str,
) -> Result<Vec<SkillMarketSkillDto>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_recommended(source).await
}

pub async fn gateway_skill_market_skills_async(endpoint: gateway_endpoint::GatewayEndpoint,
    source: &str,
    query: &SkillMarketSkillQuery,
) -> Result<SkillMarketSkillPageDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_skills(source, query).await
}

pub async fn gateway_skill_market_categories_async(endpoint: gateway_endpoint::GatewayEndpoint,
    source: &str,
) -> Result<Vec<SkillMarketCategoryDto>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_categories(source).await
}

pub async fn gateway_skill_market_skill_detail_async(endpoint: gateway_endpoint::GatewayEndpoint,
    locator: &SkillMarketSkillLocatorDto,
) -> Result<SkillMarketSkillDetailDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_skill_detail(locator).await
}

pub async fn gateway_skill_market_skill_file_async(endpoint: gateway_endpoint::GatewayEndpoint,
    locator: &SkillMarketSkillLocatorDto,
) -> Result<SkillMarketSkillFileDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_skill_file(locator).await
}

pub async fn gateway_skill_market_install_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: &str,
    coordinate: &SkillMarketCoordinateDto,
) -> Result<String, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_install(project_path, coordinate).await
}

pub async fn gateway_skill_market_install_to_targets_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: Option<&str>,
    coordinate: &SkillMarketCoordinateDto,
    installations: &[SkillMarketInstallSelectionDto],
) -> Result<Vec<String>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_install_to_targets(project_path, coordinate, installations).await
}

pub async fn gateway_skill_market_skill_sets_async(endpoint: gateway_endpoint::GatewayEndpoint,
    source: &str,
    query: &SkillMarketSkillQuery,
) -> Result<SkillMarketSkillSetPageDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_skill_sets(source, query).await
}

pub async fn gateway_skill_market_skill_set_detail_async(endpoint: gateway_endpoint::GatewayEndpoint,
    source: &str,
    slug: &str,
) -> Result<SkillMarketSkillSetDetailDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_skill_set_detail(source, slug).await
}

pub async fn gateway_skill_market_skill_set_install_async(endpoint: gateway_endpoint::GatewayEndpoint,
    project_path: &str,
    source: &str,
    slug: &str,
) -> Result<Vec<String>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone());
    client.skill_market_skill_set_install(project_path, source, slug).await
}

pub async fn gateway_plugins_async(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<Vec<DesktopPluginCatalogEntryDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugins_status_get().await
}

pub async fn gateway_hooks_async(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<Vec<HookCatalogEntryDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).hooks_get().await
}

pub async fn gateway_hook_trust_async(endpoint: gateway_endpoint::GatewayEndpoint,
    plugin_id: &str,
    hook_hash: &str,
) -> Result<HookMutationResponseDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).hook_trust(plugin_id, hook_hash).await
}

pub async fn gateway_hook_revoke_async(endpoint: gateway_endpoint::GatewayEndpoint,
    plugin_id: &str,
    hook_hash: &str,
) -> Result<HookMutationResponseDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).hook_revoke(plugin_id, hook_hash).await
}

pub async fn gateway_hook_audit_async(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<Vec<HookAuditEntryDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).hook_audit_get().await
}

pub async fn gateway_plugin_set_enabled_async(endpoint: gateway_endpoint::GatewayEndpoint,
    plugin_id: &str,
    enabled: bool,
) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugin_set_enabled(plugin_id, enabled).await
}

pub async fn gateway_plugin_delete_async(endpoint: gateway_endpoint::GatewayEndpoint, plugin_id: &str) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugin_delete(plugin_id).await
}

pub async fn gateway_plugin_prepare_async(endpoint: gateway_endpoint::GatewayEndpoint,
    source: String,
    update_id: Option<String>,
) -> Result<gateway_desktop_settings::DesktopPluginPreviewDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    match update_id {
        Some(id) => {
            gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugin_prepare_update(&id, (!source.trim().is_empty()).then_some(source.trim()))
                .await
        }
        None => gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugin_prepare_install(source.trim()).await,
    }
}

pub async fn gateway_plugin_activate_async(endpoint: gateway_endpoint::GatewayEndpoint, token: String, update: bool) -> Result<String, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    if update {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugin_update(&token, true).await
    } else {
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).plugin_install(&token, true).await
    }
}

pub async fn gateway_plugin_discard_async(endpoint: gateway_endpoint::GatewayEndpoint, token: String) -> Result<(), String> {
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone()).plugin_cancel(&token).await
}
