//! Final desktop composition for third-party resources and application details.
use crate::app::{App, Message, message::{SettingsMessage, settings::AppsCatalogMessage}};
use desktop_ui_controls::settings_navigation::ThirdPartyPage;
use iced::{Element, Task, widget::{button, column, row, text}};
use super::system_settings_common::{rounded_action_btn_style, settings_page_heading};

fn message(event: AppsCatalogMessage) -> Message {
    Message::Settings(SettingsMessage::AppsCatalog(event))
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.apps_catalog;
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    let mut tabs = row![].spacing(8);
    for &(page, label) in ThirdPartyPage::ALL {
        tabs = tabs.push(button(text(i18n.__(label))).padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(message(AppsCatalogMessage::Navigate(page))));
    }
    let mut applications = row![].spacing(8);
    if matches!(state.third_party_page, ThirdPartyPage::Mcp | ThirdPartyPage::Skills | ThirdPartyPage::Usage) {
        for entry in &state.rows {
            applications = applications.push(button(text(format!("{:?}", entry.entry.id))).padding([8, 12])
                .style(rounded_action_btn_style).on_press(message(AppsCatalogMessage::SelectApp(entry.entry.id))));
        }
    }
    let selected = state.expanded;
    let body: Element<'_, Message> = match state.third_party_page {
        ThirdPartyPage::Agents => super::system_settings_apps::view(state, i18n),
        ThirdPartyPage::Mcp => match mcp_scope(app) {
            Some(scope) => {
                let installations = state.rows.iter().find(|r| Some(r.entry.id) == selected)
                    .and_then(|r| r.observation.as_ref().ok())
                    .map(|r| r.installations.as_slice()).unwrap_or(&[]);
                // The component captures the scope by value in messages, not this projection.
                mcp_view(app, scope, installations)
            }
            None => text("请先在智能体管理中选择当前主机上的应用").into(),
        },
        ThirdPartyPage::Skills => {
            let applications = state.rows.iter().map(|r| desktop_settings_skills::bindings_view::BindingApplication {
                entry: &r.entry, observation: r.observation.as_ref().ok(),
            }).collect::<Vec<_>>();
            desktop_settings_skills::third_party::view(&state.third_party_skills, &applications)
                .map(|event| message(AppsCatalogMessage::ThirdPartySkills(event)))
        }
        ThirdPartyPage::Sessions => super::third_party_resources::sessions(app),
        #[cfg(not(target_arch = "wasm32"))]
        ThirdPartyPage::Authorization => super::third_party_resources::authorization(app),
        #[cfg(not(target_arch = "wasm32"))]
        ThirdPartyPage::Usage => match selected {
            Some(id) => super::third_party_resources::usage(app, id),
            None => text("请先选择应用，再查看用量与额度").into(),
        },
        #[cfg(target_arch = "wasm32")]
        ThirdPartyPage::Authorization | ThirdPartyPage::Usage => text("此平台不支持授权与用量管理").into(),
        ThirdPartyPage::ManagementTools => column![
            super::third_party_tools::view(app, selected),
            super::system_settings_data::migration_preview_ui::view(app),
            crate::app::message::settings::backup_inventory::view(app),
            super::system_settings_data::export_ui::view(app),
        ].spacing(12).into(),
    };
    column![settings_page_heading(i18n.__("第三方智能体"), ""), tabs.wrap(), applications.wrap(), body].spacing(16).into()
}

fn mcp_scope(app: &App) -> Option<desktop_settings_mcp::third_party::Scope> {
    let app_id = app.apps_catalog.expanded?;
    if app.apps_catalog.loading || !app.apps_catalog.rows.iter().any(|row| row.entry.id == app_id) { return None; }
    Some(desktop_settings_mcp::third_party::Scope {
        context: crate::app::message::settings::mcp_management_list::context(app)?,
        app_id, generation: app.apps_catalog.navigation_generation,
    })
}

// The scope must live with the host state for the duration of the rendered Element.
fn mcp_view<'a>(app: &'a App, scope: desktop_settings_mcp::third_party::Scope,
    installations: &'a [vw_api_app_identity::AppInstallation]) -> Element<'a, Message> {
    // Build the scoped projection using the cached scope owned by navigation.
    let Some(current) = app.apps_catalog.third_party_mcp_scope.as_ref().filter(|current| **current == scope) else {
        return text("请重新打开 MCP 页面以刷新作用域").into();
    };
    desktop_settings_mcp::third_party::view(desktop_settings_mcp::third_party::View {
        scope: current, state: &app.mcp_settings, installations,
        authorized: app.apps_catalog.context.as_ref() == Some(&current.context.endpoint)
            && !app.apps_catalog.token.is_empty(),
    }).map(|event| message(AppsCatalogMessage::ThirdPartyMcp(event)))
}

pub(crate) fn navigate(app: &mut App, page: ThirdPartyPage) -> Task<Message> {
    invalidate(app);
    app.apps_catalog.third_party_page = page;
    use crate::app::message::settings as settings;
    match page {
        ThirdPartyPage::Mcp => {
            app.apps_catalog.third_party_mcp_scope = mcp_scope(app);
            settings::mcp_management_list::update(app, settings::mcp_management_list::CatalogMessage::Refresh)
        }
        ThirdPartyPage::Skills => {
            let scope = app.apps_catalog.expanded.zip(app.apps_catalog.context.clone()).map(|(id, context)|
                desktop_provider_apps::Scope { context, host: app.apps_catalog.host.clone(), app: id });
            app.apps_catalog.third_party_skills.sync(scope, app.apps_catalog.navigation_generation);
            Task::none()
        }
        ThirdPartyPage::Sessions => super::external_sessions::update(app, super::external_sessions::Event::Refresh),
        #[cfg(not(target_arch = "wasm32"))]
        ThirdPartyPage::Authorization => crate::app::message::agent_apps_auth::update(app,
            crate::app::message::agent_apps_auth::Event::Refresh),
        _ => Task::none(),
    }
}

