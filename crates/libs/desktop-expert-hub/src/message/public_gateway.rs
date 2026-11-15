use gateway_skill_market::*;
use gateway_desktop_settings::DesktopSkillCatalogEntryDto;
pub(super) async fn gateway_skill_delete_async(gateway: crate::GatewayFactory,
    project_path: Option<&str>,
    skill_id: &str,
) -> Result<String, String> {
    let client = gateway()?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client).skill_delete(project_path, skill_id).await
}
pub(super) async fn gateway_skill_market_categories_async(gateway: crate::GatewayFactory,
    source: &str,
) -> Result<Vec<SkillMarketCategoryDto>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(gateway()?);
    client.skill_market_categories(source).await
}
pub(super) async fn gateway_skill_market_install_to_targets_async(gateway: crate::GatewayFactory,
    project_path: Option<&str>,
    coordinate: &SkillMarketCoordinateDto,
    installations: &[SkillMarketInstallSelectionDto],
) -> Result<Vec<String>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(gateway()?);
    client.skill_market_install_to_targets(project_path, coordinate, installations).await
}
pub(super) async fn gateway_skill_market_recommended_async(gateway: crate::GatewayFactory,
    source: &str,
) -> Result<Vec<SkillMarketSkillDto>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(gateway()?);
    client.skill_market_recommended(source).await
}
pub(super) async fn gateway_skill_market_skill_detail_async(gateway: crate::GatewayFactory,
    locator: &SkillMarketSkillLocatorDto,
) -> Result<SkillMarketSkillDetailDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(gateway()?);
    client.skill_market_skill_detail(locator).await
}
pub(super) async fn gateway_skill_market_skill_file_async(gateway: crate::GatewayFactory,
    locator: &SkillMarketSkillLocatorDto,
) -> Result<SkillMarketSkillFileDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(gateway()?);
    client.skill_market_skill_file(locator).await
}
pub(super) async fn gateway_skill_market_skill_set_detail_async(gateway: crate::GatewayFactory,
    source: &str,
    slug: &str,
) -> Result<SkillMarketSkillSetDetailDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(gateway()?);
    client.skill_market_skill_set_detail(source, slug).await
}
pub(super) async fn gateway_skill_market_skill_set_install_async(gateway: crate::GatewayFactory,
    project_path: &str,
    source: &str,
    slug: &str,
) -> Result<Vec<String>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(gateway()?);
    client.skill_market_skill_set_install(project_path, source, slug).await
}
pub(super) async fn gateway_skill_market_skill_sets_async(gateway: crate::GatewayFactory,
    source: &str,
    query: &SkillMarketSkillQuery,
) -> Result<SkillMarketSkillSetPageDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(gateway()?);
    client.skill_market_skill_sets(source, query).await
}
pub(super) async fn gateway_skill_market_skills_async(gateway: crate::GatewayFactory,
    source: &str,
    query: &SkillMarketSkillQuery,
) -> Result<SkillMarketSkillPageDto, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(gateway()?);
    client.skill_market_skills(source, query).await
}
pub(super) async fn gateway_skill_market_sources_async(gateway: crate::GatewayFactory, ) -> Result<Vec<SkillMarketSourceDto>, String> {
    let client = gateway_skill_market::GatewaySkillMarketClient::new(gateway()?);
    client.skill_market_sources().await
}
pub(super) async fn gateway_skill_set_enabled_async(gateway: crate::GatewayFactory,
    project_path: Option<&str>,
    skill_id: &str,
    enabled: bool,
) -> Result<String, String> {
    let client = gateway()?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client).skill_set_enabled(project_path, skill_id, enabled).await
}
pub(super) async fn gateway_skills_async(gateway: crate::GatewayFactory,
    project_path: Option<&str>,
) -> Result<Vec<DesktopSkillCatalogEntryDto>, String> {
    let client = gateway()?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client).skills_get(project_path).await
}
