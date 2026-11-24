use super::super::{
    system_settings_apps, system_settings_common::rounded_action_btn_style, third_party_settings,
};
use crate::app::{
    App, Message,
    message::settings::{AppsCatalogMessage, prompts},
};
use desktop_assets::assets::Icon;
use desktop_settings_apps::system_settings_apps::list::brand_icon;
use desktop_settings_skills::prompts::{
    Event as PromptEvent,
    list::{Menu, menu_action_style, prompt_menu},
};
use desktop_ui_controls::{
    settings_navigation::ThirdPartyPage,
    system_settings_common::{
        icon_svg, icons::icon_btn_svg_style, settings_divider, settings_muted_text_style,
    },
};
use iced::{
    Alignment, Element, Length, Task,
    widget::{button, column, container, row, text},
};
use vw_api_app_identity::AppId;

pub(crate) type State =
    desktop_settings_skills::prompts::list::State<crate::app::state::AgentAppsContext>;

pub(crate) fn switch_blocked(state: &State) -> bool {
    state.mutating
        || state.editor.is_some()
        || state.shared_import.is_some()
        || state.transfer.entry.is_some()
        || state.templates.open
        || state.pi_files.open
        || state.sync.busy
        || (state.sync.submitted && !state.sync.terminal())
}

fn supports_prompts(id: AppId) -> bool {
    desktop_settings_skills::prompts::transfer::TARGETS.contains(&id)
}

fn available(app: &App, id: AppId) -> bool {
    let state = &app.apps_catalog;
    // Catalog observations already come from the authenticated Gateway session.
    !state.loading
        && state.error.is_none()
        && state.context.is_some()
        && (supports_prompts(id) || matches!(id, AppId::ClaudeDesktop | AppId::OpenClaw))
        && state.rows.iter().any(|row| row.entry.id == id && row.observation.is_ok())
}

pub(crate) fn select(app: &mut App, id: AppId) -> Task<Message> {
    if switch_blocked(&app.apps_catalog.prompts) || !available(app, id) {
        return Task::none();
    }
    if app.apps_catalog.third_party_page != ThirdPartyPage::Prompts {
        app.apps_catalog.expanded = Some(id);
        return third_party_settings::navigate(app, ThirdPartyPage::Prompts);
    }

    app.apps_catalog.expanded = Some(id);
    if supports_prompts(id) {
        prompts::update(app, prompts::Event::Open(id))
    } else {
        app.apps_catalog.prompts.invalidate();
        Task::none()
    }
}

pub(crate) fn open_selected(app: &mut App) -> Task<Message> {
    if app.apps_catalog.loading || switch_blocked(&app.apps_catalog.prompts) {
        return Task::none();
    }
    let selected = app
        .apps_catalog
        .expanded
        .filter(|id| available(app, *id))
        .or_else(|| {
            app.apps_catalog
                .rows
                .iter()
                .map(|row| row.entry.id)
                .find(|id| supports_prompts(*id) && available(app, *id))
        })
        .or_else(|| {
            app.apps_catalog.rows.iter().map(|row| row.entry.id).find(|id| available(app, *id))
        });
    app.apps_catalog.expanded = selected;
    match selected {
        Some(id) if supports_prompts(id) => prompts::update(app, prompts::Event::Open(id)),
        _ => Task::none(),
    }
}

