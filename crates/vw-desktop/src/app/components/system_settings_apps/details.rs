//! Expanded installation details keep diagnostics and destructive actions out of the overview.
use super::{catalog, launch, lifecycle, message, uninstall};
use crate::app::components::system_settings_common::*;
use crate::app::{
    Message,
    message::settings::{AppsCatalogMessage, apps_lifecycle::LifecycleMessage},
    state::AppsCatalogState,
};
use iced::{
    Element, Length,
    widget::{button, column, container, row, text, text::Wrapping},
};

pub(super) fn view<'a>(
    state: &'a AppsCatalogState,
    app: &'a catalog::CatalogRow,
) -> Element<'a, Message> {
    let mut details = column![text("运行状态").size(15)].spacing(10);
    for kind in &app.entry.runtime_kinds {
        details = details.push(
            text(format!(
                "{}：{}",
                catalog::runtime_label(*kind),
                catalog::runtime_status(app, *kind)
            ))
            .size(13)
            .wrapping(Wrapping::Glyph),
        );
    }
    if let Ok(observation) = &app.observation {
        use vw_gateway_client::vw_api_types::agent_apps::*;
        if observation.installations.is_empty()
            && app.entry.runtime_kinds.contains(&AppRuntimeKind::NativeCli)
        {
            details = details.push(
                row![
                    lifecycle::action_button(
                        "macOS 安装预检",
                        AppActionPreviewRequest {
                            app_id: app.entry.id,
                            action: AppLifecycleAction::Install,
                            target: AppActionTarget::Environment {
                                environment: AppInstallationEnvironment::Macos
                            }
                        },
                        !state.lifecycle.active() && state.lifecycle.preview.is_none()
                    ),
                    lifecycle::action_button(
                        "Linux 安装预检",
                        AppActionPreviewRequest {
                            app_id: app.entry.id,
                            action: AppLifecycleAction::Install,
                            target: AppActionTarget::Environment {
                                environment: AppInstallationEnvironment::Linux
                            }
                        },
                        !state.lifecycle.active() && state.lifecycle.preview.is_none()
                    ),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .wrap(),
            );
        }
        for installation in &observation.installations {
            let environment = match &installation.environment {
                AppInstallationEnvironment::Macos => "macOS".to_string(),
                AppInstallationEnvironment::Windows => "Windows".to_string(),
                AppInstallationEnvironment::Linux => "Linux".to_string(),
                AppInstallationEnvironment::Wsl { distribution } => format!("WSL · {distribution}"),
            };
            let mut installation_details = column![
                text(format!(
                    "{} · {} · {}",
                    super::presentation::source_label(installation.source),
                    environment,
                    installation.version.as_deref().unwrap_or("版本待确认")
                ))
                .size(14)
                .wrapping(Wrapping::Glyph),
                path_field("安装位置", &installation.path),
            ]
            .spacing(10);
            if let Some(real) = &installation.real_path {
                if real != &installation.path {
                    installation_details = installation_details.push(path_field("真实位置", real));
                }
            } else {
                installation_details = installation_details
                    .push(text("真实位置尚未确认").size(12).style(settings_muted_text_style));
            }
            let enabled = !state.lifecycle.active()
                && state.lifecycle.preview.is_none()
                && state.drag.source.is_none();
            let mut actions = row![
                button(text("终端启动").size(SETTINGS_CONTROL_TEXT_SIZE))
                    .padding([8, 12])
                    .style(primary_action_btn_style)
                    .on_press_maybe(enabled.then(|| message(AppsCatalogMessage::Launch(
                        launch::LaunchMessage::Open(app.entry.id, installation.id.clone())
                    )))),
                lifecycle::action_button(
                    "检查升级",
                    AppActionPreviewRequest {
                        app_id: app.entry.id,
                        action: AppLifecycleAction::Update,
                        target: AppActionTarget::Installation {
                            installation_id: installation.id.clone()
                        }
                    },
                    enabled
                ),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);
            if installation.source != AppInstallationSource::Unknown {
                let request = AppActionPreviewRequest {
                    app_id: app.entry.id,
                    action: AppLifecycleAction::Uninstall,
                    target: AppActionTarget::Installation {
                        installation_id: installation.id.clone(),
                    },
                };
                actions = actions.push(
                    button(text("卸载").size(SETTINGS_CONTROL_TEXT_SIZE))
                        .padding([8, 12])
                        .style(danger_action_btn_style)
                        .on_press_maybe(enabled.then_some(message(AppsCatalogMessage::Lifecycle(
                            LifecycleMessage::Preview(request),
                        )))),
                );
            }
            installation_details = installation_details.push(actions.wrap());
            if installation.source == AppInstallationSource::Unknown {
                installation_details = installation_details.push(
                    text(uninstall::MANUAL_GUIDANCE).size(12).style(settings_muted_text_style),
                );
            }
            details = details.push(settings_divider()).push(installation_details);
        }
        if let Some(desktop) = &observation.desktop {
            for path in desktop.app_bundles.iter().flatten() {
                details = details.push(path_field("安装位置", path));
            }
        }
    }
    if app.entry.id == vw_gateway_client::vw_api_types::agent_apps::AppId::ClaudeDesktop {
        details = details
            .push(text("请从 Claude 官网手动下载安装到目标主机的 Applications 文件夹，然后刷新应用列表。下载页将在当前客户端浏览器打开；远程主机需在该主机上安装。")
                .size(12).style(settings_muted_text_style))
            .push(text("https://claude.com/download").size(13).wrapping(Wrapping::Glyph))
            .push(button(text("打开 Claude 官方下载页").size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press(Message::View(crate::app::message::ViewMessage::OpenUrlExternal(
                    "https://claude.com/download".into(),
                ))));
    }
    container(details).padding([16, 18]).width(Length::Fill).into()
}

fn path_field<'a>(label: &'a str, path: &'a str) -> Element<'a, Message> {
    column![
        text(label).size(12).style(settings_muted_text_style),
        text(path).size(13).wrapping(Wrapping::Glyph),
    ]
    .spacing(4)
    .into()
}
