//! 技能市场兼容门面。
use super::GatewayClient;
pub use gateway_skill_market::*;

impl GatewayClient {
    pub async fn skill_market_sources(&self) -> Result<Vec<SkillMarketSourceDto>, String> {
        gateway_skill_market::GatewaySkillMarketClient::new(self.transport.clone())
            .skill_market_sources().await
    }
    pub async fn skill_market_recommended(
        &self,
        source: &str,
    ) -> Result<Vec<SkillMarketSkillDto>, String> {
        gateway_skill_market::GatewaySkillMarketClient::new(self.transport.clone())
            .skill_market_recommended(source).await
    }
    pub async fn skill_market_skills(
        &self,
        source: &str,
        query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillPageDto, String> {
        gateway_skill_market::GatewaySkillMarketClient::new(self.transport.clone())
            .skill_market_skills(source, query).await
    }
    pub async fn skill_market_categories(
        &self,
        source: &str,
    ) -> Result<Vec<SkillMarketCategoryDto>, String> {
        gateway_skill_market::GatewaySkillMarketClient::new(self.transport.clone())
            .skill_market_categories(source).await
    }
    pub async fn skill_market_skill_detail(
        &self,
        locator: &SkillMarketSkillLocatorDto,
    ) -> Result<SkillMarketSkillDetailDto, String> {
        gateway_skill_market::GatewaySkillMarketClient::new(self.transport.clone())
            .skill_market_skill_detail(locator).await
    }
    pub async fn skill_market_skill_file(
        &self,
        locator: &SkillMarketSkillLocatorDto,
    ) -> Result<SkillMarketSkillFileDto, String> {
        gateway_skill_market::GatewaySkillMarketClient::new(self.transport.clone())
            .skill_market_skill_file(locator).await
    }
    pub async fn skill_market_install(
        &self,
        project_path: &str,
        coordinate: &SkillMarketCoordinateDto,
    ) -> Result<String, String> {
        gateway_skill_market::GatewaySkillMarketClient::new(self.transport.clone())
            .skill_market_install(project_path, coordinate).await
    }
    pub async fn skill_market_install_to_targets(
        &self,
        project_path: Option<&str>,
        coordinate: &SkillMarketCoordinateDto,
        installations: &[SkillMarketInstallSelectionDto],
    ) -> Result<Vec<String>, String> {
        gateway_skill_market::GatewaySkillMarketClient::new(self.transport.clone())
            .skill_market_install_to_targets(project_path, coordinate, installations).await
    }
    pub async fn skill_market_skill_sets(
        &self,
        source: &str,
        query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillSetPageDto, String> {
        gateway_skill_market::GatewaySkillMarketClient::new(self.transport.clone())
            .skill_market_skill_sets(source, query).await
    }
    pub async fn skill_market_skill_set_detail(
        &self,
        source: &str,
        slug: &str,
    ) -> Result<SkillMarketSkillSetDetailDto, String> {
        gateway_skill_market::GatewaySkillMarketClient::new(self.transport.clone())
            .skill_market_skill_set_detail(source, slug).await
    }
    pub async fn skill_market_skill_set_install(
        &self,
        project_path: &str,
        source: &str,
        slug: &str,
    ) -> Result<Vec<String>, String> {
        gateway_skill_market::GatewaySkillMarketClient::new(self.transport.clone())
            .skill_market_skill_set_install(project_path, source, slug).await
    }
}
