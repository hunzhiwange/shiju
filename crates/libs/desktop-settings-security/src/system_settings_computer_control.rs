//! Five independent server-reported capabilities and explicit authorization management.
use desktop_i18n::I18n;
use crate::computer_control_state::{ComputerControlSettingsState, ControlIdentity};
use crate::computer_control_update::{Event, message};
use desktop_ui_controls::system_settings_common::*;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{pick_list, button, column, container, row, scrollable, svg, text, text_input};
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
        text(i18n.__("电脑操控")).size(28),
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
    let mut applications = column![].width(Length::Fill);
    if state.saving {
        content = content.push(text(i18n.__("正在等待服务端确认…")).size(13));
    }
    if let Some(error) = &state.error {
        content = content.push(settings_error_banner(desktop_i18n::settings::message(i18n, error)));
    }
    for (feature, title) in FEATURES {
        let report = state.feature(feature);
        let mut toggle = settings_toggle(report.is_some_and(|s| s.enabled));
        if report.is_some() && !state.busy() {
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
                        ControlCapability::StructuredRead => i18n.__("结构化读取"),
                        ControlCapability::StructuredWrite => i18n.__("结构化写入"),
                        ControlCapability::LockedExecution => i18n.__("锁屏执行"),
                    })
                    .collect::<Vec<_>>()
                    .join("、");
                details = details.push(
                    text(desktop_i18n::__!(i18n, "能力：%s", (capabilities).to_string()).expect("valid settings message")).size(11).style(settings_muted_text_style),
                );
            }
        }
        if matches!(feature, ControlSetting::NativeApps | ControlSetting::Chrome) {
            let label = if feature == ControlSetting::NativeApps
                && state.native_service.as_ref().is_some_and(|report| report.state == NativeServiceState::NotInstalled)
            {
                i18n.__("安装与权限")
            } else {
                i18n.__("管理权限")
            };
            details = details.push(action(i18n, label, Event::Manage(feature), true));
        }
        if matches!(feature, ControlSetting::Excel | ControlSetting::Word) {
            details = details.push(action(i18n,
                i18n.__("确认 Office 文档配对"),
                Event::PairOpen(feature),
                !state.busy() && report.is_some_and(|s| s.enabled),
            ));
        }
        if feature == ControlSetting::LockedUse {
            details = details.push(text(i18n.__("锁屏需要独立的可执行后端；开关与未锁屏操作均不能证明支持。当前后端不支持时，开启请求会被拒绝。")).size(12));
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
    for (chrome, heading) in [(true, i18n.__("Chrome 站点管理")), (false, i18n.__("始终允许的应用与文档"))]
    {
        let mut grants = column![].spacing(12);
        let mut count = 0;
        for scope in &state.approvals {
            if matches!(scope, ControlGrantScope::ChromeOrigin(_)) != chrome {
                continue;
            }
            count += 1;
            let (kind, identity) = match scope {
                ControlGrantScope::Application(id) => (i18n.__("应用 Bundle ID"), id),
                ControlGrantScope::ChromeOrigin(id) => (i18n.__("站点 Origin"), id),
                ControlGrantScope::ExcelDocument(id) => (i18n.__("Excel 文档 ID"), id),
                ControlGrantScope::WordDocument(id) => (i18n.__("Word 文档 ID"), id),
            };
            grants = grants.push(
                row![
                    column![
                        text(i18n.__(kind)).size(12).style(settings_muted_text_style),
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
        if count == 0 {
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
        if chrome {
            grants = grants.push(
                text(i18n.__("站点授权由实际操作时明确批准；撤销后须重新授权。原有浏览器配置保持独立。"))
                    .size(12)
                    .style(settings_muted_text_style),
            );
        }
        content = content.push(text(i18n.__(heading)).size(17));
        content = content.push(settings_group_card(container(grants).padding([16, 0])));
    }
    let body = scrollable(container(content).padding([0, 7]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill);
    column![
        header,
        text(i18n.__("开关保存使用策略；系统权限与扩展连接由当前网关主机检查。"))
            .size(12)
            .style(settings_muted_text_style),
        body
    ]
    .spacing(24)
    .height(Length::Fill)
    .into()
}

fn management_page(i18n: I18n, state: &ComputerControlSettingsState, feature: ControlSetting) -> Element<'_, Event> {
    let chrome = feature == ControlSetting::Chrome;
    let title = if chrome { "Google Chrome" } else { i18n.__("任意应用") };
    let mut content = column![
        action(i18n, i18n.__("返回电脑操控"), Event::ManageClose, true),
        text(i18n.__(title)).size(28),
        text(i18n.__(state.feature(feature).map(|report| feature_readiness_label(feature, report.readiness)).unwrap_or("尚未取得服务端状态")))
            .size(13).style(settings_muted_text_style),
        action(i18n, i18n.__("刷新状态"), Event::Refresh, !state.busy()),
    ].spacing(20).width(Length::Fill);
    if let Some(error) = &state.error {
        content = content.push(settings_error_banner(desktop_i18n::settings::message(i18n, error)));
    }
    if chrome {
        content = content.push(browser_permissions_form(i18n, state));
        content = content.push(settings_group_card(column![
            text(i18n.__("浏览器连接")).size(17),
            text(i18n.__("当前通过 ShiJu Computer Control 扩展连接网关。使用目标网页前，点击扩展授予当前标签页访问权限；导航或打开新标签页后需要重新授权。")).size(13),
            text(i18n.__("Chrome 可以同时供你和 Codex 使用，但 Codex 的扩展连接与授权不会自动授予识矩。请勿卸载 Codex 扩展或清除现有浏览器配置。")).size(13),
            text(i18n.__("历史记录、网站工具和完整 CDP 尚无执行后端；浏览、上传、下载的策略在下方配置并保存。")).size(13).style(settings_muted_text_style),
        ].spacing(12)));
    } else {
        let service_state = state.native_service.as_ref().map(|report| report.state);
        let mut permissions = column![
            text(i18n.__("系统权限")).size(17),
            text(i18n.__("请在网关所在的 Mac 上为 ShiJu Computer Use 授予辅助功能和屏幕录制权限。")).size(13),
        ].spacing(12);
        permissions = permissions.push(text(desktop_i18n::__!(i18n, "目标网关：%s", (state.target_host).to_string()).expect("valid settings message")).size(13));
        if let Some(service_state) = service_state {
            let label = match service_state {
                NativeServiceState::Available => i18n.__("执行应用可用"),
                NativeServiceState::NotInstalled => i18n.__("执行应用未安装"),
                NativeServiceState::Disconnected => i18n.__("执行应用断开或身份验证失败"),
                NativeServiceState::Unsupported => i18n.__("目标系统暂不支持"),
            };
            permissions = permissions.push(text(label).size(13));
        }
        if let Some(install) = &state.native_install {
            let mut actions = row![action(i18n, i18n.__("复制安装地址"), Event::CopyNativeInstallPath, !state.busy())]
                .spacing(10);
            permissions = permissions
                .push(text(i18n.__("安装包内位置")).size(13))
                .push(text(install.app_path.as_str()).size(13).wrapping(iced::widget::text::Wrapping::WordOrGlyph));
            if service_state == Some(NativeServiceState::NotInstalled) {
                permissions = permissions.push(text(i18n.__("在本机终端运行安装命令，完成后刷新状态。")).size(13));
                actions = actions.push(action(i18n, i18n.__("复制安装命令"), Event::CopyNativeInstallCommand, !state.busy()));
            }
            permissions = permissions.push(actions.wrap());
        } else if service_state == Some(NativeServiceState::NotInstalled) {
            let hint = if state.target_is_local {
                i18n.__("请安装包含 ShiJu Computer Use.app 的完整识矩 macOS 安装包，然后刷新状态。")
            } else {
                i18n.__("请在运行网关的 Mac 上打开识矩，从该电脑的安装包安装执行应用，然后刷新状态。")
            };
            permissions = permissions.push(text(hint).size(13));
        }
        if cfg!(target_os = "macos") && state.target_is_local
            && matches!(service_state, Some(NativeServiceState::Available | NativeServiceState::Disconnected))
        {
            permissions = permissions
                .push(text(i18n.__("打开对应的系统设置，将授权窗口里的应用图标拖放到列表中，开启开关后返回刷新。"))
                    .size(13))
                .push(row![
                    action(i18n, i18n.__("打开辅助功能"), Event::OpenNativePermissionGuide { screen_recording: false }, !state.busy()),
                    action(i18n, i18n.__("打开屏幕录制"), Event::OpenNativePermissionGuide { screen_recording: true }, !state.busy()),
                ].spacing(10).wrap());
        }
        if let Some(report) = &state.native_permission_report {
            permissions = permissions.push(text(desktop_i18n::__!(i18n, "辅助功能：%s；屏幕录制：%s", (if report.accessibility { i18n.__("已授权") } else { i18n.__("未授权") }).to_string(), (if report.screen_recording { i18n.__("已授权") } else { i18n.__("未授权") }).to_string()).expect("valid settings message")).size(13));
        }
        content = content.push(settings_group_card(container(permissions).padding([16, 0])));
    }
    let mut grants = column![text(if chrome { i18n.__("已授权网站") } else { i18n.__("已授权应用") }).size(17)].spacing(12);
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
            action(i18n, i18n.__("撤销授权"), Event::Revoke(scope.clone()), !state.busy()),
        ].spacing(12).align_y(Alignment::Center));
    }
    if count == 0 {
        grants = grants.push(text(if state.status.is_some() { i18n.__("暂无授权") } else { i18n.__("授权列表尚未加载") }).size(13));
    }
    grants = grants.push(text(i18n.__("持久授权在实际操作的确认提示中授予，可在此撤销。")).size(13).style(settings_muted_text_style));
    content = content.push(settings_group_card(grants));
    scrollable(container(content).padding([0, 7]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill).into()
}

fn browser_permissions_form(i18n: I18n, state: &ComputerControlSettingsState) -> Element<'_, Event> {
    let Some(snapshot) = &state.browser else { return text(i18n.__("网站权限尚未加载，请刷新状态")).size(13).into(); };
    let mut form = column![text(i18n.__("网站权限")).size(17), text(i18n.__("设置默认权限，并为特定网站配置例外。保存后生效，现有操控会话会停止。")).size(13)].spacing(14);
    for (origin, policy) in std::iter::once((None, &snapshot.permissions.defaults))
        .chain(snapshot.permissions.origins.iter().map(|(o, p)| (Some(o.clone()), p))) {
        let mut controls = column![text(origin.clone().unwrap_or_else(|| i18n.__("默认").into())).size(15)].spacing(10);
        for (field, label, value) in [(0, i18n.__("浏览"), policy.access), (1, i18n.__("下载"), policy.downloads), (2, i18n.__("上传"), policy.uploads), (3, i18n.__("调试 / CDP（后端尚不支持）"), policy.full_cdp_access), (4, i18n.__("网站工具（后端尚不支持）"), policy.website_tools)] {
            let key = origin.clone();
            let mut options = vec![BrowserPermission::Ask, BrowserPermission::Allow, BrowserPermission::Deny];
            if field >= 3 { options = vec![BrowserPermission::Deny]; }
            let choice = |value| BrowserPermissionChoice { value, label: browser_permission_label(i18n, value) };
            let options: Vec<_> = options.into_iter().map(choice).collect();
            let select = pick_list(options, value.map(choice), move |v| message(Event::BrowserEdit { origin: key.clone(), field, value: Some(v.value) }))
                .padding(SETTINGS_CONTROL_PADDING).text_size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_pick_list_style).menu_style(settings_pick_list_menu_style);
            let mut control = row![text(label).size(13).width(Length::Fill), select].spacing(10).align_y(Alignment::Center);
            if origin.is_some() && value.is_some() {
                let key = origin.clone();
                control = control.push(action(i18n, i18n.__("使用默认设置"), Event::BrowserEdit { origin: key, field, value: None }, !state.busy()));
            }
            controls = controls.push(control);
        }
        if let Some(origin) = origin { controls = controls.push(action(i18n, i18n.__("移除网站例外"), Event::BrowserRemove(origin), !state.busy())); }
        form = form.push(settings_group_card(controls));
    }
    form = form.push(row![
        text_input(i18n.__("精确网站 Origin，例如 https://example.com"), &state.browser_origin)
            .on_input(|v| message(Event::BrowserOrigin(v))).padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE).style(settings_text_input_style),
        action(i18n, i18n.__("添加网站"), Event::BrowserEdit { origin: Some(state.browser_origin.trim().to_owned()), field: 0, value: None }, !state.busy() && !state.browser_origin.trim().is_empty()),
    ].spacing(10));
    form.push(action(i18n, i18n.__("保存网站权限"), Event::BrowserSave, !state.busy())).into()
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

pub fn host_overlays<'a, M: Clone + 'static>(i18n: I18n,
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
        text(i18n.__("确认 Office 文档配对")).size(20),
        settings_close_button(close.clone()),
    );
    let mut body = column![
        text(desktop_i18n::__!(i18n, "应用：%s", (if draft.feature == ControlSetting::Excel { "Excel" } else { "Word" }).to_string()).expect("valid settings message")),
        text(desktop_i18n::__!(i18n, "聊天会话：%s", (draft.session).to_string()).expect("valid settings message")).size(13),
        text(i18n.__("输入加载项提供的稳定文档 ID。确认会为此文档授予持久权限，并生成一次性配对码；可以在授权列表中撤销。")).size(13),
    ].spacing(14);
    if let Some(credential) = &draft.credential {
        body = body.push(
            text(i18n.__("请在 60 秒内将此一次性配对码填入对应加载项；关闭弹窗即清除本地显示。")).size(13),
        );
        // Default font also handles non-ASCII document identities safely.
        body = body.push(text(&credential.0).size(14));
        body = body.push(action(i18n, i18n.__("复制一次性配对码"), Event::CopyCredential, true));
    } else {
        let input = text_input(i18n.__("加载项中的文档 ID"), &draft.document)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style);
        let input =
            if state.saving { input } else { input.on_input(|v| message(Event::PairDocument(v))) };
        body = body.push(input);
    }
    let actions = row![
        action(i18n, i18n.__("关闭"), Event::PairClose, !state.saving),
        button(text(if state.saving { i18n.__("正在确认…") } else { i18n.__("确认授权并生成配对码") }).size(13))
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

#[derive(Clone, Copy, PartialEq)]
struct BrowserPermissionChoice { value: BrowserPermission, label: &'static str }
impl std::fmt::Display for BrowserPermissionChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(self.label) }
}
pub fn browser_permission_label(i18n: I18n, permission: BrowserPermission) -> &'static str {
    i18n.__(match permission {
        BrowserPermission::Ask => "需要批准",
        BrowserPermission::Allow => "始终允许",
        BrowserPermission::Deny => "阻止",
    })
}
