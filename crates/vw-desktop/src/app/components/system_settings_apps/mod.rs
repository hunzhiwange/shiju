pub(crate) mod catalog;
#[cfg(test)]
mod catalog_tests;
pub(crate) mod lifecycle;
#[cfg(test)]
mod lifecycle_tests;
pub(crate) mod uninstall;

use super::system_settings_common::*;
use crate::app::{
    Message,
    message::{SettingsMessage, settings::AppsCatalogMessage},
    state::AppsCatalogState,
};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};

fn message(value: AppsCatalogMessage) -> Message {
    Message::Settings(SettingsMessage::AppsCatalog(value))
}

pub(crate) fn view(state: &AppsCatalogState) -> Element<'_, Message> {
    let refresh = button(text(if state.error.is_some() { "重试" } else { "刷新" }))
        .padding([8, 16])
        .style(rounded_action_btn_style)
        .on_press_maybe((!state.loading).then_some(message(AppsCatalogMessage::Refresh)));
    let connection = button(text("选择主机"))
        .padding([8, 16])
        .style(rounded_action_btn_style)
        .on_press(message(AppsCatalogMessage::ConnectionOpen));
    let mut content = column![
        settings_section_card(
            "应用列表",
            "读取所选网关执行主机的应用；各运行形态分别展示观察结果。"
        ),
        row![
            connection,
            refresh,
            button(text("应用设置"))
                .padding([8, 16])
                .style(rounded_action_btn_style)
                .on_press_maybe((!state.loading && !state.rows.is_empty()).then_some(message(
                    AppsCatalogMessage::Preferences(preferences::PreferencesMessage::Open)
                )))
        ]
        .spacing(10),
        text(format!("执行主机：{}", if state.host.is_empty() { "尚未选择" } else { &state.host }))
            .size(13),
    ]
    .spacing(12);
    if state.loading {
        content = content.push(text("正在加载应用与安装诊断…").style(settings_muted_text_style));
    }
    if let Some(error) = &state.error {
        content = content.push(settings_error_banner(error.clone()));
    }
    if state.rows.is_empty() && !state.loading && state.error.is_none() {
        content = content
            .push(text("选择已授权的执行主机后刷新应用列表。").style(settings_muted_text_style));
    }
    content = content.push(lifecycle::progress(&state.lifecycle));
    for app in preferences::visible_rows(state) {
        let mut details = column![text(&app.entry.name).size(18)].spacing(8);
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
                details = details.push(
                    text(format!(
                        "{:?} · {:?} · {}\n实例路径：{}\n真实路径：{}",
                        installation.environment,
                        installation.source,
                        installation.version.as_deref().unwrap_or("版本未验证"),
                        installation.path,
                        installation.real_path.as_deref().unwrap_or("未确认")
                    ))
                    .size(12),
                );
                details = details.push(
                    button(text("终端启动"))
                        .padding([8, 16])
                        .style(rounded_action_btn_style)
                        .on_press(message(AppsCatalogMessage::Launch(
                            launch::LaunchMessage::Open(app.entry.id, installation.id.clone()),
                        ))),
                );
                details = details.push(lifecycle::action_button(
                    "升级此实例（预检）",
                    AppActionPreviewRequest {
                        app_id: app.entry.id,
                        action: AppLifecycleAction::Update,
                        target: AppActionTarget::Installation {
                            installation_id: installation.id.clone(),
                        },
                    },
                    !state.lifecycle.active() && state.lifecycle.preview.is_none(),
                ));
                if installation.source == AppInstallationSource::Unknown {
                    details = details.push(text(uninstall::MANUAL_GUIDANCE).size(12));
                } else {
                    details = details.push(lifecycle::action_button(
                        "卸载此实例（预检）",
                        AppActionPreviewRequest {
                            app_id: app.entry.id,
                            action: AppLifecycleAction::Uninstall,
                            target: AppActionTarget::Installation {
                                installation_id: installation.id.clone(),
                            },
                        },
                        !state.lifecycle.active() && state.lifecycle.preview.is_none(),
                    ));
                }
            }
            if let Some(desktop) = &observation.desktop {
                for path in desktop.app_bundles.iter().flatten() {
                    details = details.push(text(path).size(12));
                }
            }
        }
        content = content.push(settings_section_content(details));
    }
    container(content).padding([0, 7]).width(Length::Fill).into()
}

pub(crate) fn view_overlays<'a>(
    state: &'a AppsCatalogState,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    if !state.connection_open {
        return launch::overlay(
            state,
            preferences::overlay(state, lifecycle::overlay(&state.lifecycle, &state.host, base)),
        );
    }
    let close = message(AppsCatalogMessage::ConnectionClose);
    let host = text_input("管理员登记的主机 ID", &state.host)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .on_input(|value| message(AppsCatalogMessage::HostInput(value)));
    let token = text_input("应用管理令牌", &state.token)
        .secure(true)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .on_input(|value| message(AppsCatalogMessage::TokenInput(value)));
    let connect =
        button(text("加载应用")).padding([8, 16]).style(primary_action_btn_style).on_press_maybe(
            (!state.host.trim().is_empty() && !state.token.is_empty())
                .then_some(message(AppsCatalogMessage::Refresh)),
        );
    let cancel = button(text("取消"))
        .padding([8, 16])
        .style(rounded_action_btn_style)
        .on_press(close.clone());
    let card = settings_modal_card(
        column![
            row![
                container(text("选择执行主机").size(22)).width(Length::Fill),
                settings_close_button(close.clone())
            ],
            text("主机 ID 与令牌由网关管理员提供；令牌仅在本次会话内存中保留。")
                .size(12)
                .style(settings_muted_text_style),
            text("执行主机").size(13),
            host,
            text("应用管理令牌").size(13),
            token,
            row![container(text("")).width(Length::Fill), cancel, connect].spacing(10),
        ]
        .spacing(12),
    )
    .width(Length::Fixed(520.0));
    settings_modal_overlay(Some(base), close, card)
}

#[cfg(test)]
mod uninstall_tests;

pub(crate) mod preferences;
#[cfg(test)]
mod preferences_tests;

pub(crate) mod launch;
#[cfg(test)]
mod launch_tests;
