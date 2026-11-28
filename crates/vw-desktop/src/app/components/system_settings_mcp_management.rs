//! Desktop composition of the managed MCP matrix and revision-bound actions.
use crate::app::{
    App, Message,
    message::{
        SettingsMessage,
        settings::{self, AppsCatalogMessage, McpMessage},
    },
};
use desktop_assets::assets::Icon;
use desktop_settings_mcp::{
    management_bindings::{BindingsMessage, matrix},
    management_context::Context,
    management_probe::actions,
    management_state::ViewEvent as Action,
};
use desktop_ui_controls::{overlays::BelowOverlay, system_settings_common::*};
use iced::{
    Alignment, Element, Length, Task, alignment,
    widget::{Space, column, container, row, stack, text},
};

fn message(scope: Context, action: Action) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Management(scope, action)))
}

pub(crate) fn refresh(app: &mut App) -> Task<Message> {
    if !super::is_visible(app) {
        return Task::none();
    }
    app.mcp_settings.bindings.matrix.column_menu = None;
    settings::mcp_management_list::update(
        app,
        settings::mcp_management_list::CatalogMessage::Refresh,
    )
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    let Some(scope) = settings::mcp_management_list::context(app) else {
        return text(i18n.__("连接网关后刷新 MCP 目录")).style(settings_muted_text_style).into();
    };
    let targets = settings::mcp_management_import::targets(app);
    let shown = super::super::system_settings_apps::preferences::visible_rows(&app.apps_catalog)
        .iter()
        .map(|r| r.entry.id)
        .collect::<Vec<_>>();
    let diagnostics = &app.mcp_settings.diagnostics;
    let current = crate::app::message::agent_apps_diagnostics::context(app);
    let show_diagnostics = current.is_some()
        && current.as_ref() == diagnostics.context.as_ref()
        && (diagnostics.active() || !diagnostics.targets.is_empty() || diagnostics.error.is_some());
    let catalog = settings::mcp_management_list::visible(app).and_then(|r| r.as_ref().ok());
    let ids = catalog
        .map(|c| {
            matrix::visible_records(c, &app.mcp_settings.bindings.matrix.query)
                .into_iter()
                .map(|r| r.record.id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let menu = &app.mcp_settings.actions;
    let menu_current =
        menu.scope.as_ref() == Some(&scope) && catalog.is_some_and(|c| c.records == menu.records);
    let more = icon_btn(
        Icon::DotsThreeVertical,
        i18n.__("更多 MCP 操作"),
        Some(message(
            scope.clone(),
            Action::Menu(actions::MenuMessage::Action(actions::Message::More)),
        )),
    );
    let more = if menu_current && menu.more {
        let menu_scope = scope.clone();
        BelowOverlay::new(
            more,
            container(
                actions::menu(i18n, menu)
                    .map(move |event| message(menu_scope.clone(), Action::Menu(event))),
            )
            .padding(7)
            .width(288)
            .style(settings_modal_card_style),
        )
        .show(true)
        .gap(6.0)
        .dismiss_on_escape()
        .on_close(message(
            scope.clone(),
            Action::Menu(actions::MenuMessage::Action(actions::Message::Close)),
        ))
        .into()
    } else {
        more
    };
    let toolbar_actions = row![
        desktop_settings_mcp::toolbar_button(
            i18n.__("检查连通性"),
            (!ids.is_empty() && !diagnostics.active() && current.is_some()).then(|| {
                crate::app::message::agent_apps_diagnostics::message(actions::ProbeMessage::Start(
                    ids,
                ))
            }),
        ),
        desktop_settings_mcp::toolbar_button(
            i18n.__("从原生应用导入"),
            Some(settings::mcp_management_import::message(
                settings::mcp_management_import::ImportMessage::Open,
            )),
        ),
        icon_btn(
            Icon::ArrowClockwise,
            i18n.__("刷新目录"),
            Some(Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::Refresh)))
        ),
        more,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap()
    .align_x(alignment::Horizontal::Right);
    let toolbar = row![
        text(i18n.__("共享 MCP 目录")).size(16),
        container(toolbar_actions).align_right(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .width(Length::Fill);
    let matrix = matrix::panel(
        i18n,
        settings::mcp_management_list::visible(app),
        &app.mcp_settings.bindings.matrix,
        &targets,
        &shown,
        &scope.host,
        settings::mcp_management_list::loading(app),
        app.mcp_settings.mutating() || app.apps_catalog.loading,
        menu_current.then_some(menu),
    )
    .map(move |event| message(scope.clone(), Action::Matrix(event)));
    let mut body = column![matrix].spacing(12).width(Length::Fill);
    if show_diagnostics {
        body = body.push(super::super::agent_apps_diagnostics::mcp_panel::view(app));
    }
    body = body.push(super::resync::panel(app)).push(super::tools::panel(app));
    column![settings_group_header(toolbar), body].spacing(12).width(Length::Fill).into()
}
pub(crate) fn update(app: &mut App, scope: Context, action: Action) -> Task<Message> {
    if !super::is_visible(app)
        || settings::mcp_management_list::context(app).as_ref() != Some(&scope)
    {
        return Task::none();
    }
    let records = settings::mcp_management_list::visible(app)
        .and_then(|r| r.as_ref().ok())
        .map(|c| c.records.clone())
        .unwrap_or_default();
    match action {
        Action::Matrix(matrix::ViewMessage::ColumnMenu(target)) => {
            app.mcp_settings.actions.row = None;
            app.mcp_settings.actions.more = false;
            let menu = &mut app.mcp_settings.bindings.matrix.column_menu;
            *menu = if *menu == target { None } else { target };
            Task::none()
        }
        Action::Matrix(matrix::ViewMessage::MenuAction(menu)) => {
            update(app, scope, Action::Menu(menu))
        }
        Action::Matrix(matrix::ViewMessage::Binding(action)) => {
            app.mcp_settings.bindings.matrix.column_menu = None;
            if matches!(&action, matrix::Action::Row { .. }) {
                app.mcp_settings.actions.row = None;
            }
            settings::mcp_management_bindings::update(app, BindingsMessage::Matrix(action))
        }
        Action::Matrix(matrix::ViewMessage::Edit(reference)) => {
            let Some(record) = records.into_iter().find(|r| r.record == reference) else {
                return Task::none();
            };
            settings::mcp_management_form::update(
                app,
                settings::mcp_management_form::FormMessage::Open(Some(record)),
            )
        }
        Action::Matrix(matrix::ViewMessage::Sync(reference)) => {
            let Some(record) = records.into_iter().find(|r| r.record == reference) else {
                return Task::none();
            };
            settings::mcp_management_form::update(
                app,
                settings::mcp_management_form::FormMessage::Sync(record),
            )
        }
        Action::Matrix(matrix::ViewMessage::Menu(reference)) => {
            let Some(record) = records.into_iter().find(|r| r.record == reference) else {
                return Task::none();
            };
            update(
                app,
                scope,
                Action::Menu(actions::MenuMessage::Action(actions::Message::Row(record))),
            )
        }
        Action::Menu(actions::MenuMessage::Action(action)) => {
            app.mcp_settings.bindings.matrix.column_menu = None;
            if matches!(&action, actions::Message::CopyAll | actions::Message::CopyRow(_)) {
                app.mcp_settings.actions.more = false;
                app.mcp_settings.actions.row = None;
            }
            let mut input = actions::Update {
                state: &mut app.mcp_settings.actions,
                scope: &scope,
                catalog: &records,
                authorized: !app.apps_catalog.loading,
                client: crate::app::config::gateway_client().map_err(|e| e.to_string()),
                token: app.apps_catalog.token.clone(),
                effects: Vec::new(),
            };
            let task = actions::update(
                &mut input,
                actions::ScopedMessage { scope: scope.clone(), action },
            )
            .map(|event| {
                message(event.scope, Action::Menu(actions::MenuMessage::Action(event.action)))
            });
            let effects = std::mem::take(&mut input.effects);
            settings::mcp_management_tools::apply_effects(app, task, effects)
        }
        Action::Menu(menu) => {
            app.mcp_settings.actions.more = false;
            app.mcp_settings.actions.row = None;
            match menu {
                actions::MenuMessage::Rewrite => settings::mcp_management_resync::update(
                    app,
                    settings::mcp_management_resync::ResyncMessage::Open,
                ),
                actions::MenuMessage::Tools(record) => settings::mcp_management_tools::update(
                    app,
                    settings::mcp_management_tools::ToolsMessage::Open(record),
                ),
                actions::MenuMessage::Documentation(url) => Task::done(Message::View(
                    crate::app::message::ViewMessage::OpenUrlExternal(url),
                )),
                actions::MenuMessage::Action(_) => Task::none(),
            }
        }
    }
}
pub(crate) fn overlays<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(scope) = settings::mcp_management_list::context(app) else {
        return base;
    };
    let state = &app.mcp_settings.actions;
    let current = state.scope.as_ref() == Some(&scope)
        && settings::mcp_management_list::visible(app)
            .and_then(|c| c.as_ref().ok())
            .is_some_and(|c| c.records == state.records);
    if current && (state.delete.is_some() || state.export.is_some()) {
        let overlay = actions::overlay(
            desktop_i18n::I18n::new(app.general_settings.desktop_locale),
            state,
            Space::new().width(Length::Fill).height(Length::Fill).into(),
        )
        .map(move |event| {
            message(scope.clone(), Action::Menu(actions::MenuMessage::Action(event)))
        });
        stack![settings_inactive_view(base), overlay].into()
    } else {
        base
    }
}
