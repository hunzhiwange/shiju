//! Expanded installation details keep diagnostics and destructive actions out of the overview.
use super::{catalog, launch, lifecycle, message, uninstall};
use crate::app::components::system_settings_common::*;
use crate::app::{Message, message::settings::AppsCatalogMessage, state::AppsCatalogState};
use iced::{
    Element, Length,
    widget::{button, column, container, row, text},
};

pub(super) fn view<'a>(
    state: &'a AppsCatalogState,
    app: &'a catalog::CatalogRow,
) -> Element<'a, Message> {
    let mut details = column![].spacing(10);
    for kind in &app.entry.runtime_kinds {
        details = details.push(
            text(format!(
                "{}：{}",
                catalog::runtime_label(*kind),
                catalog::runtime_status(app, *kind)
            ))
            .size(13),
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
                .spacing(10),
            );
        }
        for installation in &observation.installations {
            let environment = match &installation.environment {
                AppInstallationEnvironment::Macos => "macOS".to_string(),
                AppInstallationEnvironment::Windows => "Windows".to_string(),
                AppInstallationEnvironment::Linux => "Linux".to_string(),
                AppInstallationEnvironment::Wsl { distribution } => format!("WSL · {distribution}"),
            };
            details = details.push(
                text(format!(
                    "{} · {} · {}",
                    super::presentation::source_label(installation.source),
                    environment,
                    installation.version.as_deref().unwrap_or("版本待确认")
                ))
                .size(12)
                .style(settings_muted_text_style),
            );
            details = details.push(
                column![
                    text("安装位置").size(11).style(settings_muted_text_style),
                    text(&installation.path).size(12)
                ]
                .spacing(4),
            );
            if let Some(real) = &installation.real_path {
                if real != &installation.path {
                    details = details.push(
                        column![
                            text("真实位置").size(11).style(settings_muted_text_style),
                            text(real).size(12)
                        ]
                        .spacing(4),
                    );
                }
            } else {
                details = details
                    .push(text("真实位置尚未确认").size(11).style(settings_muted_text_style));
            }
            let enabled = !state.lifecycle.active()
                && state.lifecycle.preview.is_none()
                && state.drag.source.is_none();
            let mut actions = row![
                button(text("终端启动").size(12))
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
            .spacing(8);
            if installation.source != AppInstallationSource::Unknown {
                actions = actions.push(button(text("卸载").size(12)).padding([8,12]).style(danger_action_btn_style)
                    .on_press_maybe(enabled.then(||message(AppsCatalogMessage::Lifecycle(crate::app::message::settings::apps_lifecycle::LifecycleMessage::Preview(AppActionPreviewRequest {app_id:app.entry.id,action:AppLifecycleAction::Uninstall,target:AppActionTarget::Installation {installation_id:installation.id.clone()}}))))));
            }
            details = details.push(actions.wrap());
            if installation.source == AppInstallationSource::Unknown {
                details = details.push(
                    text(uninstall::MANUAL_GUIDANCE).size(11).style(settings_muted_text_style),
                );
            }
        }
        if let Some(desktop) = &observation.desktop {
            for path in desktop.app_bundles.iter().flatten() {
                details = details.push(text(path).size(12));
            }
        }
    }
    container(details).padding([16, 18]).width(Length::Fill).into()
}
