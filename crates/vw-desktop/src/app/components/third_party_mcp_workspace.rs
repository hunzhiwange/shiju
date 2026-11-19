//! Desktop composition of the managed MCP matrix and revision-bound actions.
use crate::app::{
    App, Message,
    message::{
        SettingsMessage,
        settings::{self, AppsCatalogMessage},
    },
};
use desktop_settings_mcp::{
    management_bindings::{BindingsMessage, matrix},
    management_probe::actions,
    third_party::Scope,
};
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Element, Length, Task,
    widget::{Space, column, container, row, scrollable, stack, text},
};

#[derive(Debug, Clone)]
pub(crate) struct Event {
    scope: Scope,
    action: Action,
}
#[derive(Debug, Clone)]
enum Action {
    Matrix(matrix::ViewMessage),
    Menu(actions::MenuMessage),
}
fn message(scope: Scope, action: Action) -> Message {
    Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::McpWorkspace(Event {
        scope,
        action,
    })))
}
pub(crate) fn refresh(app: &mut App) -> Task<Message> {
    if !super::is_visible(app)
        || app.apps_catalog.third_party_page
            != desktop_ui_controls::settings_navigation::ThirdPartyPage::Mcp
    {
        return Task::none();
    }
    app.apps_catalog.third_party_mcp_scope = super::mcp_scope(app);
    desktop_settings_mcp::management_probe::actions::invalidate(
        &mut app.mcp_settings.actions,
        app.apps_catalog.third_party_mcp_scope.as_ref(),
        &[],
    );
    settings::mcp_management_list::update(
        app,
        settings::mcp_management_list::CatalogMessage::Refresh,
    )
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let Some(scope) = app
        .apps_catalog
        .third_party_mcp_scope
        .as_ref()
        .filter(|s| Some(*s) == super::mcp_scope(app).as_ref())
    else {
        return column![
            text("连接网关后刷新 MCP 目录"),
            crate::app::components::system_settings_common::icon_btn(
                desktop_assets::assets::Icon::ArrowClockwise,
                "刷新目录",
                Some(Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::Refresh)))
            )
        ]
        .into();
    };
    let targets = settings::mcp_management_import::targets(app);
    let shown = super::super::system_settings_apps::preferences::visible_rows(&app.apps_catalog)
        .iter()
        .map(|r| r.entry.id)
        .collect::<Vec<_>>();
    let mut brands = row![].spacing(10);
    for id in &shown {
        brands =
            brands.push(desktop_settings_apps::system_settings_apps::list::brand_icon(*id, 18.0));
    }
    let toolbar = row![
        toolbar_button(
            "添加 MCP",
            Some(settings::mcp_management_form::message(
                settings::mcp_management_form::FormMessage::Open(None)
            ))
        ),
        toolbar_button(
            "从应用导入",
            Some(settings::mcp_management_import::message(
                settings::mcp_management_import::ImportMessage::Open
            ))
        ),
        toolbar_button(
            "刷新",
            Some(Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::Refresh)))
        ),
        toolbar_button(
            "更多",
            Some(message(
                scope.clone(),
                Action::Menu(actions::MenuMessage::Action(actions::Message::More))
            ))
        ),
    ]
    .spacing(8)
    .wrap();
    let matrix = matrix::panel(
        settings::mcp_management_list::visible(app),
        &app.mcp_settings.bindings.matrix,
        &targets,
        &shown,
        &scope.context.host,
        settings::mcp_management_list::loading(app),
        app.mcp_settings.mutating() || app.apps_catalog.loading,
    )
    .map(move |event| message(scope.clone(), Action::Matrix(event)));
    column![
        toolbar,
        scrollable(
            container(
                column![
                    brands,
                    matrix,
                    super::super::agent_apps_diagnostics::mcp_panel::view(app),
                    super::super::system_settings_mcp::resync::rewrite_panel(app),
                ]
                .spacing(12)
            )
            .padding([0, 7])
        )
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4)
        ))
        .height(Length::Fill)
    ]
    .spacing(12)
    .height(Length::Fill)
    .into()
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if !super::is_visible(app)
        || app.apps_catalog.third_party_page
            != desktop_ui_controls::settings_navigation::ThirdPartyPage::Mcp
        || super::mcp_scope(app).as_ref() != Some(&event.scope)
    {
        return Task::none();
    }
    let scope = event.scope;
    let records = settings::mcp_management_list::visible(app)
        .and_then(|r| r.as_ref().ok())
        .map(|c| c.records.clone())
        .unwrap_or_default();
    match event.action {
        Action::Matrix(matrix::ViewMessage::Binding(action)) => super::mcp_update(
            app,
            desktop_settings_mcp::third_party::Message {
                scope,
                action: desktop_settings_mcp::third_party::Action::Bindings(
                    BindingsMessage::Matrix(action),
                ),
            },
        ),
        Action::Matrix(matrix::ViewMessage::Edit(reference)) => {
            let Some(record) = records.into_iter().find(|r| r.record == reference) else {
                return Task::none();
            };
            settings::mcp_management_form::update(
                app,
                settings::mcp_management_form::FormMessage::Open(Some(record)),
            )
        }
        Action::Matrix(matrix::ViewMessage::Menu(reference)) => {
            let Some(record) = records.into_iter().find(|r| r.record == reference) else {
                return Task::none();
            };
            update(
                app,
                Event {
                    scope,
                    action: Action::Menu(actions::MenuMessage::Action(actions::Message::Row(
                        record,
                    ))),
                },
            )
        }
        Action::Menu(actions::MenuMessage::Action(action)) => {
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
    let Some(scope) = app.apps_catalog.third_party_mcp_scope.as_ref() else {
        return base;
    };
    let state = &app.mcp_settings.actions;
    let current = state.scope.as_ref() == Some(scope)
        && settings::mcp_management_list::visible(app)
            .and_then(|c| c.as_ref().ok())
            .is_some_and(|c| c.records == state.records);
    let base = if current && (state.delete.is_some() || state.export.is_some()) {
        let overlay =
            actions::overlay(state, Space::new().width(Length::Fill).height(Length::Fill).into())
                .map(move |event| {
                    message(scope.clone(), Action::Menu(actions::MenuMessage::Action(event)))
                });
        stack![base, overlay].into()
    } else if current && (state.more || state.row.is_some()) {
        settings_modal_overlay(
            Some(base),
            message(
                scope.clone(),
                Action::Menu(actions::MenuMessage::Action(actions::Message::Close)),
            ),
            settings_modal_card(
                actions::menu(state).map(move |event| message(scope.clone(), Action::Menu(event))),
            )
            .width(380),
        )
    } else {
        base
    };
    super::super::system_settings_mcp::third_party_overlays(app, base)
}

fn toolbar_button(
    label: &'static str,
    event: Option<Message>,
) -> iced::widget::Button<'static, Message> {
    let button =
        iced::widget::button(text(label).size(13)).padding([7, 14]).style(rounded_action_btn_style);
    if let Some(event) = event { button.on_press(event) } else { button }
}
