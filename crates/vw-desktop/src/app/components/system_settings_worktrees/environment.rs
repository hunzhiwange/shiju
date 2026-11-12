//! Project selection and the shared environment settings modal.
use super::message;
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::worktrees::{
    WorktreeSettingsMessage,
    environment::{EnvironmentMessage as Event, can_add},
};
use crate::app::{Message, i18n::I18n, state::WorktreeEnvironmentState};
use iced::widget::{Space, button, column, row, text};
use iced::{Alignment, Element, Length};
fn event(value: Event) -> Message {
    message(WorktreeSettingsMessage::Environment(value))
}
pub(crate) fn view<'a>(state: &'a WorktreeEnvironmentState, i18n: I18n) -> Element<'a, Message> {
    let local = can_add(&crate::app::gateway_client_endpoint());
    let mut body = column![
        row![
            text(i18n.__("选择项目")).size(18).width(Length::Fill),
            button(text(i18n.__("刷新")))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press(event(Event::Refresh)),
            button(text(i18n.__("添加项目")))
                .padding([8, 14])
                .style(primary_action_btn_style)
                .on_press_maybe(
                    (local && !state.adding && !state.projects_loading).then(|| event(Event::Add))
                )
        ]
        .spacing(10)
        .align_y(Alignment::Center)
    ]
    .spacing(12)
    .padding([0, 7]);
    if !local {
        body = body.push(
            text(i18n.__("远程网关仅支持选择已注册项目。请在网关所在主机添加项目。"))
                .size(13)
                .style(settings_muted_text_style),
        );
    }
    if state.projects_loading || state.adding {
        body = body.push(text(i18n.__("正在加载…")));
    }
    if let Some(error) = &state.list_error {
        body = body.push(settings_error_banner(error.clone()));
    }
    if state.projects.is_empty() && !state.projects_loading {
        body = body.push(text(i18n.__("暂无项目，请添加项目。")));
    }
    for project in &state.projects {
        body = body.push(settings_group_card(
            row![
                column![
                    text(&project.name).size(14),
                    text(&project.directory).size(13).style(settings_muted_text_style)
                ]
                .spacing(6)
                .width(Length::Fill),
                button(text("+").size(20))
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!state.saving).then(|| event(Event::Edit(project.clone()))))
            ]
            .padding([12, 0])
            .spacing(12)
            .align_y(Alignment::Center),
        ));
    }
    body.into()
}
pub(crate) fn editor<'a>(
    state: &'a WorktreeEnvironmentState,
    i18n: I18n,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(project) = &state.selected else {
        return base;
    };
    let close = event(Event::Cancel);
    let busy = state.loading || state.saving;
    let mut header = column![settings_modal_header(
        text(i18n.__("项目环境")).size(20),
        settings_close_button(close.clone())
    )]
    .spacing(10);
    if let Some(error) = &state.error {
        header = header.push(settings_error_banner(error.clone()));
    }
    let mut input = settings_text_editor(&state.editor).height(240);
    if !busy && state.loaded {
        input = input.on_action(|action| event(Event::Action(action)));
    }
    let mut body = column![
        text(&project.name).size(17),
        text(&project.directory).size(13).style(settings_muted_text_style),
        text(i18n.__("初始化命令")),
        text(i18n.__("仅在新建工作树时运行；保存不会执行命令。留空可清除。"))
            .size(13)
            .style(settings_muted_text_style),
        input
    ]
    .spacing(12);
    if state.loading {
        body = body.push(text(i18n.__("正在加载…")));
    }
    if !state.loaded && !state.loading {
        body = body.push(
            button(text(i18n.__("重试")))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press(event(Event::Edit(project.clone()))),
        );
    }
    let footer = row![
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.saving).then_some(close.clone())),
        button(text(i18n.__(if state.saving { "正在保存…" } else { "保存" })))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe((!busy && state.loaded).then(|| event(Event::Save)))
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    settings_modal_overlay(
        Some(base),
        close,
        settings_modal_layout(header, body, Some(footer.into())).width(680).height(560),
    )
}
