//! Maintenance confirmation uses the shared settings modal and server-owned snapshots.
use crate::app::{Message, message::project_profiles::{Event, message}, state::project_profiles::{Action, State}};
use crate::app::components::system_settings_common::*;
use iced::{Element, widget::{button, column, row, text}};
use vw_gateway_client::vw_api_types::project_profiles::{ProjectProfile, ProjectProfileScope};

pub(super) fn actions(profile: &ProjectProfile, scope: ProjectProfileScope, enabled: bool) -> Element<'_, Message> {
    let mut actions = row![].spacing(8);
    for action in [Action::Rename(profile.id.clone(), profile.version), Action::Resnapshot(profile.id.clone(), profile.version), Action::Delete(profile.id.clone(), profile.version)] {
        actions = actions.push(button(text(action.title())).padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe(enabled.then(|| message(Event::Open(scope, action)))));
    }
    actions.into()
}

pub(super) fn overlay<'a>(state: &'a State, scope: ProjectProfileScope, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(form) = state.management.as_ref().filter(|f| f.scope == scope) else { return base; };
    let close = message(Event::Close(scope));
    let header = settings_modal_header(text(form.action.title()).size(20), settings_close_button(close.clone()));
    let description = match form.action {
        Action::Create => "从执行主机的当前配置创建组合，不应用配置。",
        Action::Rename(..) => "更新共享组合名称。",
        Action::Resnapshot(..) => "以执行主机当前配置覆盖此组合的当前应用组快照。",
        Action::Delete(..) => "删除共享组合及其标记，影响所有引用者；不会撤销已写入的配置。确认删除？",
        Action::Clear => "仅清除当前项目、执行主机和应用组的选择标记，不撤销配置。",
    };
    let mut body = column![text(description)].spacing(12);
    if form.action.needs_name() {
        body = body.push(settings_field_label("组合名称"))
            .push(settings_text_input("请输入组合名称", &form.name)
                .on_input_maybe((!form.busy).then_some(move |name| message(Event::Name(scope, name)))));
    } else if !form.name.is_empty() {
        body = body.push(text(&form.name));
    }
    let actions = row![
        button(text("取消")).padding([8, 14]).style(rounded_action_btn_style)
            .on_press_maybe((!form.busy).then_some(close.clone())),
        button(text(if form.busy { "处理中" } else { "确认" })).padding([8, 14]).style(primary_action_btn_style)
            .on_press_maybe((!form.busy && (!form.action.needs_name() || !form.name.trim().is_empty())).then(|| message(Event::Submit(scope))))
    ].spacing(10);
    let card = settings_modal_layout(header, body, Some(actions.into())).width(560).height(320);
    settings_modal_overlay(Some(base), close, card)
}
