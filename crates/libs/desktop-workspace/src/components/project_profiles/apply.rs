//! Executor preview and result use the shared themed, scrolling settings modal.
use crate::project_profiles::{State, PreviewAction};
use super::{Event, ApplyEvent};
use desktop_ui_controls::system_settings_common::*;
use iced::{Element, widget::{button, column, row, text}};
use vw_api_project_profiles::*;

pub(super) fn action(i18n: desktop_i18n::I18n, profile: &ProjectProfile, scope: ProjectProfileScope, enabled: bool) -> Element<'_, Event> {
    button(text(i18n.__("预览并应用"))).padding([8, 12]).style(primary_action_btn_style)
        .on_press_maybe(enabled.then(|| Event::Apply(scope, ApplyEvent::Open(profile.id.clone(), profile.version)))).into()
}
pub(super) fn overlay<'a, Identity>(i18n: desktop_i18n::I18n, state: &'a State<Identity>, scope: ProjectProfileScope, base: Element<'a, Event>) -> Element<'a, Event> {
    let Some(form) = state.application.as_ref().filter(|f| f.scope == scope) else { return base; };
    let close = Event::Apply(scope, ApplyEvent::Close);
    let header = settings_modal_header(text(i18n.__("预览与一键应用")).size(20), settings_close_button(close.clone()));
    let mut body = column![text(&form.name)].spacing(10);
    if form.loading { body = body.push(text(if form.submitted { i18n.__("等待执行主机确认，尚未声明成功") } else { i18n.__("正在读取执行主机预览") })); }
    if let Some(error) = form.error { body = body.push(text(i18n.__(error))); }
    if let Some(preview) = &form.preview {
        body = body.push(text(if !preview.captured { i18n.__("此组未拍快照：只选择为当前组合，不写入四类配置。") }
            else { i18n.__("未拍的类别保持原状；已拍的空集合恢复为空，空提示词停止激活。") }));
        if preview.captured && preview.items.is_empty() { body = body.push(text(i18n.__("执行主机未列出差异。"))); }
        for item in &preview.items {
            let action = match item.action { PreviewAction::Select => i18n.__("选择"), PreviewAction::Enable => i18n.__("启用"), PreviewAction::Disable => i18n.__("停用"), PreviewAction::Missing => i18n.__("引用缺失") };
            body = body.push(text(format!("{} · {} · {}", category(i18n, item.category), item.reference_id, action)));
        }
    }
    if let Some(result) = &form.result {
        let label = match result.status { ProjectProfileApplyStatus::Applied => i18n.__("执行主机确认已应用"), ProjectProfileApplyStatus::Partial => i18n.__("部分应用失败"), ProjectProfileApplyStatus::Uncaptured => i18n.__("未拍快照，未应用配置"), ProjectProfileApplyStatus::Cancelled => i18n.__("执行主机确认已取消"), ProjectProfileApplyStatus::Failed => i18n.__("应用失败") };
        body = body.push(text(label)).push(text(if result.selected { i18n.__("已选择为当前组合") } else { i18n.__("未确认当前选择") }));
        for item in &result.items {
            body = body.push(text(format!("{} · {} · {:?}{}", category(i18n, item.category), item.reference_id.as_deref().unwrap_or(i18n.__("空集合")), item.status,
                item.reason_code.as_ref().map(|r| format!(" · {r}")).unwrap_or_default())));
        }
    }
    if let Some(operation) = &form.operation_id {
        body = body.push(text(i18n.format("操作：%s", &[format!("{}", operation).into()]).expect("valid workbench template")));
        if form.cancelling && form.result.is_none() { body = body.push(text(i18n.__("已请求取消，等待执行主机确认；已完成项不会自动撤销。"))); }
    }
    let actions = row![
        button(text(i18n.__("查询进度"))).padding([8, 14]).style(rounded_action_btn_style)
            .on_press_maybe((!form.loading && form.operation_id.is_some() && form.result.is_none()).then(|| Event::Apply(scope, ApplyEvent::Recover))),
        button(text(i18n.__("取消操作"))).padding([8, 14]).style(rounded_action_btn_style)
            .on_press_maybe((!form.loading && !form.cancelling && form.operation_id.is_some() && form.result.is_none()).then(|| Event::Apply(scope, ApplyEvent::Cancel))),
        button(text(i18n.__("关闭"))).padding([8, 14]).style(rounded_action_btn_style).on_press_maybe((!form.loading && (!form.submitted || form.result.is_some())).then_some(close.clone())),
        button(text(i18n.__("确认应用一次"))).padding([8, 14]).style(primary_action_btn_style)
            .on_press_maybe((!form.loading && !form.submitted && form.preview.is_some()).then(|| Event::Apply(scope, ApplyEvent::Confirm)))
    ].spacing(10);
    let card = settings_modal_layout(header, body, Some(actions.into())).width(620).height(520);
    settings_modal_overlay(Some(base), close, card)
}
fn category(i18n: desktop_i18n::I18n, category: ProjectProfileCategory) -> &'static str {
    match category { ProjectProfileCategory::Provider => i18n.__("供应商"), ProjectProfileCategory::Mcp => "MCP", ProjectProfileCategory::Skill => i18n.__("技能"), ProjectProfileCategory::Prompt => i18n.__("提示词") }
}
