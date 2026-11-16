//! 将账号登录态连接到识矩模型认证；秘密只在当前账号的结果被接受后写入。

use crate::app::config;
use crate::app::i18n::settings_models::messages::Feedback;
use crate::app::message::account::AccountMessage;
use crate::app::{App, Message};
use iced::Task;
use gateway_shiju_account::ShijuModelTokenDto;

use desktop_settings_providers::managed_shiju::save_token;

/// 登录、启动恢复、切换账号及退出时统一撤销旧模型凭据。
pub(crate) fn sync(app: &mut App) -> Task<Message> {
    let generation = app.account.begin_model_sync();
    Task::done(Message::Account(AccountMessage::ResetModels { generation }))
}

fn retry_task(generation: u64) -> Task<Message> {
    desktop_settings_providers::managed_shiju::retry_task(generation)
        .map(|generation| Message::Account(AccountMessage::ResetModels { generation }))
}

pub(crate) fn reset(app: &mut App, generation: u64) -> Task<Message> {
    if !app.account.model_sync_is_current(generation) {
        return Task::none();
    }
    let path = super::models::auth_filepath();
    if let Err(error) = desktop_settings_providers::managed_shiju::reset_credentials(&path, &mut app.provider_settings) {
        return app.show_error_toast(error.display(app.desktop_locale));
    }
    let Some(account) = app.account.active_account() else {
        return super::tasks::refresh_task();
    };
    let account_id = account.id().to_string();
    let credential = match account.skill_hub_credential().gateway_credential() {
        Ok(credential) => credential,
        Err(_) => {
            return app.show_error_toast(
                Feedback::owned("识矩登录凭据无效，请重新登录", vec![]).display(app.desktop_locale),
            );
        }
    };
    Task::perform(
        async move { gateway_shiju_account::GatewayShijuAccountClient::new(config::gateway_client()?.transport().clone()).shiju_model_token(&credential).await },
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
        return app.show_error_toast(error.display(app.desktop_locale));
    }
    Task::perform(
        desktop_settings_providers::managed_shiju::configure(super::models::access(), token),
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
            app.show_success_toast(
                Feedback::owned("识矩模型已自动配置", vec![]).display(app.desktop_locale),
            ),
        ]),
        Err(_) => retry_task(generation),
    }
}

#[cfg(test)]
mod tests;
