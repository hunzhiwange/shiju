//! Five independent server-reported capabilities and explicit authorization management.
use super::system_settings_common::*;
use crate::app::{App, Message};
use crate::app::message::settings::computer_control::{Event, message};
use crate::app::state::{ComputerControlSettingsState, ControlIdentity};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::{Alignment, Element, Length};
use vw_gateway_client::vw_api_types::computer_control::*;

pub(crate) const FEATURES: [(ControlSetting, &str); 5] = [
    (ControlSetting::NativeApps, "任意应用"),
    (ControlSetting::Chrome, "Chrome"),
    (ControlSetting::Excel, "Excel"),
    (ControlSetting::Word, "Word"),
    (ControlSetting::LockedUse, "锁屏使用"),
];
pub(crate) fn readiness_label(readiness: ControlReadiness) -> &'static str {
    match readiness {
        ControlReadiness::Disabled => "已关闭",
        ControlReadiness::NotInstalled => "后端未安装",
        ControlReadiness::Unauthorized => "缺少系统权限：请在网关主机授予辅助功能及屏幕录制权限",
        ControlReadiness::Disconnected => "未连接：请连接 Chrome 扩展或 Office 加载项",
        ControlReadiness::Ready => "服务端报告已就绪",
        ControlReadiness::Unsupported => "当前平台或后端不支持",
    }
}
fn action(label: &'static str, event: Event, enabled: bool) -> Element<'static, Message> {
    button(text(label).size(13)).padding([8, 14]).style(rounded_action_btn_style)
        .on_press_maybe(enabled.then(|| message(event))).into()
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    if app.computer_control_settings.identity.as_ref() != Some(&ControlIdentity::current()) {
        return column![text("电脑操控"), text("网关已变化，请重新加载状态"), action("加载状态", Event::Refresh, true)].spacing(14).into();
    }
    page(&app.computer_control_settings)
}
pub(crate) fn page(state: &ComputerControlSettingsState) -> Element<'_, Message> {
    let header = column![
        text("电脑操控").size(22),
        text("状态来自当前网关主机。开启开关只保存策略；系统权限、连接与实际后端能力由服务端检查。安装扩展和加载项需自行完成。")
            .size(13).style(settings_muted_text_style),
        row![
            action(if state.loading { "加载中…" } else { "刷新状态" }, Event::Refresh, !state.busy()),
            button(text(if state.stopping { "正在停止…" } else { "停止所有电脑操控" }).size(13))
                .padding([8, 14]).style(danger_action_btn_style)
                .on_press_maybe((!state.stopping).then(|| message(Event::Stop))),
        ].spacing(10).align_y(Alignment::Center).wrap(),
    ].spacing(16).width(Length::Fill);
    let mut content = column![].spacing(16).width(Length::Fill);
    if state.saving { content = content.push(text("正在等待服务端确认…").size(13)); }
    if let Some(error) = &state.error { content = content.push(settings_error_banner(error)); }
    for (feature, title) in FEATURES {
        let report = state.feature(feature);
        let mut toggle = settings_toggle(report.is_some_and(|s| s.enabled));
        if report.is_some() && !state.busy() {
            toggle = toggle.on_toggle(move |enabled| message(Event::Toggle(feature, enabled)));
        }
        let mut details = column![
            row![text(title).size(16).width(Length::Fill), toggle].align_y(Alignment::Center),
            text(report.map(|s| readiness_label(s.readiness)).unwrap_or("尚未取得服务端状态"))
                .size(13).style(settings_muted_text_style),
        ].spacing(10);
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
            details = details.push(text(format!("系统授权：{authorization} · 连接：{connection} · 后端：{backend}")).size(12));
            if !report.capabilities.is_empty() {
                let capabilities = report.capabilities.iter().map(|capability| match capability {
                    ControlCapability::Observe => "观察",
                    ControlCapability::Screenshot => "截图",
                    ControlCapability::Input => "输入",
                    ControlCapability::StructuredRead => "结构化读取",
                    ControlCapability::StructuredWrite => "结构化写入",
                    ControlCapability::LockedExecution => "锁屏执行",
                }).collect::<Vec<_>>().join("、");
                details = details.push(text(format!("后端能力：{capabilities}")).size(12));
            }
        }
        if matches!(feature, ControlSetting::Excel | ControlSetting::Word) {
            details = details.push(action("确认 Office 文档配对", Event::PairOpen(feature),
                !state.busy() && report.is_some_and(|s| s.enabled)));
        }
        if feature == ControlSetting::LockedUse {
            details = details.push(text("锁屏需要独立的可执行后端；开关与未锁屏操作均不能证明支持。当前后端不支持时，开启请求会被拒绝。").size(12));
        }
        content = content.push(settings_group_card(details));
    }
    for (chrome, heading) in [(true, "Chrome 站点管理"), (false, "始终允许的应用与文档")] {
        let mut grants = column![text(heading).size(17)].spacing(12);
        let mut count = 0;
        for scope in &state.approvals {
            if matches!(scope, ControlGrantScope::ChromeOrigin(_)) != chrome { continue; }
            count += 1;
            let (kind, identity) = match scope {
                ControlGrantScope::Application(id) => ("应用 Bundle ID", id),
                ControlGrantScope::ChromeOrigin(id) => ("站点 Origin", id),
                ControlGrantScope::ExcelDocument(id) => ("Excel 文档 ID", id),
                ControlGrantScope::WordDocument(id) => ("Word 文档 ID", id),
            };
            grants = grants.push(row![
                column![text(kind).size(12).style(settings_muted_text_style), text(identity).size(13)].spacing(4).width(Length::Fill),
                action("撤销授权", Event::Revoke(scope.clone()), !state.busy()),
            ].spacing(12).align_y(Alignment::Center));
        }
        if count == 0 { grants = grants.push(text(if state.status.is_some() { "没有持久授权" } else { "授权列表尚未加载" }).size(13)); }
        if chrome { grants = grants.push(text("站点授权由实际操作时明确批准；撤销后须重新授权。原有浏览器配置保持独立。").size(12).style(settings_muted_text_style)); }
        content = content.push(settings_group_card(grants));
    }
    let body = scrollable(container(content).padding([0, 7]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill);
    column![header, body].spacing(16).height(Length::Fill).into()
}

