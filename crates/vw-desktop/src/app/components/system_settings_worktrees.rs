//! Worktree policy summary and the shared settings modal editor.
use super::system_settings_common::*;
use crate::app::i18n::I18n;
use crate::app::message::SettingsMessage;
use crate::app::message::settings::worktrees::WorktreeSettingsMessage as Event;
use crate::app::state::{WorktreeSettingsState, WorktreesTab};
use crate::app::{App, Message};
use iced::widget::{Space, button, checkbox, column, row, text};
use iced::{Alignment, Element, Length};

mod environment;
mod managed;

fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::Worktrees(event))
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    page(&app.worktree_settings, I18n::new(app.desktop_locale))
}

pub(crate) fn page(state: &WorktreeSettingsState, i18n: I18n) -> Element<'_, Message> {
    let tabs = settings_tab_row([
        settings_tab_button(
            i18n.__("常规"),
            state.active_tab == WorktreesTab::General,
            Some(message(Event::TabSelected(WorktreesTab::General))),
        )
        .into(),
        settings_tab_button(
            i18n.__("环境"),
            state.active_tab == WorktreesTab::Environment,
            Some(message(Event::TabSelected(WorktreesTab::Environment))),
        )
        .into(),
    ]);
    let mut content = column![tabs].spacing(16);
    if state.active_tab == WorktreesTab::Environment {
        return content.push(environment::view(&state.environment, i18n)).into();
    }
    let actions = row![
        button(text(i18n.__("刷新")))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.saving).then(|| message(Event::Refresh))),
        button(text(i18n.__("编辑")))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe((state.saved.is_some() && !state.saving).then(|| message(Event::Edit))),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    content = content.push(actions);
    if state.loading {
        content = content.push(text(i18n.__("正在加载…")));
    }
    if let Some(error) = &state.error {
        content = content.push(settings_error_banner(i18n.__(error).to_owned()));
    }
    if let Some(policy) = &state.saved {
        let enabled = |value| i18n.__(if value { "开启" } else { "已关闭" });
        let field = |label: &'static str, value: String| -> Element<'_, Message> {
            column![
                text(i18n.__(label)).size(14),
                text(value).size(13).style(settings_muted_text_style)
            ]
            .spacing(6)
            .padding([10, 0])
            .into()
        };
        let root = if policy.root.trim().is_empty() { &state.default_root } else { &policy.root };
        content = content.push(settings_group_card(column![
            field("工作树根目录", root.clone()),
            settings_divider(),
            field("创建前获取更新", enabled(policy.fetch_before_create).into()),
            settings_divider(),
            field("自动清理", enabled(policy.auto_cleanup_enabled).into()),
            settings_divider(),
            field("保留数量", policy.keep_count.to_string()),
            settings_divider(),
            field("删除前询问", enabled(policy.ask_before_delete).into()),
        ]));
        if policy.ask_before_delete {
            content = content.push(
                text(i18n.__("自动清理会提出建议，等待确认后才会删除。"))
                    .size(13)
                    .style(settings_muted_text_style),
            );
        }
    }
    content.push(managed::view(&state.managed, i18n)).into()
}

pub(crate) fn view_overlays<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let base = editor(&app.worktree_settings, I18n::new(app.desktop_locale), base);
    environment::editor(&app.worktree_settings.environment, I18n::new(app.desktop_locale), base)
}

pub(crate) fn editor<'a>(
    state: &'a WorktreeSettingsState,
    i18n: I18n,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(draft) = &state.draft else {
        return base;
    };
    let busy = state.saving;
    let close = message(Event::Cancel);
    let toggle = |label: &'static str, checked, change: fn(bool) -> Event| {
        checkbox(checked)
            .label(i18n.__(label))
            .style(settings_checkbox_style)
            .on_toggle_maybe((!busy).then_some(move |value| message(change(value))))
    };
    let count_enabled = !busy && draft.policy.auto_cleanup_enabled;
    let mut body = column![
        text(i18n.__("工作树根目录")),
        settings_text_input(&state.default_root, &draft.policy.root)
            .on_input_maybe((!busy).then_some(|value| message(Event::RootChanged(value)))),
        text(i18n.__("留空使用网关默认目录；更改仅影响以后新建的工作树。"))
            .size(13)
            .style(settings_muted_text_style),
        text(state.default_root.clone()).size(13).style(settings_muted_text_style),
        toggle("创建前获取更新", draft.policy.fetch_before_create, Event::FetchChanged),
        toggle("自动清理", draft.policy.auto_cleanup_enabled, Event::CleanupChanged),
        text(i18n.__("保留数量")),
        settings_text_input("15", &draft.keep_count)
            .on_input_maybe(count_enabled.then_some(|value| message(Event::CountChanged(value)))),
        toggle("删除前询问", draft.policy.ask_before_delete, Event::AskChanged),
    ]
    .spacing(14);
    if draft.policy.ask_before_delete {
        body = body.push(
            text(i18n.__("自动清理会提出建议，等待确认后才会删除。"))
                .size(13)
                .style(settings_muted_text_style),
        );
    }
    let validation = draft.validated().err();
    let mut header = column![settings_modal_header(
        text(i18n.__("工作树设置")).size(20),
        settings_close_button(close.clone())
    )]
    .spacing(10);
    if let Some(error) = validation.or(state.error.as_deref()) {
        header = header.push(settings_error_banner(i18n.__(error).to_owned()));
    }
    let footer = row![
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!busy).then_some(close.clone())),
        button(text(i18n.__(if busy { "正在保存…" } else { "保存" })))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe((!busy && validation.is_none()).then(|| message(Event::Save))),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    settings_modal_overlay(
        Some(base),
        close,
        settings_modal_layout(header, body, Some(footer.into())).width(640).height(580),
    )
}

#[cfg(test)]
#[path = "system_settings_worktrees/managed_tests.rs"]
mod managed_tests;

#[cfg(test)]
#[path = "system_settings_worktrees/environment_tests.rs"]
mod environment_tests;
