use vw_api_app_operations::{AppActionPreviewRequest, AppActionTarget, AppLifecycleAction};
use vw_api_app_identity::{ AppId, AppInstallationEnvironment, AppInstallationRuntimeStatus, AppInstallationSource, AppRuntimeKind};
// Only the selected application receives configuration and lifecycle controls.
use super::{catalog, catalog::CatalogRow, list::action, presentation::{self, Presence}, Action, ViewState};
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, text, text::Wrapping},
};

pub fn view<'a, M: Clone + 'static>(
    state: &ViewState<'a>,
    app: &'a catalog::CatalogRow,
    cross_domain: Element<'a, M>,
    manual_guidance: &'a str,
    message: impl Fn(Action) -> M + Copy + 'static,
) -> Element<'a, M> {
    let mut details = column![
        settings_group_heading(&app.entry.name, "配置和操作仅作用于此智能体"),
        text("运行状态").size(15),
        button(text("重新探测此应用").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.loading && state.drag.source.is_none()).then(move ||
                message(Action::RefreshObservation(app.entry.id)))),
    ].spacing(10);
    details = details.push(cross_domain);
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
                if observation.installations.is_empty()
            && app.entry.runtime_kinds.contains(&AppRuntimeKind::NativeCli)
        {
            details = details.push(
                row![
                    super::lifecycle_view::action_button(
                        "macOS 安装预检",
                        AppActionPreviewRequest {
                            app_id: app.entry.id,
                            action: AppLifecycleAction::Install,
                            target: AppActionTarget::Environment {
                                environment: AppInstallationEnvironment::Macos
                            }
                        },
                        !state.lifecycle.active() && state.lifecycle.preview.is_none(), message
                    ),
                    super::lifecycle_view::action_button(
                        "Linux 安装预检",
                        AppActionPreviewRequest {
                            app_id: app.entry.id,
                            action: AppLifecycleAction::Install,
                            target: AppActionTarget::Environment {
                                environment: AppInstallationEnvironment::Linux
                            }
                        },
                        !state.lifecycle.active() && state.lifecycle.preview.is_none(), message
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
                    .on_press_maybe(enabled.then(move || message(Action::LaunchOpen(app.entry.id, installation.id.clone())))),
                super::lifecycle_view::action_button(
                    "检查升级",
                    AppActionPreviewRequest {
                        app_id: app.entry.id,
                        action: AppLifecycleAction::Update,
                        target: AppActionTarget::Installation {
                            installation_id: installation.id.clone()
                        }
                    },
                    enabled, message
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
                        .on_press_maybe(enabled.then_some(message(Action::Preview(request)))),
                );
            }
            installation_details = installation_details.push(actions.wrap());
            if installation.source == AppInstallationSource::Unknown {
                installation_details = installation_details.push(
                    text(manual_guidance).size(12).style(settings_muted_text_style),
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
    if app.entry.id == vw_api_app_identity::AppId::ClaudeDesktop {
        details = details
            .push(text("请从 Claude 官网手动下载安装到目标主机的 Applications 文件夹，然后刷新应用列表。下载页将在当前客户端浏览器打开；远程主机需在该主机上安装。")
                .size(12).style(settings_muted_text_style))
            .push(text("https://claude.com/download").size(13).wrapping(Wrapping::Glyph))
            .push(button(text("打开 Claude 官方下载页").size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press(message(Action::Download)));
    }
    container(details).padding([16, 18]).width(Length::Fill).into()
}

fn path_field<'a, M: 'a>(label: &'a str, path: &'a str) -> Element<'a, M> {
    column![
        text(label).size(12).style(settings_muted_text_style),
        text(path).size(13).wrapping(Wrapping::Glyph),
    ]
    .spacing(4)
    .into()
}

pub(super) fn actions<'a, M: Clone + 'static>(
    state: &ViewState<'a>,
    app: &'a CatalogRow,
    local_available: bool,
    message: impl Fn(Action) -> M + Copy + 'static,
) -> Element<'a, M> {
    let installed = presentation::presence(app) == Presence::Installed;
    let ready = presentation::preferred(app).is_some_and(|installation|
        matches!(installation.runtime_status, AppInstallationRuntimeStatus::Runnable));
    let busy = state.loading || state.lifecycle.active()
        || state.lifecycle.preview.is_some() || state.drag.source.is_some();
    let primary: Element<'_, M> = if ready {
        let installation = presentation::preferred(app).unwrap();
        button(
            text("启动")
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
        )
        .width(68.)
        .padding([8, 12])
        .style(|theme, status| {
            if status == iced::widget::button::Status::Disabled {
                rounded_action_btn_style(theme, status)
            } else {
                primary_action_btn_style(theme, status)
            }
        })
        .on_press_maybe((!busy).then(move || {
            message(Action::LaunchOpen(app.entry.id, installation.id.clone()))
        }))
        .into()
    } else if presentation::presence(app) == Presence::Missing
        && app.entry.runtime_kinds.contains(&AppRuntimeKind::NativeCli)
        && state.host == local_app_auth::HOST_ID
        && local_available
    {
        let environment = if cfg!(target_os = "macos") {
            AppInstallationEnvironment::Macos
        } else {
            AppInstallationEnvironment::Linux
        };
        action(
            "安装",
            Icon::CloudDownload,
            Action::Preview(
                    AppActionPreviewRequest {
                        app_id: app.entry.id,
                        action: AppLifecycleAction::Install,
                        target: AppActionTarget::Environment { environment },
                    },
            ),
            !busy, message,
        )
    } else {
        action(
            if installed { "检查" } else { "详情" },
            Icon::Sliders,
            Action::RefreshObservation(app.entry.id),
            !busy, message,
        )
    };
    let providers = button(text("供应商").size(SETTINGS_CONTROL_TEXT_SIZE))
        .padding([8, 11])
        .style(rounded_action_btn_style)
        .on_press_maybe((!busy).then_some(message(Action::Providers(app.entry.id))));
    let mut actions = row![providers].spacing(8).align_y(Alignment::Center);
    // Dedicated applications retain their workspace / explicit Claude Code entry.
    if !matches!(app.entry.id, AppId::ClaudeDesktop | AppId::OpenClaw) {
        let event = Action::Prompts(app.entry.id);
        actions = actions.push(
            button(text("提示词").size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 11])
                .style(rounded_action_btn_style)
                .on_press_maybe((!busy).then_some(message(event))),
        );
    }
    actions.push(primary).wrap().into()
}
