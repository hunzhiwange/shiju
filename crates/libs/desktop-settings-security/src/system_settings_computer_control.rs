//! Server-reported native control capabilities and explicit authorization management.
use desktop_i18n::I18n;
use crate::computer_control_state::{ComputerControlSettingsState, ControlIdentity};
use crate::computer_control_update::{Event, message};
use desktop_ui_controls::system_settings_common::*;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, svg, text};
use iced::{Alignment, Element, Length};
use vw_api_computer_control::*;

pub const FEATURES: [(ControlSetting, &str); 2] = [
    (ControlSetting::NativeApps, "任意应用"),
    (ControlSetting::LockedUse, "锁屏使用"),
];
pub fn readiness_label(readiness: ControlReadiness) -> &'static str {
    match readiness {
        ControlReadiness::Disabled => "已关闭",
        ControlReadiness::NotInstalled => "后端未安装",
        ControlReadiness::Unauthorized => "缺少系统权限：请在网关主机授予辅助功能及屏幕录制权限",
        ControlReadiness::Disconnected => "未连接",
        ControlReadiness::Ready => "服务端报告已就绪",
        ControlReadiness::Unsupported => "当前平台或后端不支持",
    }
}
fn feature_readiness_label(feature: ControlSetting, readiness: ControlReadiness) -> &'static str {
    match (feature, readiness) {
        (ControlSetting::NativeApps, ControlReadiness::NotInstalled) => "执行应用未安装",
        (ControlSetting::NativeApps, ControlReadiness::Disconnected) => "执行应用无法连接",
        _ => readiness_label(readiness),
    }
}
fn action(i18n: I18n, label: &'static str, event: Event, enabled: bool) -> Element<'static, Event> {
    button(text(i18n.__(label)).size(13))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then(|| message(event)))
        .into()
}
pub fn view<'a>(i18n: I18n,
    state: &'a ComputerControlSettingsState,
    identity: &ControlIdentity,
) -> Element<'a, Event> {
    if state.identity.as_ref() != Some(identity) {
        return column![
            text(i18n.__("电脑操控")),
            text(i18n.__("网关已变化，请重新加载状态")),
            action(i18n, i18n.__("加载状态"), Event::Refresh, true)
        ]
        .spacing(14)
        .into();
    }
    page(i18n, state)
}
pub fn page(i18n: I18n, state: &ComputerControlSettingsState) -> Element<'_, Event> {
    if let Some(feature) = state.management {
        return management_page(i18n, state, feature);
    }
    let header = column![
        text(i18n.__("电脑操控")).size(SETTINGS_PAGE_TITLE_SIZE),
        text(i18n.__("管理识矩如何使用电脑上的应用程序")).size(13).style(settings_muted_text_style),
        row![
            action(i18n,
                if state.loading { i18n.__("加载中…") } else { i18n.__("刷新状态") },
                Event::Refresh,
                !state.busy()
            ),
            button(text(if state.stopping { i18n.__("正在停止…") } else { i18n.__("停止所有电脑操控") }).size(13))
                .padding([8, 14])
                .style(danger_action_btn_style)
                .on_press_maybe((!state.stopping).then(|| message(Event::Stop))),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .wrap(),
    ]
    .spacing(16)
    .width(Length::Fill);
    let mut content = column![text(i18n.__("控制")).size(17)].spacing(16).width(Length::Fill);
    if state.saving {
        content = content.push(text(i18n.__("正在等待服务端确认…")).size(13));
    }
    if let Some(error) = &state.error {
        content = content.push(settings_error_banner(desktop_i18n::settings::message(i18n, error)));
    }
    for (feature, title) in FEATURES {
        let report = state.feature(feature);
        let enabled = report.is_some_and(|s| s.enabled);
        let mut toggle = settings_toggle(enabled);
        if !state.busy() && state.can_set_feature(feature, !enabled) {
            toggle = toggle.on_toggle(move |enabled| message(Event::Toggle(feature, enabled)));
        }
        let mut details = column![
            text(i18n.__(title)).size(16),
            text(i18n.__(feature_description(feature))).size(13).style(settings_muted_text_style),
            text(i18n.__(report.map(|s| feature_readiness_label(feature, s.readiness)).unwrap_or("尚未取得服务端状态")))
                .size(13)
                .style(settings_muted_text_style),
        ]
        .spacing(10);
        if let Some(report) = report {
            let authorization = match report.system_authorization {
                SystemAuthorization::NotChecked => i18n.__("未检查"),
                SystemAuthorization::NotRequired => i18n.__("无需系统授权"),
                SystemAuthorization::Granted => i18n.__("已授予"),
                SystemAuthorization::Denied => i18n.__("未授予"),
            };
            let connection = match report.connection {
                ControlConnection::Disconnected => i18n.__("未连接"),
                ControlConnection::Connected => i18n.__("已连接"),
            };
            let backend = match report.backend {
                ControlBackend::NotInstalled => i18n.__("未安装"),
                ControlBackend::Available => i18n.__("可用"),
                ControlBackend::Unsupported => i18n.__("不支持"),
            };
            details = details.push(
                text(desktop_i18n::__!(i18n, "授权 %s  ·  连接 %s  ·  后端 %s", (authorization).to_string(), (connection).to_string(), (backend).to_string()).expect("valid settings message"))
                    .size(11)
                    .style(settings_muted_text_style),
            );
            if !report.capabilities.is_empty() {
                let capabilities = report
                    .capabilities
                    .iter()
                    .map(|capability| match capability {
                        ControlCapability::Observe => i18n.__("观察"),
                        ControlCapability::Screenshot => i18n.__("截图"),
                        ControlCapability::Input => i18n.__("输入"),
                        ControlCapability::LockedExecution => i18n.__("锁屏执行"),
                    })
                    .collect::<Vec<_>>()
                    .join("、");
                details = details.push(
                    text(desktop_i18n::__!(i18n, "能力：%s", (capabilities).to_string()).expect("valid settings message")).size(11).style(settings_muted_text_style),
                );
            }
        }
        if feature == ControlSetting::NativeApps {
            let label = if state.native_service.as_ref().is_some_and(|report| report.state == NativeServiceState::NotInstalled)
            {
                i18n.__("安装与权限")
            } else {
                i18n.__("管理权限")
            };
            details = details.push(action(i18n, label, Event::Manage(feature), true));
        }
        if feature == ControlSetting::LockedUse && let Some(status) = &state.status {
            let hint = match status.locked_use.probe {
                LockedUseProbe::Supported => "锁屏时仅支持可访问性观察及部分控件操作。",
                LockedUseProbe::Unsupported => "当前执行应用或平台不支持锁屏使用。",
                LockedUseProbe::NotProbed => "请确认执行应用已安装且可连接，然后刷新状态。",
            };
            details = details.push(text(i18n.__(hint)).size(12));
        }
        let feature_row = container(
            row![feature_icon(feature), details.width(Length::Fill), toggle,]
                .spacing(18)
                .align_y(Alignment::Center),
        )
        .padding([20, 0]);
        content = content.push(settings_group_card(feature_row));
    }
    let mut grants = column![].spacing(12);
    for scope @ ControlGrantScope::Application(identity) in &state.approvals {
        grants = grants.push(
            row![
                column![
                    text(i18n.__("应用 Bundle ID")).size(12).style(settings_muted_text_style),
                    text(identity).size(13)
                ]
                .spacing(4)
                .width(Length::Fill),
                action(i18n, i18n.__("撤销授权"), Event::Revoke(scope.clone()), !state.busy()),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );
    }
    if state.approvals.is_empty() {
        grants = grants.push(
            container(
                text(if state.status.is_some() {
                    i18n.__("暂无授权")
                } else {
                    i18n.__("授权列表尚未加载")
                })
                .size(13)
                .style(settings_muted_text_style),
            )
            .center_x(Length::Fill)
            .padding([20, 0]),
        );
    }
    content = content.push(text(i18n.__("始终允许的应用")).size(17));
    content = content.push(settings_group_card(container(grants).padding([16, 0])));
    let body = scrollable(container(content).padding([0, 7]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill);
    column![
        header,
        text(i18n.__("开关保存使用策略，系统权限由当前网关主机检查。"))
            .size(12)
            .style(settings_muted_text_style),
        body
    ]
    .spacing(24)
    .height(Length::Fill)
    .into()
}

fn management_page(
    i18n: I18n,
    state: &ComputerControlSettingsState,
    feature: ControlSetting,
) -> Element<'_, Event> {
    let title = i18n.__("任意应用");
    let readiness = state.feature(feature).map(|report| report.readiness);
    let status = i18n.__(readiness
        .map(|readiness| feature_readiness_label(feature, readiness))
        .unwrap_or("尚未取得服务端状态"));
    let refresh_label = if state.loading { "加载中…" } else { "刷新状态" };
    let mut content = column![
        row![
            action(i18n, "返回电脑操控", Event::ManageClose, true),
            action(i18n, refresh_label, Event::Refresh, !state.busy()),
        ]
        .spacing(8)
        .wrap(),
        settings_page_heading(title, i18n.__(feature_description(feature))),
    ]
    .spacing(16)
    .width(Length::Fill);
    if let Some(error) = &state.error {
        content = content.push(settings_error_banner(desktop_i18n::settings::message(i18n, error)));
    }
    let status = if matches!(
            readiness,
            Some(
                ControlReadiness::Unauthorized
                    | ControlReadiness::NotInstalled
                    | ControlReadiness::Disconnected
            )
        ) {
        settings_error_banner(status)
    } else {
        text(status).size(13).style(settings_muted_text_style).into()
    };
    content = content.push(status);
    content = content.push(native_permissions(i18n, state));
    let mut grants = column![settings_group_heading(
        i18n.__("已授权应用"),
        i18n.__("持久授权在实际操作的确认提示中授予，可在此撤销。"),
    )]
    .spacing(12)
    .width(Length::Fill);
    for scope @ ControlGrantScope::Application(identity) in &state.approvals {
        grants = grants.push(settings_divider()).push(
            row![
                text(identity.as_str())
                    .size(13)
                    .width(Length::Fill)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                action(i18n, "撤销授权", Event::Revoke(scope.clone()), !state.busy()),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );
    }
    if state.approvals.is_empty() {
        let empty_label =
            if state.status.is_some() { "暂无授权" } else { "授权列表尚未加载" };
        grants = grants
            .push(settings_divider())
            .push(text(i18n.__(empty_label)).size(13).style(settings_muted_text_style));
    }
    content = content.push(settings_group_card(grants).padding([16, 0]));
    scrollable(container(content).padding([8, 7]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn native_permissions(i18n: I18n, state: &ComputerControlSettingsState) -> Element<'_, Event> {
    use iced::widget::text::Wrapping;

    let service_state = state.native_service.as_ref().map(|report| report.state);
    let can_open_guide = cfg!(target_os = "macos")
        && state.target_is_local
        && matches!(
            service_state,
            Some(NativeServiceState::Available | NativeServiceState::Disconnected)
        );
    let mut permissions = column![settings_group_heading(
        i18n.__("系统权限"),
        i18n.__("请在网关所在的 Mac 上为 ShiJu Computer Use 授予辅助功能和屏幕录制权限。"),
    )]
    .spacing(12)
    .width(Length::Fill);
    let report = state.native_permission_report.as_ref();
    if can_open_guide || report.is_some() {
        if can_open_guide {
            permissions = permissions.push(
                text(i18n.__(
                    "打开对应的系统设置，将授权窗口里的应用图标拖放到列表中，开启开关后返回刷新。",
                ))
                .size(13)
                .style(settings_muted_text_style),
            );
        }
        for (label, open_label, screen_recording, granted) in [
            ("辅助功能", "打开辅助功能", false, report.map(|report| report.accessibility)),
            ("屏幕录制", "打开屏幕录制", true, report.map(|report| report.screen_recording)),
        ] {
            let mut details = column![text(i18n.__(label)).size(14)].spacing(4).width(Length::Fill);
            if let Some(granted) = granted {
                let status = if granted { "已授权" } else { "未授权" };
                details =
                    details.push(text(i18n.__(status)).size(12).style(settings_muted_text_style));
            }
            let mut permission = row![details].spacing(12).align_y(Alignment::Center);
            if can_open_guide {
                permission = permission.push(action(
                    i18n,
                    open_label,
                    Event::OpenNativePermissionGuide { screen_recording },
                    !state.busy(),
                ));
            }
            permissions = permissions.push(settings_divider()).push(permission);
        }
    }

    let mut installation = column![
        text(i18n.__("目标网关")).size(12).style(settings_muted_text_style),
        text(state.target_host.as_str()).size(13).wrapping(Wrapping::WordOrGlyph),
    ]
    .spacing(6)
    .width(Length::Fill);
    if let Some(service_state) = service_state {
        let label = match service_state {
            NativeServiceState::Available => "执行应用可用",
            NativeServiceState::NotInstalled => "执行应用未安装",
            NativeServiceState::Disconnected => "执行应用断开或身份验证失败",
            NativeServiceState::Unsupported => "目标系统暂不支持",
        };
        installation =
            installation.push(text(i18n.__(label)).size(12).style(settings_muted_text_style));
    }
    if let Some(install) = &state.native_install {
        let mut actions =
            row![action(i18n, "复制安装地址", Event::CopyNativeInstallPath, !state.busy())]
                .spacing(8);
        let mut location = column![
            text(i18n.__("安装包内位置")).size(12).style(settings_muted_text_style),
            text(install.app_path.as_str()).size(13).wrapping(Wrapping::WordOrGlyph),
        ]
        .spacing(8)
        .width(Length::Fill);
        if service_state == Some(NativeServiceState::NotInstalled) {
            location = location.push(
                text(i18n.__("在本机终端运行安装命令，完成后刷新状态。"))
                    .size(13)
                    .style(settings_muted_text_style),
            );
            actions = actions.push(action(
                i18n,
                "复制安装命令",
                Event::CopyNativeInstallCommand,
                !state.busy(),
            ));
        }
        installation = installation.push(settings_section_content(location.push(actions.wrap())));
    } else if service_state == Some(NativeServiceState::NotInstalled) {
        let hint = if state.target_is_local {
            "请安装包含 ShiJu Computer Use.app 的完整识矩 macOS 安装包，然后刷新状态。"
        } else {
            "请在运行网关的 Mac 上打开识矩，从该电脑的安装包安装执行应用，然后刷新状态。"
        };
        installation = installation.push(text(i18n.__(hint)).size(13));
    }
    settings_group_card(permissions.push(settings_divider()).push(installation))
        .padding([16, 0])
        .into()
}

fn feature_description(feature: ControlSetting) -> &'static str {
    match feature {
        ControlSetting::NativeApps => "允许识矩观察和操作电脑上的应用",
        ControlSetting::LockedUse => "允许在电脑锁定时继续执行任务，需要后端支持",
    }
}

fn feature_icon(feature: ControlSetting) -> Element<'static, Event> {
    let (color, path) = match feature {
        ControlSetting::NativeApps => ("#818cf8", "M4 4h24v17H4z M11 27h10 M16 21v6"),
        ControlSetting::LockedUse => {
            ("#fbbf24", "M7 14h18v14H7z M11 14V9a5 5 0 0 1 10 0v5 M16 20v3")
        }
    };
    let data = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 48 48"><rect width="48" height="48" rx="12" fill="{color}" fill-opacity="0.12"/><path d="{path}" transform="translate(8 8)" fill="none" stroke="{color}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>"#
    );
    svg(svg::Handle::from_memory(data.into_bytes())).width(48).height(48).into()
}

#[cfg(test)]
#[path = "system_settings_computer_control_tests.rs"]
mod tests;
