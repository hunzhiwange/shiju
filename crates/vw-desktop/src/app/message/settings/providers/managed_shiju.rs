//! 将账号登录态连接到识矩模型认证；秘密只在当前账号的结果被接受后写入。

use crate::app::config;
use crate::app::message::account::AccountMessage;
use crate::app::state::SHIJU_OFFICIAL_PROVIDER_ID;
use crate::app::{App, Message};
use iced::Task;
use std::time::Duration;
use vw_gateway_client::ShijuModelTokenDto;
use vw_shared::auth;

fn save_token(path: &std::path::Path, token: &ShijuModelTokenDto) -> Result<(), &'static str> {
    if token.endpoint_url != "https://router.shiju.cloud/v1"
        || !token.key.expose().bytes().all(|byte| (33..=126).contains(&byte))
    {
        return Err("识矩模型授权响应无效");
    }
    let info = auth::Info::Api(auth::ApiInfo { key: token.key.expose().to_string() });
    auth::store::set_to(path, SHIJU_OFFICIAL_PROVIDER_ID, &info)
        .map_err(|_| "识矩模型凭据保存失败，请检查认证文件权限")
}

/// 登录、启动恢复、切换账号及退出时统一撤销旧模型凭据。
pub(crate) fn sync(app: &mut App) -> Task<Message> {
    let generation = app.account.begin_model_sync();
    Task::done(Message::Account(AccountMessage::ResetModels { generation }))
}

fn retry_task(generation: u64) -> Task<Message> {
    // 沿用当前代次；切换账号、退出或手动重试会使旧定时任务失效。
    Task::perform(async { tokio::time::sleep(Duration::from_secs(10)).await }, move |()| {
        Message::Account(AccountMessage::ResetModels { generation })
    })
}

pub(crate) fn reset(app: &mut App, generation: u64) -> Task<Message> {
    if !app.account.model_sync_is_current(generation) {
        return Task::none();
    }
    let path = super::models::auth_filepath();
    if auth::store::get_from(&path, SHIJU_OFFICIAL_PROVIDER_ID).is_some() {
        if auth::store::remove_from(&path, SHIJU_OFFICIAL_PROVIDER_ID).is_err() {
            return app.show_error_toast("无法清理识矩模型凭据，请检查认证文件权限");
        }
    }
    for provider in &mut app.provider_settings.providers {
        if provider.id == SHIJU_OFFICIAL_PROVIDER_ID {
            provider.connected = false;
        }
    }
    app.provider_settings.connect_modal = None;
    let Some(account) = app.account.active_account() else {
        return super::tasks::refresh_task();
    };
    let account_id = account.id().to_string();
    let credential = match account.skill_hub_credential().gateway_credential() {
        Ok(credential) => credential,
        Err(_) => return app.show_error_toast("识矩登录凭据无效，请重新登录"),
    };
    Task::perform(
        async move { config::gateway_client()?.shiju_model_token(&credential).await },
        move |result| {
            Message::Account(AccountMessage::ModelTokenLoaded {
                generation,
                account_id: account_id.clone(),
                result,
            })
        },
    )
}

pub(crate) fn token_loaded(
    app: &mut App,
    generation: u64,
    account_id: String,
    result: Result<ShijuModelTokenDto, String>,
) -> Task<Message> {
    if !app.account.model_sync_is_current(generation)
        || app.account.active_account().map(|account| account.id()) != Some(account_id.as_str())
    {
        return Task::none();
    }
    let token = match result {
        Ok(token) => token,
        Err(_) => return retry_task(generation),
    };
    // 网关响应仍需在持久化边界校验，避免错误配置将模型密钥发送给其他服务。
    if let Err(error) = save_token(&super::models::auth_filepath(), &token) {
        return app.show_error_toast(error);
    }
    Task::perform(
        async move {
            config::patch_full_agent_config_async(serde_json::json!({
                "providers": {"shiju": {
                    "name": "识矩", "api": token.endpoint_url,
                    "adapter": "openai-compatible", "env": []
                }}
            }))
            .await
            .map_err(config::server_config_unreachable_error)?;
            Ok(())
        },
        move |result| Message::Account(AccountMessage::ModelsConfigured { generation, result }),
    )
}

pub(crate) fn configured(
    app: &mut App,
    generation: u64,
    result: Result<(), String>,
) -> Task<Message> {
    if !app.account.model_sync_is_current(generation) {
        return Task::none();
    }
    match result {
        Ok(()) => Task::batch([
            super::tasks::refresh_task(),
            app.show_success_toast("识矩模型已自动配置"),
        ]),
        Err(_) => retry_task(generation),
    }
}

#[cfg(test)]
mod tests;