/// Invalidate transient UI and request generations without changing domain targets.
pub(crate) fn invalidate(app: &mut App) {
    use crate::app::message::settings::skills;
    for event in [
        SettingsMessage::SkillRepositories(skills::repo_modal::Event::Close),
        SettingsMessage::SkillsImport(skills::import_modal::Event::Close),
        SettingsMessage::SkillsInstall(skills::install_view::Event::Close),
        SettingsMessage::SkillsMarket(skills::market_view::Event::Close),
        SettingsMessage::SkillsUpdates(skills::updates_view::Event::Close),
        SettingsMessage::SkillsJobs(skills::jobs_view::Event::Close),
        SettingsMessage::SkillsStorage(skills::storage_modal::Event::Close),
        SettingsMessage::SkillsBackups(skills::backup_modal::Event::Close),
    ] {
        // Close reducers only dismiss presentation; writes keep their original scope.
        let _ = skills::update(app, event);
    }
    let s = &mut app.apps_catalog;
    s.navigation_generation = s.navigation_generation.wrapping_add(1);
    s.third_party_mcp_scope = None;
    s.third_party_skills.sync(None, s.generation);
    s.external_sessions.invalidate();
    s.launch.invalidate(); s.preferences.invalidate(); s.lifecycle.invalidate();
    s.workspace_files.invalidate(); s.daily_memory.invalidate(); s.hermes_memory.invalidate();
    s.openclaw_catalog.invalidate(); s.openclaw_model.invalidate(); s.openclaw_tools.invalidate();
    s.openclaw_env.invalidate(); s.openclaw_defaults.invalidate(); s.hermes_dashboard.invalidate();
    s.prompts.invalidate(); s.routing.invalidate(); s.high_availability.invalidate();
    s.connection_open = false;
    // Provider overlays retain their own request scope; dismiss their presentation.
    app.app_providers.scope = None;
    #[cfg(not(target_arch = "wasm32"))]
    { app.agent_apps_auth.form = None; }
}

pub(crate) fn mcp_update(app: &mut App, event: desktop_settings_mcp::third_party::Message) -> Task<Message> {
    let Some(current) = mcp_scope(app) else { return Task::none(); };
    let mut input = crate::app::message::settings::mcp_management_list::input(app);
    let task = desktop_settings_mcp::third_party::update(&mut input, &current, event)
        .map(|event| message(AppsCatalogMessage::ThirdPartyMcp(event)));
    let effects = std::mem::take(&mut input.effects);
    crate::app::message::settings::mcp_management_tools::apply_effects(app, task, effects)
}

pub(crate) fn skills_update(app: &mut App, event: desktop_settings_skills::third_party::Message) -> Task<Message> {
    use desktop_settings_skills::{third_party::{Effect, Action, DirectoryAction}, update::bindings_view};
    let mut input = crate::app::message::settings::skills::update_input(app);
    input.catalog = app.apps_catalog.navigation_generation;
    match desktop_settings_skills::third_party::update(&mut app.apps_catalog.third_party_skills, &input, event) {
        Effect::None => Task::none(),
        Effect::Binding { scope, catalog, effect } => match effect {
            bindings_view::Effect::None => Task::none(),
            bindings_view::Effect::Error(error) => app.show_error_toast(error),
            bindings_view::Effect::Success(notice) => app.show_success_toast(notice),
            bindings_view::Effect::Request { generation, scope: request_scope, change } => {
                let token = app.apps_catalog.token.clone();
                Task::perform(async move {
                    desktop_session::session_gateway::gateway_skill_bindings_request(
                        crate::app::config::gateway_client_endpoint(), request_scope, token, change).await
                }, move |result| message(AppsCatalogMessage::ThirdPartySkills(desktop_settings_skills::third_party::Message {
                    scope: scope.clone(), catalog, action: Action::Binding(bindings_view::Event::Completed(generation, result)),
                })))
            }
        },
        Effect::Directory { action, .. } => {
            use crate::app::message::settings::skills;
            let event = match action {
                DirectoryAction::Repositories => SettingsMessage::SkillRepositories(skills::repo_modal::Event::Open),
                DirectoryAction::Import => SettingsMessage::SkillsImport(skills::import_modal::Event::Open),
                DirectoryAction::Install => SettingsMessage::SkillsInstall(skills::install_view::Event::Open),
                DirectoryAction::Market => SettingsMessage::SkillsMarket(skills::market_view::Event::Open),
                DirectoryAction::Updates => SettingsMessage::SkillsUpdates(skills::updates_view::Event::Open),
                DirectoryAction::Jobs => SettingsMessage::SkillsJobs(skills::jobs_view::Event::Open),
                DirectoryAction::Storage => SettingsMessage::SkillsStorage(skills::storage_modal::Event::Open),
                DirectoryAction::Backup => SettingsMessage::SkillsBackups(skills::backup_modal::Event::Open),
            };
            skills::update(app, event)
        }
    }
}

pub(crate) fn overlays<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let base = match app.apps_catalog.third_party_page {
        ThirdPartyPage::Skills => super::system_settings_skills::view_overlays(app, base),
        ThirdPartyPage::Mcp => super::system_settings_mcp::view_overlays(app, base),
        _ => base,
    };
    if app.apps_catalog.third_party_page == ThirdPartyPage::ManagementTools {
        super::system_settings_data::export_ui::overlay(app, super::third_party_tools::overlay(app, base))
    } else { base }
}

#[cfg(test)]
#[path = "third_party_settings_tests.rs"]
mod tests;