pub(crate) fn view_overlays<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let state = &app.computer_control_settings;
    if state.identity.as_ref() != Some(&ControlIdentity::current()) { return base; }
    let Some(draft) = &state.pairing else { return base; };
    let close = message(Event::PairClose);
    let header = settings_modal_header(text("确认 Office 文档配对").size(20), settings_close_button(close.clone()));
    let mut body = column![
        text(format!("应用：{}", if draft.feature == ControlSetting::Excel { "Excel" } else { "Word" })),
        text(format!("聊天会话：{}", draft.session)).size(13),
        text("输入加载项提供的稳定文档 ID。确认会为此文档授予持久权限，并生成一次性配对码；可以在授权列表中撤销。").size(13),
    ].spacing(14);
    if let Some(credential) = &draft.credential {
        body = body.push(text("请在 60 秒内将此一次性配对码填入对应加载项；关闭弹窗即清除本地显示。").size(13));
        // Default font also handles non-ASCII document identities safely.
        body = body.push(text(&credential.0).size(14));
        body = body.push(action("复制一次性配对码", Event::CopyCredential, true));
    } else {
        let input = text_input("加载项中的文档 ID", &draft.document)
            .padding(SETTINGS_CONTROL_PADDING).size(SETTINGS_CONTROL_TEXT_SIZE).style(settings_text_input_style);
        let input = if state.saving { input } else { input.on_input(|v| message(Event::PairDocument(v))) };
        body = body.push(input);
    }
    let actions = row![
        action("关闭", Event::PairClose, !state.saving),
        button(text(if state.saving { "正在确认…" } else { "确认授权并生成配对码" }).size(13))
            .padding([8, 14]).style(primary_action_btn_style)
            .on_press_maybe((!state.busy() && draft.credential.is_none() && !draft.document.trim().is_empty()).then(|| message(Event::PairConfirm))),
    ].spacing(10).align_y(Alignment::Center).wrap();
    let card = settings_modal_layout(header, body, Some(actions.into())).width(620).height(460);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}

#[cfg(test)]
#[path = "system_settings_computer_control_tests.rs"]
mod tests;
