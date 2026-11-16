use crate::provider_state::SHIJU_OFFICIAL_PROVIDER_ID;
use auth_file_store as auth;
use desktop_i18n::settings_models::messages::Feedback;
use gateway_shiju_account::ShijuModelTokenDto;
pub fn save_token(path: &std::path::Path, token: &ShijuModelTokenDto) -> Result<(), Feedback> {
    if token.endpoint_url != "https://router.shiju.cloud/v1"
        || !token.key.expose().bytes().all(|byte| (33..=126).contains(&byte))
    {
        return Err(Feedback::owned("识矩模型授权响应无效", vec![]));
    }
    let info = auth_types::Info::Api(auth_types::ApiInfo { key: token.key.expose().to_string() });
    auth::set_to(path, SHIJU_OFFICIAL_PROVIDER_ID, &info)
        .map_err(|_| Feedback::owned("识矩模型凭据保存失败，请检查认证文件权限", vec![]))
}

pub async fn configure(
    config: desktop_config::ConfigAccess,
    token: ShijuModelTokenDto,
) -> Result<(), String> {
    config
        .patch_full_agent_config_async(serde_json::json!({
            "providers": {"shiju": {"name": "识矩", "api": token.endpoint_url,
            "adapter": "openai-compatible", "env": []}}
        }))
        .await
        .map_err(desktop_config::config_gateway::server_config_unreachable_error)?;
    Ok(())
}
#[cfg(test)]
mod tests;

/// Revoke the previous account's model credential before accepting another account.
pub fn reset_credentials(
    path: &std::path::Path,
    state: &mut crate::provider_state::ProviderSettingsState,
) -> Result<(), Feedback> {
    if auth::get_from(path, SHIJU_OFFICIAL_PROVIDER_ID).is_some() {
        auth::remove_from(path, SHIJU_OFFICIAL_PROVIDER_ID)
            .map_err(|_| Feedback::owned("无法清理识矩模型凭据，请检查认证文件权限", vec![]))?;
    }
    for provider in &mut state.providers {
        if provider.id == SHIJU_OFFICIAL_PROVIDER_ID {
            provider.connected = false;
        }
    }
    state.connect_modal = None;
    Ok(())
}

/// Preserve the current generation across delayed model-credential retries.
pub fn retry_task(generation: u64) -> iced::Task<u64> {
    iced::Task::perform(
        async { tokio::time::sleep(std::time::Duration::from_secs(10)).await },
        move |()| generation,
    )
}
