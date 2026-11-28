use super::{SkillsSettingsTab, is_visible};
use crate::app::{App, Message, message::SettingsMessage};
use desktop_i18n::I18n;
use desktop_settings_skills::sync::{Event, Form};
use desktop_ui_controls::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_close_button, settings_error_banner, settings_group_card, settings_modal_header,
    settings_modal_layout, settings_modal_overlay, settings_muted_text_style,
    settings_page_heading, settings_tab_button,
};
use gateway_desktop_settings::{
    DesktopSkillSyncRequest, DesktopSkillSyncStatus, GatewayDesktopSettingsClient, SkillSyncScope,
    SkillSyncTarget,
};
use iced::{
    Alignment, Element, Length, Task,
    widget::{button, checkbox, column, container, row, text},
};

fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::SkillsSync(event))
}

pub(super) fn available() -> bool {
    !cfg!(target_arch = "wasm32") && crate::app::config::gateway_client_endpoint().is_loopback()
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if !is_visible(app)
        || app.skills_settings.active_tab != SkillsSettingsTab::Skills
        || !available()
    {
        return Task::none();
    }
    let state = &mut app.skills_settings.sync;
    let project = &app.navigation_tabs.project_path;
    match event {
        Event::Menu(id, point) => state.menu = Some((id, point)),
        Event::CloseMenu => state.menu = None,
        Event::Close => {
            if !state.form.as_ref().is_some_and(|form| form.busy) {
                state.form = None;
            }
        }
        Event::Open(id) => {
            state.menu = None;
            if state.form.as_ref().is_some_and(|form| form.busy) {
                return Task::none();
            }
            let Some(skill) = app
                .skills_settings
                .catalog
                .iter()
                .find(|skill| skill.id == id && skill.installed && !id.starts_with("plugin:"))
            else {
                return Task::none();
            };
            let skill_id = if skill.source == "bundled" && !id.starts_with("bundled:") {
                format!("bundled:{id}")
            } else {
                id
            };
            state.form = Some(Form {
                skill_id,
                expected_source_path: skill.source_path.clone(),
                title: skill.title.clone(),
                project_path: project.clone(),
                scope: SkillSyncScope::Global,
                targets: Vec::new(),
                busy: false,
                error: None,
                results: Vec::new(),
            });
        }
        event => {
            let Some(form) = state.form.as_mut().filter(|form| &form.project_path == project)
            else {
                return Task::none();
            };
            match event {
                Event::Completed(generation, result)
                    if generation == state.generation && form.busy =>
                {
                    form.busy = false;
                    let (kind, label) = match result {
                        Ok(items) => {
                            let failed = items
                                .iter()
                                .any(|item| item.status == DesktopSkillSyncStatus::Failed);
                            form.results = items;
                            if failed {
                                (crate::app::state::ToastKind::Error, "部分同步目标安装失败")
                            } else {
                                (crate::app::state::ToastKind::Success, "技能同步完成")
                            }
                        }
                        Err(error) => {
                            form.error = Some(error);
                            (crate::app::state::ToastKind::Error, "技能同步失败")
                        }
                    };
                    return app.show_message_toast(
                        kind,
                        desktop_i18n::format::Message::new(label, vec![]),
                    );
                }
                Event::Scope(scope)
                    if !form.busy && (scope == SkillSyncScope::Global || project.is_some()) =>
                {
                    form.scope = scope;
                    form.targets.retain(|target| target.supports_scope(scope));
                    form.results.clear();
                    form.error = None;
                }
                Event::Select(target, selected)
                    if !form.busy
                        && SkillSyncTarget::THIRD_PARTY_AGENTS.contains(&target)
                        && target.supports_scope(form.scope) =>
                {
                    form.targets.retain(|other| *other != target);
                    if selected {
                        form.targets.push(target);
                    }
                }
                Event::Submit if !form.busy && !form.targets.is_empty() => {
                    let request = DesktopSkillSyncRequest {
                        project_path: form.project_path.clone(),
                        skill_id: form.skill_id.clone(),
                        expected_source_path: form.expected_source_path.clone(),
                        scope: form.scope,
                        targets: form.targets.clone(),
                    };
                    form.busy = true;
                    form.error = None;
                    form.results.clear();
                    state.generation = state.generation.wrapping_add(1);
                    let generation = state.generation;
                    let client = crate::app::config::gateway_client();
                    return Task::perform(
                        async move {
                            let client = client?;
                            GatewayDesktopSettingsClient::new(client.transport().clone())
                                .sync_skill(&request)
                                .await
                        },
                        move |result| message(Event::Completed(generation, result)),
                    );
                }
                _ => {}
            }
        }
    }
    Task::none()
}

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(form) = app.skills_settings.sync.form.as_ref().filter(|form| {
        form.project_path == app.navigation_tabs.project_path
            && app.skills_settings.active_tab == SkillsSettingsTab::Skills
            && available()
    }) else {
        return base;
    };
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let close = message(Event::Close);
    let scope_button = |label, scope| {
        settings_tab_button(
            i18n.__(label),
            form.scope == scope,
            (!form.busy
                && scope != form.scope
                && (scope == SkillSyncScope::Global || form.project_path.is_some()))
            .then_some(message(Event::Scope(scope))),
        )
    };
    let scope = row![
        text(i18n.__("安装范围")).size(13),
        scope_button("全局", SkillSyncScope::Global),
        scope_button("项目", SkillSyncScope::Project),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let mut choices = column![].spacing(10);
    for pair in SkillSyncTarget::THIRD_PARTY_AGENTS.chunks(2) {
        let mut line = row![].spacing(12);
        for &target in pair {
            let enabled = !form.busy && target.supports_scope(form.scope);
            let label = if target == SkillSyncTarget::ClaudeDesktop {
                i18n.__("请在 Claude Desktop 中上传技能 ZIP")
            } else if !target.supports_scope(form.scope) {
                i18n.__("支持全局安装")
            } else if let Some(item) = form.results.iter().find(|item| item.target == target) {
                i18n.__(match item.status {
                    DesktopSkillSyncStatus::Installed => "已安装",
                    DesktopSkillSyncStatus::AlreadyInstalled => "已有同名技能",
                    DesktopSkillSyncStatus::Failed => "安装失败",
                })
            } else {
                ""
            };
            let choice = checkbox(form.targets.contains(&target))
                .label(i18n.__(target.name()))
                .text_size(13)
                .style(settings_checkbox_style)
                .on_toggle_maybe(
                    enabled.then_some(move |selected| message(Event::Select(target, selected))),
                );
            let mut item = column![choice].spacing(4).width(Length::FillPortion(1));
            if !label.is_empty() {
                item = item.push(text(label).size(12).style(settings_muted_text_style));
            }
            line = line.push(item);
        }
        choices = choices.push(line);
    }
    let mut body = column![scope, settings_group_card(container(choices).padding([12, 0]))]
        .spacing(12)
        .width(Length::Fill);
    if let Some(error) = &form.error {
        body = body.push(settings_error_banner(i18n.__(error).to_owned()));
    }
    let actions = row![
        button(text(i18n.__("关闭")).size(13))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!form.busy).then_some(close.clone())),
        button(text(i18n.__(if form.busy { "正在安装技能…" } else { "安装技能" })).size(13))
            .padding([8, 12])
            .style(primary_action_btn_style)
            .on_press_maybe(
                (!form.busy && !form.targets.is_empty()).then_some(message(Event::Submit))
            ),
    ]
    .spacing(8)
    .width(Length::Fill)
    .align_y(Alignment::Center);
    let header = settings_modal_header(
        settings_page_heading(i18n.__("同步到第三方"), form.title.as_str()),
        settings_close_button(close.clone()),
    );
    let card = settings_modal_layout(header, body, Some(actions.into())).width(590).height(540);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