pub(crate) fn page(app: &App) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    let state = &app.apps_catalog;
    let selected = state.expanded.filter(|id| available(app, *id));
    let Some(item) = state.rows.iter().find(|item| Some(item.entry.id) == selected) else {
        let (status, retry) = if state.loading {
            (i18n.__("正在读取应用目录…"), false)
        } else if state.error.is_some() {
            (i18n.__("应用目录读取失败"), true)
        } else {
            (i18n.__("当前主机没有可管理提示词的应用"), true)
        };
        let mut empty = column![text(status).size(16)].spacing(10).align_x(Alignment::Center);
        if retry {
            empty = empty.push(
                button(text(i18n.__("刷新应用目录")))
                    .padding([7, 12])
                    .style(rounded_action_btn_style)
                    .on_press(system_settings_apps::message(AppsCatalogMessage::Refresh)),
            );
        }
        return frame(i18n, container(empty).center(Length::Fill).into());
    };

    let id = item.entry.id;
    let application = item.entry.name.as_str();
    let scope_current = state.prompts.scope.as_ref().is_some_and(|scope| {
        scope.app_id == id
            && scope.target_host_id == state.host
            && scope.project_id.as_ref()
                == vw_api_transport::local_settings_project(
                    app.workspace_state.project_id.as_deref(),
                )
    });
    if scope_current && let Some(draft) = &state.prompts.editor {
        return desktop_settings_skills::prompts::editor::page(
            i18n,
            draft,
            application,
            state.prompts.preview.as_ref().and_then(|preview| preview.display_path.as_deref()),
            super::ui_message,
        );
    }

    let count = scope_current
        .then_some(state.prompts.catalog.as_ref())
        .flatten()
        .map(|catalog| catalog.entries.len().to_string());
    let open = state.prompts.menu == Some(Menu::Apps);
    let mut items = column![].spacing(2);
    for candidate in state
        .rows
        .iter()
        .filter(|row| supports_prompts(row.entry.id) && available(app, row.entry.id))
    {
        let candidate_id = candidate.entry.id;
        let is_selected = candidate_id == id;
        let marker: Element<'_, Message> = if is_selected {
            icon_svg(Icon::Check, 14.0).style(icon_btn_svg_style).into()
        } else {
            iced::widget::Space::new().into()
        };
        let item = row![
            brand_icon(candidate_id, 17.0),
            text(&candidate.entry.name).size(14).width(Length::Fill),
            text(if is_selected { count.clone().unwrap_or_default() } else { String::new() })
                .size(12)
                .style(settings_muted_text_style),
            container(marker).width(15),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        items = items.push(
            button(container(item).center_y(Length::Fill))
                .padding([0, 9])
                .height(32)
                .width(Length::Fill)
                .style(move |theme, status| {
                    let mut style = menu_action_style(theme, status);
                    if is_selected && status == button::Status::Active {
                        style.background =
                            Some(theme.extended_palette().background.weak.color.into());
                    }
                    style
                })
                .on_press(if is_selected {
                    super::ui_message(PromptEvent::Menu(None))
                } else {
                    system_settings_apps::message(AppsCatalogMessage::SelectPromptApp(candidate_id))
                }),
        );
    }
    let related = [
        (AppId::ClaudeDesktop, i18n.__("Claude Desktop 使用 Claude Code 的提示词")),
        (AppId::OpenClaw, i18n.__("在工作区管理 OpenClaw 提示词")),
    ];
    if related.iter().any(|(id, _)| available(app, *id)) {
        items = items.push(settings_divider());
    }
    for (related_id, label) in related.into_iter().filter(|(id, _)| available(app, *id)) {
        items = items.push(
            button(
                row![
                    text(label).size(12).style(settings_muted_text_style).width(Length::Fill),
                    icon_svg(Icon::ChevronRight, 11.0).style(icon_btn_svg_style),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([6, 9])
            .width(Length::Fill)
            .style(menu_action_style)
            .on_press(if related_id == id {
                super::ui_message(PromptEvent::Menu(None))
            } else {
                system_settings_apps::message(AppsCatalogMessage::SelectPromptApp(related_id))
            }),
        );
    }
    let mut label = row![brand_icon(id, 17.0), text(application).size(14)]
        .spacing(8)
        .align_y(Alignment::Center);
    if let Some(count) = &count {
        label = label.push(text(count.clone()).size(12).style(settings_muted_text_style));
    }
    let chevron: Element<'_, Message> =
        icon_svg(Icon::ChevronDown, 12.0).style(icon_btn_svg_style).into();
    let toggle = button(container(label.push(chevron)).center_y(Length::Fill))
        .height(desktop_ui_controls::system_settings_common::SETTINGS_CONTROL_HEIGHT)
        .padding([0, 10])
        .style(move |theme, status| {
            let mut style = rounded_action_btn_style(theme, status);
            if open && status == button::Status::Active {
                style.background = Some(theme.extended_palette().background.weak.color.into());
            }
            style
        })
        .on_press_maybe(
            (!switch_blocked(&state.prompts))
                .then(|| super::ui_message(PromptEvent::Menu((!open).then_some(Menu::Apps)))),
        );
    let selector = prompt_menu(
        toggle,
        items.into(),
        &Menu::Apps,
        open,
        super::ui_message(PromptEvent::Menu(None)),
    );

    match id {
        AppId::ClaudeDesktop => frame(
            i18n,
            container(
                column![
                    selector,
                    text(i18n.__("Claude Desktop 使用 Claude Code 的提示词。")).size(14),
                    button(text(i18n.__("打开 Claude Code 提示词")))
                        .padding([7, 12])
                        .style(rounded_action_btn_style)
                        .on_press_maybe(available(app, AppId::Claude).then(|| {
                            system_settings_apps::message(AppsCatalogMessage::SelectPromptApp(
                                AppId::Claude,
                            ))
                        })),
                ]
                .spacing(12),
            )
            .padding([12, 18])
            .into(),
        ),
        AppId::OpenClaw => frame(
            i18n,
            container(
                column![
                    selector,
                    text(i18n.__("在工作区管理 OpenClaw 提示词。")).size(14),
                    super::super::workspace_files::entry(),
                ]
                .spacing(12),
            )
            .padding([12, 18])
            .into(),
        ),
        _ if scope_current => {
            let local = state.host == local_app_auth::HOST_ID
                && crate::app::config::gateway_client()
                    .is_ok_and(|client| client.endpoint().is_loopback());
            let base = desktop_settings_skills::prompts::list::view(
                i18n,
                &state.prompts,
                id,
                application,
                selector,
                local,
                prompt_created_date,
                super::ui_message,
            );
            desktop_settings_skills::prompts::transfer::overlay(
                i18n,
                &state.prompts.transfer,
                id,
                state.prompts.mutating || state.prompts.loading,
                base,
                super::ui_message,
            )
        }
        _ => frame(
            i18n,
            container(column![selector, text(i18n.__("正在读取提示词…")).size(14)].spacing(12))
                .padding([12, 18])
                .into(),
        ),
    }
}

fn frame(i18n: desktop_i18n::I18n, body: Element<'_, Message>) -> Element<'_, Message> {
    column![
        container(
            row![
                icon_svg(Icon::FileText, 19.0).style(icon_btn_svg_style),
                text(i18n.__("提示词")).size(21),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .padding([12, 18])
        .width(Length::Fill),
        settings_divider(),
        body,
    ]
    .height(Length::Fill)
    .into()
}

fn prompt_created_date(timestamp_ms: i64) -> Option<String> {
    let utc = chrono::DateTime::from_timestamp_millis(timestamp_ms)?;
    Some(utc.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
}
