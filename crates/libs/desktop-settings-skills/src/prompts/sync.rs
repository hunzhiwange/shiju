use desktop_ui_controls::system_settings_common::*;
#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Close,
    Start,
    Refresh,
    Cancel,
    Retry,
}
// Server-owned synchronization: closing this panel never cancels or rolls back work.
use gateway_app_prompt_resources::{
    AgentAppPromptSync, AgentAppPromptSyncTargetStatus, PiPromptResourcesScope,
};
use iced::{
    Element,
    widget::{button, column, row, text},
};
use vw_api_app_operations::AppOperationPhase;
use vw_api_types::agent_apps::AppId;

#[derive(Clone, Debug)]
pub struct State<C> {
    pub open: bool,
    pub busy: bool,
    pub generation: u64,
    pub context: Option<C>,
    pub scope: Option<PiPromptResourcesScope>,
    pub report: Option<AgentAppPromptSync>,
    pub error: Option<String>,
    // An ambiguous submission must not be automatically replayed.
    pub submitted: bool,
}
impl<C> State<C> {
    pub fn terminal(&self) -> bool {
        self.report.as_ref().is_some_and(|r| r.operation.phase == AppOperationPhase::Terminal)
    }
    pub fn retry_targets(&self) -> Vec<AppId> {
        if !self.terminal() {
            return Vec::new();
        }
        self.report
            .iter()
            .flat_map(|r| &r.targets)
            .filter(|t| {
                matches!(
                    t.status,
                    AgentAppPromptSyncTargetStatus::Failed
                        | AgentAppPromptSyncTargetStatus::NotExecuted
                )
            })
            .map(|t| t.app_id)
            .collect()
    }
}
pub fn overlay<'a, M: Clone + 'static, C: Clone + 'a>(
    state: &'a State<C>,
    base: Element<'a, M>,
    message: fn(super::Event) -> M,
) -> Element<'a, M> {
    let message_event = move |event| message(super::Event::Sync(event));
    if !state.open {
        return base;
    }
    let close = message_event(Event::Close);
    let mut body = column![text(
        "同步保存的选择；关闭面板不影响后端操作。取消只阻止尚未开始的目标，已完成写入不会回滚。"
    )]
    .spacing(10);
    if let Some(report) = &state.report {
        body = body.push(text(format!(
            "操作 {} · {:?} · 已返回 {} 个目标",
            report.operation.operation_id.as_ref(),
            report.operation.status,
            report.targets.len()
        )));
        for target in &report.targets {
            let status = match target.status {
                AgentAppPromptSyncTargetStatus::Succeeded => "已完成",
                AgentAppPromptSyncTargetStatus::Failed => "失败",
                AgentAppPromptSyncTargetStatus::Skipped => "已跳过",
                AgentAppPromptSyncTargetStatus::NotExecuted => "未执行",
            };
            body = body.push(text(format!(
                "{:?} · {} · {}{}",
                target.app_id,
                status,
                target.reason.as_deref().unwrap_or(""),
                if target.duplicate_selection {
                    " · 多选警告：已按保存顺序采用首项"
                } else {
                    ""
                }
            )));
        }
    } else {
        body = body.push(text("尚无操作结果；空结果不代表同步成功。"));
    }
    if state.busy {
        body = body.push(text("正在请求后端…"));
    }
    if let Some(error) = &state.error {
        body = body.push(text(error));
    }
    let footer = row![
        button(text("全部同步"))
            .padding([8, 12])
            .style(primary_action_btn_style)
            .on_press_maybe((!state.busy && !state.submitted).then(|| message_event(Event::Start))),
        button(text("刷新进度")).padding([8, 12]).style(rounded_action_btn_style).on_press_maybe(
            (!state.busy && state.report.is_some()).then(|| message_event(Event::Refresh))
        ),
        button(text("取消剩余目标"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe(
                (!state.busy && state.report.is_some() && !state.terminal())
                    .then(|| message_event(Event::Cancel))
            ),
        button(text("重试未完成目标"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe(
                (!state.busy && !state.retry_targets().is_empty())
                    .then(|| message_event(Event::Retry))
            ),
    ]
    .spacing(8);
    let header =
        settings_modal_header(text("全部同步进度").size(20), settings_close_button(close.clone()));
    settings_modal_overlay(
        Some(base),
        close,
        settings_modal_layout(header, body, Some(footer.into())).width(900).height(600),
    )
}

impl<C> Default for State<C> {
    fn default() -> Self {
        Self {
            open: Default::default(),
            busy: Default::default(),
            generation: Default::default(),
            context: Default::default(),
            scope: Default::default(),
            report: Default::default(),
            error: Default::default(),
            submitted: Default::default(),
        }
    }
}
