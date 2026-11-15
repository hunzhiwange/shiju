//! 网络设置领域效果的宿主适配。
use crate::app::{App, Message};
use iced::Task;
use super::messages::*;
use desktop_settings_network::message::gateway::{self, Effect};
use std::time::Duration;
fn service_command_task(command: String) -> Task<Message> {
    let request_command = command.clone();
    Task::perform(
        async move {
            let client = crate::app::config::gateway_client()?;
            gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_service_command(&request_command).await.map(|response| response.output)
        },
        move |result| {
            Message::Settings(SettingsMessage::Gateway(GatewayMessage::ServiceCommandCompleted(
                command.clone(),
                result,
            )))
        },
    )
}

fn service_recovery_task(command: String) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let request_command = command.clone();
        Task::perform(
            async move {
                if request_command == "restart" {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
                crate::app::config::bootstrap_cli_service_async().await
            },
            move |result| {
                Message::Settings(SettingsMessage::Gateway(
                    GatewayMessage::ServiceRecoveryCompleted(command.clone(), result),
                ))
            },
        )
    }

    #[cfg(target_arch = "wasm32")]
    {
        Task::done(Message::Settings(SettingsMessage::Gateway(
            GatewayMessage::ServiceRecoveryCompleted(
                command,
                Err("浏览器不支持本机 gateway 自动恢复".to_string()),
            ),
        )))
    }
}

pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Gateway(message) = message else { return Task::none(); };
    let message = desktop_settings_network::Message::Gateway(message);
    match gateway::update(&mut app.gateway_settings, message) {
Effect::None => Task::none(),
Effect::Persist(value) => crate::app::config::update_gateway_config_async(move |config| gateway::apply_config(config, value)),
Effect::ServiceCommand(command) => service_command_task(command),
Effect::ServiceRecovery(command) => service_recovery_task(command),
Effect::RefreshHealth => Task::done(Message::GatewayHealthTick),
Effect::Copy(value, clear) => Task::batch([iced::clipboard::write(value), crate::app::message::after(Duration::from_secs(2), Message::Settings(SettingsMessage::Gateway(clear)))]),
}
}
