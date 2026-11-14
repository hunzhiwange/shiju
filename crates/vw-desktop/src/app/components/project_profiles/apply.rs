//! Executor preview and result use the shared themed, scrolling settings modal.
use crate::app::{Message, message::project_profiles::{Event, ApplyEvent, message}, state::project_profiles::{State, PreviewAction}};
use crate::app::components::system_settings_common::*;
use iced::{Element, widget::{button, column, row, text}};
use vw_gateway_client::vw_api_types::project_profiles::*;

pub(super) fn action(profile: &ProjectProfile, scope: ProjectProfileScope, enabled: bool) -> Element<'_, Message> {
    button(text("预览并应用")).padding([8, 12]).style(primary_action_btn_style)
        .on_press_maybe(enabled.then(|| message(Event::Apply(scope, ApplyEvent::Open(profile.id.clone(), profile.version))))).into()
}
pub(super) fn overlay<'a>(state: &'a State, scope: ProjectProfileScope, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(form) = state.application.as_ref().filter(|f| f.scope == scope) else { return base; };
    let close = message(Event::Apply(scope, ApplyEvent::Close));
    let header = settings_modal_header(text("预览与一键应用").size(20), settings_close_button(close.clone()));
    let mut body = column![text(&form.name)].spacing(10);
    if form.loading { body = body.push(text(if form.submitted { "等待执行主机确认，尚未声明成功" } else { "正在读取执行主机预览" })); }
    if let Some(error) = form.error { body = body.push(text(error)); }
    if let Some(preview) = &form.preview {
        body = body.push(text(if !preview.captured { "此组未拍快照：只选择为当前组合，不写入四类配置。" }
            else { "未拍的类别保持原状；已拍的空集合恢复为空，空提示词停止激活。" }));
        if preview.captured && preview.items.is_empty() { body = body.push(text("执行主机未列出差异。")); }
        for item in &preview.items {
            let action = match item.action { PreviewAction::Select => "选择", PreviewAction::Enable => "启用", PreviewAction::Disable => "停用", PreviewAction::Missing => "引用缺失" };
            body = body.push(text(format!("{} · {} · {}", category(item.category), item.reference_id, action)));
        }
    }
    if let Some(result) = &form.result {
        let label = match result.status { ProjectProfileApplyStatus::Applied => "执行主机确认已应用", ProjectProfileApplyStatus::Partial => "部分应用失败", ProjectProfileApplyStatus::Uncaptured => "未拍快照，未应用配置", ProjectProfileApplyStatus::Cancelled => "执行主机确认已取消", ProjectProfileApplyStatus::Failed => "应用失败" };
        body = body.push(text(label)).push(text(if result.selected { "已选择为当前组合" } else { "未确认当前选择" }));
        for item in &result.items {
            body = body.push(text(format!("{} · {} · {:?}{}", category(item.category), item.reference_id.as_deref().unwrap_or("空集合"), item.status,
                item.reason_code.as_ref().map(|r| format!(" · {r}")).unwrap_or_default())));
        }
    }
    let actions = row![
        button(text("关闭")).padding([8, 14]).style(rounded_action_btn_style).on_press_maybe((!form.loading).then_some(close.clone())),
        button(text("确认应用一次")).padding([8, 14]).style(primary_action_btn_style)
            .on_press_maybe((!form.loading && !form.submitted && form.preview.is_some()).then(|| message(Event::Apply(scope, ApplyEvent::Confirm))))
    ].spacing(10);
    let card = settings_modal_layout(header, body, Some(actions.into())).width(620).height(520);
    settings_modal_overlay(Some(base), close, card)
}
fn category(category: ProjectProfileCategory) -> &'static str {
    match category { ProjectProfileCategory::Provider => "供应商", ProjectProfileCategory::Mcp => "MCP", ProjectProfileCategory::Skill => "技能", ProjectProfileCategory::Prompt => "提示词" }
}
