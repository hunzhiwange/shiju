//! 识矩私有技能兼容门面。
use super::{GatewayClient, ShijuCredential};
pub use gateway_shiju_skill::*;

impl GatewayClient {
    pub async fn shiju_skill_tags(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuSkillTagCatalogDto, String> {
        gateway_shiju_skill::GatewayShijuSkillClient::new(self.transport.clone())
            .shiju_skill_tags(credential).await
    }
    pub async fn shiju_tenant_skill_tags(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuSkillTagCatalogDto, String> {
        gateway_shiju_skill::GatewayShijuSkillClient::new(self.transport.clone())
            .shiju_tenant_skill_tags(credential).await
    }
    pub async fn shiju_skill_search(
        &self,
        credential: &ShijuCredential,
        query: &ShijuSkillSearchQuery,
    ) -> Result<ShijuSkillCursorPageDto, String> {
        gateway_shiju_skill::GatewayShijuSkillClient::new(self.transport.clone())
            .shiju_skill_search(credential, query).await
    }
    pub async fn shiju_skill_installed(
        &self,
        credential: &ShijuCredential,
    ) -> Result<Vec<ShijuInstalledSkillDto>, String> {
        gateway_shiju_skill::GatewayShijuSkillClient::new(self.transport.clone())
            .shiju_skill_installed(credential).await
    }
    pub async fn shiju_system_skills(
        &self,
        credential: &ShijuCredential,
        query: &ShijuSystemSkillQuery,
    ) -> Result<ShijuSkillPageDto<ShijuSystemSkillDto>, String> {
        gateway_shiju_skill::GatewayShijuSkillClient::new(self.transport.clone())
            .shiju_system_skills(credential, query).await
    }
    pub async fn shiju_maintained_skills(
        &self,
        credential: &ShijuCredential,
        query: &ShijuMaintainedSkillQuery,
    ) -> Result<ShijuSkillPageDto<ShijuTenantSkillDto>, String> {
        gateway_shiju_skill::GatewayShijuSkillClient::new(self.transport.clone())
            .shiju_maintained_skills(credential, query).await
    }
    pub async fn shiju_skill_detail(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<ShijuSkillDetailDto, String> {
        gateway_shiju_skill::GatewayShijuSkillClient::new(self.transport.clone())
            .shiju_skill_detail(credential, skill_id).await
    }
    pub async fn shiju_skill_readme(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<ShijuSkillReadmeDto, String> {
        gateway_shiju_skill::GatewayShijuSkillClient::new(self.transport.clone())
            .shiju_skill_readme(credential, skill_id).await
    }
    pub async fn shiju_skill_versions(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<Vec<ShijuSkillVersionDto>, String> {
        gateway_shiju_skill::GatewayShijuSkillClient::new(self.transport.clone())
            .shiju_skill_versions(credential, skill_id).await
    }
    pub async fn shiju_skill_remote_install(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<(), String> {
        gateway_shiju_skill::GatewayShijuSkillClient::new(self.transport.clone())
            .shiju_skill_remote_install(credential, skill_id).await
    }
    pub async fn shiju_skill_remote_install_version(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
        version_id: u64,
    ) -> Result<(), String> {
        gateway_shiju_skill::GatewayShijuSkillClient::new(self.transport.clone())
            .shiju_skill_remote_install_version(credential, skill_id, version_id).await
    }
    pub async fn shiju_skill_remote_uninstall(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<(), String> {
        gateway_shiju_skill::GatewayShijuSkillClient::new(self.transport.clone())
            .shiju_skill_remote_uninstall(credential, skill_id).await
    }
}
