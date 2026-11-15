use desktop_provider_apps::AgentAppsContext;
use vw_api_app_skills::{jobs_api::{SkillJobResponse, SkillJobPhase}, repos_api::SkillReposQuery};
use vw_api_transport::id::TaskId;
#[derive(Debug, Clone, Default)]
pub struct State {
    pub open: bool,
    pub busy: bool,
    pub snapshot: Option<SkillJobResponse>,
    pub error: Option<String>,
    pub task_id: Option<TaskId>,
    pub context: Option<AgentAppsContext>,
    pub scope: Option<SkillReposQuery>,
    pub generation: u64,
    pub notified: bool,
}
#[derive(Debug, Clone)]
pub enum Event {
    Open, Close, Refresh, Cancel,
    Loaded(u64, Result<SkillJobResponse, String>),
    Poll(u64),
}

pub fn terminal(phase: SkillJobPhase) -> bool {
    matches!(phase, SkillJobPhase::Completed | SkillJobPhase::Failed | SkillJobPhase::Interrupted)
}

// Task snapshots come from the service; closing only stops observation.

use desktop_ui_controls::system_settings_common::*;
use iced::{Element, widget::{button, column, container, row, text}};


pub fn view<'a, M: Clone + 'static>(s: &'a State, base: Element<'a, M>, message: fn(Event) -> M) -> Element<'a, M> {
    if !s.open { return base; }
    let mut body = column![text("关闭窗口不会取消后台任务；重开后重新读取服务端状态。")].spacing(10);
    let mut cancellable = false;
    if let Some(job) = &s.snapshot {
        let label = match job.phase {
            SkillJobPhase::Started => "已开始",
            SkillJobPhase::Staged => "已暂存，可请求取消",
            SkillJobPhase::Committing => "正在提交，此阶段不可中断",
            SkillJobPhase::Completed => "已完成",
            SkillJobPhase::Failed => "失败",
            SkillJobPhase::Interrupted => "已中断",
        };
        body = body.push(text(label));
        if job.requires_reconciliation { body = body.push(text("需要核对目标目录后再继续操作")); }
        if let Some(path) = &job.installed_directory { body = body.push(text(path.clone())); }
        cancellable = !terminal(job.phase) && job.phase != SkillJobPhase::Committing;
    }
    if s.busy { body = body.push(text("正在读取状态或提交取消请求，请等待服务端最终结果…")); }
    if let Some(error) = &s.error { body = body.push(settings_error_banner(error.clone())); }
    let close = message(Event::Close);
    let footer = row![
        button("请求取消").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((cancellable && !s.busy).then_some(message(Event::Cancel))),
        button("重连 / 刷新").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(message(Event::Refresh))),
        button("关闭").padding([8, 10]).style(rounded_action_btn_style).on_press(close.clone()),
    ].spacing(8);
    let header = settings_modal_header(text("技能任务进度").size(20), settings_close_button(close.clone()));
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(super::modal_card_style).width(700).height(420);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
