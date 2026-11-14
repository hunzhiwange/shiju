//! Task snapshots come from the service; closing only stops observation.
use crate::app::{App, Message};
use crate::app::message::settings::skills::jobs_view::{Event, message, terminal};
use crate::app::components::system_settings_common::*;
use iced::{Element, widget::{button, column, container, row, text}};
use vw_gateway_client::vw_api_types::agent_apps_skills::jobs_api::SkillJobPhase;

pub(super) fn view<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.skills_settings.jobs;
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
