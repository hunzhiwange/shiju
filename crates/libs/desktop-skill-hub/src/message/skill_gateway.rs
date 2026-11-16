use gateway_shiju_account::ShijuCredential;
use gateway_shiju_skill::*;
pub(super) async fn gateway_shiju_skill_search_async(
    gateway: crate::GatewayFactory,
    credential: &ShijuCredential,
    query: &ShijuSkillSearchQuery,
) -> Result<ShijuSkillCursorPageDto, String> {
    let client = gateway()?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client)
        .shiju_skill_search(credential, query)
        .await
}

pub(super) async fn gateway_shiju_skill_installed_async(
    gateway: crate::GatewayFactory,
    credential: &ShijuCredential,
) -> Result<Vec<ShijuInstalledSkillDto>, String> {
    let client = gateway()?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client)
        .shiju_skill_installed(credential)
        .await
}

pub(super) async fn gateway_shiju_system_skills_async(
    gateway: crate::GatewayFactory,
    credential: &ShijuCredential,
    query: &ShijuSystemSkillQuery,
) -> Result<ShijuSkillPageDto<ShijuSystemSkillDto>, String> {
    let client = gateway()?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client)
        .shiju_system_skills(credential, query)
        .await
}

pub(super) async fn gateway_shiju_maintained_skills_async(
    gateway: crate::GatewayFactory,
    credential: &ShijuCredential,
    query: &ShijuMaintainedSkillQuery,
) -> Result<ShijuSkillPageDto<ShijuTenantSkillDto>, String> {
    let client = gateway()?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client)
        .shiju_maintained_skills(credential, query)
        .await
}

pub(super) async fn gateway_shiju_skill_tags_async(
    gateway: crate::GatewayFactory,
    credential: &ShijuCredential,
) -> Result<ShijuSkillTagCatalogDto, String> {
    let client = gateway()?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client).shiju_skill_tags(credential).await
}

pub(super) async fn gateway_shiju_tenant_skill_tags_async(
    gateway: crate::GatewayFactory,
    credential: &ShijuCredential,
) -> Result<ShijuSkillTagCatalogDto, String> {
    let client = gateway()?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client)
        .shiju_tenant_skill_tags(credential)
        .await
}

pub(super) async fn gateway_shiju_skill_detail_async(
    gateway: crate::GatewayFactory,
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<ShijuSkillDetailDto, String> {
    let client = gateway()?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client)
        .shiju_skill_detail(credential, skill_id)
        .await
}

#[allow(dead_code)]
pub(super) async fn gateway_shiju_skill_readme_async(
    gateway: crate::GatewayFactory,
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<ShijuSkillReadmeDto, String> {
    let client = gateway()?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client)
        .shiju_skill_readme(credential, skill_id)
        .await
}

pub(super) async fn gateway_shiju_skill_versions_async(
    gateway: crate::GatewayFactory,
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<Vec<ShijuSkillVersionDto>, String> {
    let client = gateway()?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client)
        .shiju_skill_versions(credential, skill_id)
        .await
}

pub(super) async fn gateway_shiju_skill_remote_install_async(
    gateway: crate::GatewayFactory,
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<(), String> {
    let client = gateway()?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client)
        .shiju_skill_remote_install(credential, skill_id)
        .await
}

#[allow(dead_code)]
pub(super) async fn gateway_shiju_skill_remote_install_version_async(
    gateway: crate::GatewayFactory,
    credential: &ShijuCredential,
    skill_id: u64,
    version_id: u64,
) -> Result<(), String> {
    let client = gateway()?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client)
        .shiju_skill_remote_install_version(credential, skill_id, version_id)
        .await
}

pub(super) async fn gateway_shiju_skill_remote_uninstall_async(
    gateway: crate::GatewayFactory,
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<(), String> {
    let client = gateway()?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client)
        .shiju_skill_remote_uninstall(credential, skill_id)
        .await
}
