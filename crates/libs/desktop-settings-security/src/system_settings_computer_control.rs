//! Five independent server-reported capabilities and explicit authorization management.
use crate::computer_control_state::{ComputerControlSettingsState, ControlIdentity};
use crate::computer_control_update::{Event, message};
use desktop_ui_controls::system_settings_common::*;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, svg, text, text_input};
use iced::{Alignment, Element, Length};
use vw_api_computer_control::*;

pub const FEATURES: [(ControlSetting, &str); 5] = [
    (ControlSetting::NativeApps, "任意应用"),
    (ControlSetting::Chrome, "Chrome"),
    (ControlSetting::Excel, "Excel"),
    (ControlSetting::Word, "Word"),
    (ControlSetting::LockedUse, "锁屏使用"),
];
pub fn readiness_label(readiness: ControlReadiness) -> &'static str {
    match readiness {
        ControlReadiness::Disabled => "已关闭",
        ControlReadiness::NotInstalled => "后端未安装",
        ControlReadiness::Unauthorized => "缺少系统权限：请在网关主机授予辅助功能及屏幕录制权限",
        ControlReadiness::Disconnected => "未连接：请连接 Chrome 扩展或 Office 加载项",
        ControlReadiness::Ready => "服务端报告已就绪",
        ControlReadiness::Unsupported => "当前平台或后端不支持",
    }
}
fn action(label: &'static str, event: Event, enabled: bool) -> Element<'static, Event> {
    button(text(label).size(13))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then(|| message(event)))
        .into()
}
pub fn view<'a>(
    state: &'a ComputerControlSettingsState,
    identity: &ControlIdentity,
) -> Element<'a, Event> {
    if state.identity.as_ref() != Some(identity) {
        return column![
            text("电脑操控"),
            text("网关已变化，请重新加载状态"),
            action("加载状态", Event::Refresh, true)
        ]
        .spacing(14)
        .into();
    }
    page(state)
}
pub fn page(state: &ComputerControlSettingsState) -> Element<'_, Event> {
    if let Some(feature) = state.management {
        return management_page(state, feature);
    }
    let header = column![
        text("电脑操控").size(28),
        text("管理识矩如何使用电脑上的应用程序").size(13).style(settings_muted_text_style),
        row![
            action(
                if state.loading { "加载中…" } else { "刷新状态" },
                Event::Refresh,
                !state.busy()
            ),
            button(text(if state.stopping { "正在停止…" } else { "停止所有电脑操控" }).size(13))
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
    let mut content = column![text("控制").size(17)].spacing(16).width(Length::Fill);
    let mut applications = column![].width(Length::Fill);
    if state.saving {
        content = content.push(text("正在等待服务端确认…").size(13));
    }
    if let Some(error) = &state.error {
        content = content.push(settings_error_banner(error));
    }
    for (feature, title) in FEATURES {
        let report = state.feature(feature);
        let mut toggle = settings_toggle(report.is_some_and(|s| s.enabled));
        if report.is_some() && !state.busy() {
            toggle = toggle.on_toggle(move |enabled| message(Event::Toggle(feature, enabled)));
        }
        let mut details = column![
            text(title).size(16),
            text(feature_description(feature)).size(13).style(settings_muted_text_style),
            text(report.map(|s| readiness_label(s.readiness)).unwrap_or("尚未取得服务端状态"))
                .size(13)
                .style(settings_muted_text_style),
        ]
        .spacing(10);
        if let Some(report) = report {
            let authorization = match report.system_authorization {
                SystemAuthorization::NotChecked => "未检查",
                SystemAuthorization::NotRequired => "无需系统授权",
                SystemAuthorization::Granted => "已授予",
                SystemAuthorization::Denied => "未授予",
            };
            let connection = match report.connection {
                ControlConnection::Disconnected => "未连接",
                ControlConnection::Connected => "已连接",
            };
            let backend = match report.backend {
                ControlBackend::NotInstalled => "未安装",
                ControlBackend::Available => "可用",
                ControlBackend::Unsupported => "不支持",
            };
            details = details.push(
                text(format!("授权 {authorization}  ·  连接 {connection}  ·  后端 {backend}"))
                    .size(11)
                    .style(settings_muted_text_style),
            );
            if !report.capabilities.is_empty() {
                let capabilities = report
                    .capabilities
                    .iter()
                    .map(|capability| match capability {
                        ControlCapability::Observe => "观察",
                        ControlCapability::Screenshot => "截图",
                        ControlCapability::Input => "输入",
                        ControlCapability::StructuredRead => "结构化读取",
                        ControlCapability::StructuredWrite => "结构化写入",
                        ControlCapability::LockedExecution => "锁屏执行",
                    })
                    .collect::<Vec<_>>()
                    .join("、");
                details = details.push(
                    text(format!("能力：{capabilities}")).size(11).style(settings_muted_text_style),
                );
            }
        }
        if matches!(feature, ControlSetting::NativeApps) {
            details = details.push(action("管理权限", Event::Manage(feature), true));
        }
        if matches!(feature, ControlSetting::Excel | ControlSetting::Word) {
            details = details.push(action(
                "确认 Office 文档配对",
                Event::PairOpen(feature),
                !state.busy() && report.is_some_and(|s| s.enabled),
            ));
        }
        if feature == ControlSetting::LockedUse {
            details = details.push(text("锁屏需要独立的可执行后端；开关与未锁屏操作均不能证明支持。当前后端不支持时，开启请求会被拒绝。").size(12));
        }
        let feature_row = container(
            row![feature_icon(feature), details.width(Length::Fill), toggle,]
                .spacing(18)
                .align_y(Alignment::Center),
        )
        .padding([20, 0]);
        if feature == ControlSetting::LockedUse {
            content = content.push(settings_group_card(applications));
            content = content.push(settings_group_card(feature_row));
            applications = column![].width(Length::Fill);
        } else {
            if feature != ControlSetting::NativeApps {
                applications = applications.push(settings_divider());
            }
            applications = applications.push(feature_row);
        }
    }
    for (chrome, heading) in [(true, "Chrome 站点管理"), (false, "始终允许的应用与文档")]
    {
        let mut grants = column![].spacing(12);
        let mut count = 0;
        for scope in &state.approvals {
            if matches!(scope, ControlGrantScope::ChromeOrigin(_)) != chrome {
                continue;
            }
            count += 1;
            let (kind, identity) = match scope {
                ControlGrantScope::Application(id) => ("应用 Bundle ID", id),
                ControlGrantScope::ChromeOrigin(id) => ("站点 Origin", id),
                ControlGrantScope::ExcelDocument(id) => ("Excel 文档 ID", id),
                ControlGrantScope::WordDocument(id) => ("Word 文档 ID", id),
            };
            grants = grants.push(
                row![
                    column![
                        text(kind).size(12).style(settings_muted_text_style),
                        text(identity).size(13)
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                    action("撤销授权", Event::Revoke(scope.clone()), !state.busy()),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            );
        }
        if count == 0 {
            grants = grants.push(
                container(
                    text(if state.status.is_some() {
                        "暂无授权"
                    } else {
                        "授权列表尚未加载"
                    })
                    .size(13)
                    .style(settings_muted_text_style),
                )
                .center_x(Length::Fill)
                .padding([20, 0]),
            );
        }
        if chrome {
            grants = grants.push(
                text("站点授权由实际操作时明确批准；撤销后须重新授权。原有浏览器配置保持独立。")
                    .size(12)
                    .style(settings_muted_text_style),
            );
        }
        content = content.push(text(heading).size(17));
        content = content.push(settings_group_card(container(grants).padding([16, 0])));
    }
    let body = scrollable(container(content).padding([0, 7]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill);
    column![
        header,
        text("开关保存使用策略；系统权限与扩展连接由当前网关主机检查。")
            .size(12)
            .style(settings_muted_text_style),
        body
    ]
    .spacing(24)
    .height(Length::Fill)
    .into()
}

fn management_page(state: &ComputerControlSettingsState, feature: ControlSetting) -> Element<'_, Event> {
    let chrome = feature == ControlSetting::Chrome;
    let title = if chrome { "Google Chrome" } else { "任意应用" };
    let mut content = column![
        action("返回电脑操控", Event::ManageClose, true),
        text(title).size(28),
        text(state.feature(feature).map(|report| readiness_label(report.readiness)).unwrap_or("尚未取得服务端状态"))
            .size(13).style(settings_muted_text_style),
        action("刷新状态", Event::Refresh, !state.busy()),
    ].spacing(20).width(Length::Fill);
    if let Some(error) = &state.error {
        content = content.push(settings_error_banner(error));
    }
        let mut permissions = column![
            text("系统权限").size(17),
            text("请为 ShiJu Computer Use 执行应用分别授予辅助功能和屏幕录制权限。远程网关需在目标电脑操作；打开设置或发出申请不代表已授权。").size(13),
        ].spacing(12);
        permissions = permissions.push(text(format!("目标网关：{}", state.target_host)).size(13));
        if let Some(report) = &state.native_service {
            let label = match report.state {
                vw_api_computer_control::NativeServiceState::Available => "执行应用可用",
                vw_api_computer_control::NativeServiceState::NotInstalled => "执行应用未安装",
                vw_api_computer_control::NativeServiceState::Disconnected => "执行应用断开或身份验证失败",
                vw_api_computer_control::NativeServiceState::Unsupported => "目标系统暂不支持",
            };
            permissions = permissions.push(text(label).size(13));
        }
        if cfg!(target_os = "macos") {
            permissions = permissions.push(row![
                action("申请辅助功能权限", Event::OpenSystemPermission { screen_recording: false }, !state.busy()),
                action("申请屏幕录制权限", Event::OpenSystemPermission { screen_recording: true }, !state.busy()),
            ].spacing(10).wrap());
        }
        if let Some(report) = &state.native_permission_report {
            permissions = permissions.push(text(format!("申请进程：{}", report.executable)).size(13));
            permissions = permissions.push(text(format!("辅助功能：{}；屏幕录制：{}",
                if report.accessibility { "已授权" } else { "未授权" },
                if report.screen_recording { "已授权" } else { "未授权" })).size(13));
            if !report.screen_recording {
                permissions = permissions.push(text("屏幕录制尚未生效；若已在系统中启用，则待重启执行应用后刷新确认。").size(13));
            }
            permissions = permissions.push(text("请在系统授权列表中启用上述进程对应的应用。授权后刷新；屏幕录制仍未生效时，按系统提示退出并重新启动该应用。开发程序重新构建后可能需要重新授权。").size(13));
        }
        content = content.push(settings_group_card(permissions));
    let mut grants = column![text(if chrome { "已授权网站" } else { "已授权应用" }).size(17)].spacing(12);
    let mut count = 0;
    for scope in &state.approvals {
        let identity = match (chrome, scope) {
            (true, ControlGrantScope::ChromeOrigin(identity)) => identity,
            (false, ControlGrantScope::Application(identity)) => identity,
            _ => continue,
        };
        count += 1;
        grants = grants.push(row![
            text(identity).size(13).width(Length::Fill),
            action("撤销授权", Event::Revoke(scope.clone()), !state.busy()),
        ].spacing(12).align_y(Alignment::Center));
    }
    if count == 0 {
        grants = grants.push(text(if state.status.is_some() { "暂无授权" } else { "授权列表尚未加载" }).size(13));
    }
    grants = grants.push(text("持久授权在实际操作的确认提示中授予，可在此撤销。").size(13).style(settings_muted_text_style));
    content = content.push(settings_group_card(grants));
    scrollable(container(content).padding([0, 7]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill).into()
}

fn feature_description(feature: ControlSetting) -> &'static str {
    match feature {
        ControlSetting::NativeApps => "允许识矩观察和操作电脑上的应用",
        ControlSetting::Chrome => "通过浏览器扩展访问网页并执行操作",
        ControlSetting::Excel => "连接 Excel 加载项，读取和编辑电子表格",
        ControlSetting::Word => "连接 Word 加载项，读取和编辑文档",
        ControlSetting::LockedUse => "允许在电脑锁定时继续执行任务，需要后端支持",
    }
}

fn feature_icon(feature: ControlSetting) -> Element<'static, Event> {
    let (color, path) = match feature {
        ControlSetting::NativeApps => ("#818cf8", "M4 4h24v17H4z M11 27h10 M16 21v6"),
        ControlSetting::Chrome => (
            "#38bdf8",
            "M28 16a12 12 0 1 1-24 0 12 12 0 0 1 24 0 M4 16h24 M16 4c-7 7-7 17 0 24 M16 4c7 7 7 17 0 24",
        ),
        ControlSetting::Excel => ("#34d399", "M7 4h18v24H7z M7 12h18 M7 20h18 M16 12v16"),
        ControlSetting::Word => ("#60a5fa", "M7 4h18v24H7z M12 11h8 M12 16h8 M12 21h5"),
        ControlSetting::LockedUse => {
            ("#fbbf24", "M7 14h18v14H7z M11 14V9a5 5 0 0 1 10 0v5 M16 20v3")
        }
    };
    let data = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 48 48"><rect width="48" height="48" rx="12" fill="{color}" fill-opacity="0.12"/><path d="{path}" transform="translate(8 8)" fill="none" stroke="{color}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>"#
    );
    svg(svg::Handle::from_memory(data.into_bytes())).width(48).height(48).into()
}

pub fn host_overlays<'a, M: Clone + 'static>(
    state: &'a ComputerControlSettingsState,
    identity: &ControlIdentity,
    base: Element<'a, M>,
    map: fn(Event) -> M,
) -> Element<'a, M> {
    if state.identity.as_ref() != Some(identity) {
        return base;
    }
    let Some(draft) = &state.pairing else {
        return base;
    };
    let close = message(Event::PairClose);
    let header = settings_modal_header(
        text("确认 Office 文档配对").size(20),
        settings_close_button(close.clone()),
    );
    let mut body = column![
        text(format!("应用：{}", if draft.feature == ControlSetting::Excel { "Excel" } else { "Word" })),
        text(format!("聊天会话：{}", draft.session)).size(13),
        text("输入加载项提供的稳定文档 ID。确认会为此文档授予持久权限，并生成一次性配对码；可以在授权列表中撤销。").size(13),
    ].spacing(14);
    if let Some(credential) = &draft.credential {
        body = body.push(
            text("请在 60 秒内将此一次性配对码填入对应加载项；关闭弹窗即清除本地显示。").size(13),
        );
        // Default font also handles non-ASCII document identities safely.
        body = body.push(text(&credential.0).size(14));
        body = body.push(action("复制一次性配对码", Event::CopyCredential, true));
    } else {
        let input = text_input("加载项中的文档 ID", &draft.document)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style);
        let input =
            if state.saving { input } else { input.on_input(|v| message(Event::PairDocument(v))) };
        body = body.push(input);
    }
    let actions = row![
        action("关闭", Event::PairClose, !state.saving),
        button(text(if state.saving { "正在确认…" } else { "确认授权并生成配对码" }).size(13))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe(
                (!state.busy() && draft.credential.is_none() && !draft.document.trim().is_empty())
                    .then(|| message(Event::PairConfirm))
            ),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .wrap();
    let card = settings_modal_layout(header, body, Some(actions.into())).width(620).height(460);
    settings_modal_overlay(
        Some(base),
        map(close),
        Element::from(container(card).padding(12)).map(map),
    )
}

#[cfg(test)]
#[path = "system_settings_computer_control_tests.rs"]
mod tests;
