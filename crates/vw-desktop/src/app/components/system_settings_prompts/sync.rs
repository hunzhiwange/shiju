//! Server-owned synchronization: closing this panel never cancels or rolls back work.
use crate::app::{Message, state::AgentAppsContext, components::system_settings_common::*};
use crate::app::message::settings::prompts::sync::{Event, message};
use iced::{Element, widget::{button, column, row, text}};
use vw_gateway_client::{AgentAppPromptSync, PiPromptResourcesScope, AgentAppPromptSyncTargetStatus};
use vw_gateway_client::vw_api_types::agent_apps::{AppId, AppOperationPhase};

#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub generation: u64,
    pub context: Option<AgentAppsContext>,
    pub scope: Option<PiPromptResourcesScope>,
    pub report: Option<AgentAppPromptSync>,
    pub error: Option<String>,
    // An ambiguous submission must not be automatically replayed.
    pub submitted: bool,
}
impl State {
    pub fn terminal(&self) -> bool {
        self.report.as_ref().is_some_and(|r| r.operation.phase == AppOperationPhase::Terminal)
    }
    pub fn retry_targets(&self) -> Vec<AppId> {
        if !self.terminal() { return Vec::new(); }
        self.report.iter().flat_map(|r| &r.targets)
            .filter(|t| matches!(t.status, AgentAppPromptSyncTargetStatus::Failed | AgentAppPromptSyncTargetStatus::NotExecuted))
            .map(|t| t.app_id).collect()
    }
}
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.open { return base; }
    let close = message(Event::Close);
    let mut body = column![text("同步保存的选择；关闭面板不影响后端操作。取消只阻止尚未开始的目标，已完成写入不会回滚。")].spacing(10);
    if let Some(report) = &state.report {
        body = body.push(text(format!("操作 {} · {:?} · 已返回 {} 个目标", report.operation.operation_id, report.operation.status, report.targets.len())));
        for target in &report.targets {
            let status = match target.status {
                AgentAppPromptSyncTargetStatus::Succeeded => "已完成",
                AgentAppPromptSyncTargetStatus::Failed => "失败",
                AgentAppPromptSyncTargetStatus::Skipped => "已跳过",
                AgentAppPromptSyncTargetStatus::NotExecuted => "未执行",
            };
            body = body.push(text(format!("{:?} · {} · {}{}", target.app_id, status, target.reason.as_deref().unwrap_or(""), if target.duplicate_selection { " · 多选警告：已按保存顺序采用首项" } else { "" })));
        }
    } else { body = body.push(text("尚无操作结果；空结果不代表同步成功。")); }
    if state.busy { body = body.push(text("正在请求后端…")); }
    if let Some(error) = &state.error { body = body.push(text(error)); }
    let footer = row![
        button(text("全部同步")).padding([8,12]).style(primary_action_btn_style).on_press_maybe((!state.busy && !state.submitted).then(|| message(Event::Start))),
        button(text("刷新进度")).padding([8,12]).style(rounded_action_btn_style).on_press_maybe((!state.busy && state.report.is_some()).then(|| message(Event::Refresh))),
        button(text("取消剩余目标")).padding([8,12]).style(rounded_action_btn_style).on_press_maybe((!state.busy && state.report.is_some() && !state.terminal()).then(|| message(Event::Cancel))),
        button(text("重试未完成目标")).padding([8,12]).style(rounded_action_btn_style).on_press_maybe((!state.busy && !state.retry_targets().is_empty()).then(|| message(Event::Retry))),
    ].spacing(8);
    let header = settings_modal_header(text("全部同步进度").size(20), settings_close_button(close.clone()));
    settings_modal_overlay(Some(base), close, settings_modal_layout(header, body, Some(footer.into())).width(900).height(600))
}
