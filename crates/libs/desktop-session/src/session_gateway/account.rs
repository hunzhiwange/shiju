//! 识矩账号、应用与技能请求。
use super::*;

pub async fn gateway_shiju_account_login_async(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<ShijuLoginChallengeDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_account::GatewayShijuAccountClient::new(client.transport().clone()).shiju_account_login().await
}

pub async fn gateway_shiju_account_login_poll_async(endpoint: gateway_endpoint::GatewayEndpoint,
    state: &str,
) -> Result<ShijuLoginPollDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_account::GatewayShijuAccountClient::new(client.transport().clone()).shiju_account_login_poll(state).await
}

pub async fn gateway_shiju_account_profile_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
) -> Result<ShijuAccountProfileDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_account::GatewayShijuAccountClient::new(client.transport().clone()).shiju_account_profile(credential).await
}

pub async fn gateway_shiju_account_monthly_quota_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
) -> Result<gateway_shiju_account::ShijuMonthlyQuotaDto, String> {
    gateway_shiju_account::GatewayShijuAccountClient::new(vw_gateway_client::GatewayClient::new(endpoint.clone())?.transport().clone()).shiju_account_monthly_quota(credential).await
}

pub async fn gateway_shiju_account_me_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
) -> Result<ShijuAccountMeDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_account::GatewayShijuAccountClient::new(client.transport().clone()).shiju_account_me(credential).await
}

pub async fn gateway_shiju_account_update_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    request: &ShijuAccountUpdateRequest,
) -> Result<ShijuAccountUpdateResponseDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_account::GatewayShijuAccountClient::new(client.transport().clone()).shiju_account_update(credential, request).await
}

pub async fn gateway_shiju_account_password_update_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    request: &ShijuAccountPasswordUpdateRequest,
) -> Result<(), String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_account::GatewayShijuAccountClient::new(client.transport().clone()).shiju_account_password_update(credential, request).await
}

pub async fn gateway_shiju_app_catalog_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
) -> Result<ShijuAppCatalogDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_app::GatewayShijuAppClient::new(client.transport().clone()).shiju_app_catalog(credential).await
}

pub async fn gateway_shiju_apps_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
) -> Result<ShijuAppListDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_app::GatewayShijuAppClient::new(client.transport().clone()).shiju_apps(credential).await
}

pub async fn gateway_shiju_app_api_key_binding_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    installation_id: &str,
) -> Result<ShijuAppApiKeyBindingDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_app::GatewayShijuAppClient::new(client.transport().clone()).shiju_app_api_key_binding(credential, installation_id).await
}

pub async fn gateway_shiju_app_api_keys_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
) -> Result<ShijuIamApiKeyListDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_app::GatewayShijuAppClient::new(client.transport().clone()).shiju_app_api_keys(credential).await
}

pub async fn gateway_shiju_app_save_api_key_binding_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    installation_id: &str,
    request: &ShijuAppApiKeyBindingSaveRequest,
) -> Result<(), String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_app::GatewayShijuAppClient::new(client.transport().clone()).shiju_app_save_api_key_binding(credential, installation_id, request).await
}

pub async fn gateway_shiju_app_prepare_launch_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    installation_id: &str,
) -> Result<ShijuAppLaunchDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_app::GatewayShijuAppClient::new(client.transport().clone()).shiju_app_prepare_launch(credential, installation_id).await
}

pub async fn gateway_shiju_skill_search_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    query: &ShijuSkillSearchQuery,
) -> Result<ShijuSkillCursorPageDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client.transport().clone()).shiju_skill_search(credential, query).await
}

pub async fn gateway_shiju_skill_installed_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
) -> Result<Vec<ShijuInstalledSkillDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client.transport().clone()).shiju_skill_installed(credential).await
}

pub async fn gateway_shiju_system_skills_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    query: &ShijuSystemSkillQuery,
) -> Result<ShijuSkillPageDto<ShijuSystemSkillDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client.transport().clone()).shiju_system_skills(credential, query).await
}

pub async fn gateway_shiju_maintained_skills_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    query: &ShijuMaintainedSkillQuery,
) -> Result<ShijuSkillPageDto<ShijuTenantSkillDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client.transport().clone()).shiju_maintained_skills(credential, query).await
}

pub async fn gateway_shiju_skill_tags_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
) -> Result<ShijuSkillTagCatalogDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client.transport().clone()).shiju_skill_tags(credential).await
}

pub async fn gateway_shiju_tenant_skill_tags_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
) -> Result<ShijuSkillTagCatalogDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client.transport().clone()).shiju_tenant_skill_tags(credential).await
}

pub async fn gateway_shiju_skill_detail_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<ShijuSkillDetailDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client.transport().clone()).shiju_skill_detail(credential, skill_id).await
}

pub async fn gateway_shiju_skill_readme_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<ShijuSkillReadmeDto, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client.transport().clone()).shiju_skill_readme(credential, skill_id).await
}

pub async fn gateway_shiju_skill_versions_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<Vec<ShijuSkillVersionDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client.transport().clone()).shiju_skill_versions(credential, skill_id).await
}

pub async fn gateway_shiju_skill_remote_install_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<(), String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client.transport().clone()).shiju_skill_remote_install(credential, skill_id).await
}

pub async fn gateway_shiju_skill_remote_install_version_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    skill_id: u64,
    version_id: u64,
) -> Result<(), String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client.transport().clone()).shiju_skill_remote_install_version(credential, skill_id, version_id).await
}

pub async fn gateway_shiju_skill_remote_uninstall_async(endpoint: gateway_endpoint::GatewayEndpoint,
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<(), String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_shiju_skill::GatewayShijuSkillClient::new(client.transport().clone()).shiju_skill_remote_uninstall(credential, skill_id).await
}
