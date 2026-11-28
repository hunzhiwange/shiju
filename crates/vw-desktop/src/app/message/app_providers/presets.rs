use super::{desktop, edit, list};
use crate::app::{App, Message};
use desktop_provider_apps::{
    components::presets::{RESULTS, SEARCH},
    presets::{self, Availability, Event, Picker, Preset},
};
use iced::{Task, widget::operation};
use vw_api_app_identity::AppId;

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(scope) = app.app_providers.scope.clone() else {
        return Task::none();
    };
    if presets::app_name(scope.app).is_none() {
        return Task::none();
    }
    if matches!(event, Event::Open) {
        let state = &mut app.app_providers;
        if state.loading
            || state.form.is_some()
            || state.desktop.is_some()
            || state.editor.is_some()
            || state.tray_switch.is_some()
            || state.preset_picker.is_some()
        {
            return Task::none();
        }
        state.preset_picker = Some(Picker::default());
        return Task::batch([operation::focus(SEARCH), load_catalog(app, scope)]);
    }
    let Some(picker) = &mut app.app_providers.preset_picker else {
        return Task::none();
    };
    match event {
        Event::Close => app.app_providers.preset_picker = None,
        Event::Query(query) => {
            picker.query = query;
            picker.family = None;
            picker.catalog_provider = None;
            picker.account_help = false;
            return to_top();
        }
        Event::Filter(category) => {
            picker.category = category;
            picker.family = None;
            picker.catalog_provider = None;
            picker.account_help = false;
            return to_top();
        }
        Event::OpenVersions(family) => {
            picker.family = Some(family);
            picker.account_help = false;
            return operation::snap_to(
                desktop_provider_apps::components::presets::VERSIONS,
                iced::widget::scrollable::RelativeOffset::START,
            );
        }
        Event::BackToCatalog => {
            picker.family = None;
            picker.catalog_provider = None;
        }
        Event::RetryCatalog => {
            if matches!(picker.catalog, Some(Err(()))) {
                return load_catalog(app, scope);
            }
        }
        Event::CatalogLoaded { scope: loaded, request, mut rows } => {
            if loaded == scope && picker.catalog_request == request {
                if let Ok(rows) = &mut rows {
                    presets::models_dev::remove_native_duplicates(scope.app, rows);
                }
                picker.catalog = Some(rows);
            }
        }
        Event::ChooseCatalogProvider(index) => {
            let Some(preset) = picker
                .catalog
                .as_ref()
                .and_then(|r| r.as_ref().ok())
                .and_then(|rows| rows.get(index))
            else {
                return Task::none();
            };
            if scope.app == AppId::ClaudeDesktop {
                let preset = preset.clone();
                if !list::current(app, scope.app).is_ok_and(|(_, current)| current == scope) {
                    return Task::none();
                }
                let task = create(app, scope.app, None);
                if let Some(form) = &mut app.app_providers.desktop {
                    presets::models_dev::fill_desktop(form, &preset);
                }
                return task;
            }
            picker.catalog_provider = Some(index);
            picker.catalog_model_query.clear();
            return operation::focus(desktop_provider_apps::components::presets::MODEL_SEARCH);
        }
        Event::CatalogModelQuery(query) => {
            picker.catalog_model_query = query;
            return operation::snap_to(
                desktop_provider_apps::components::presets::VERSIONS,
                iced::widget::scrollable::RelativeOffset::START,
            );
        }
        Event::ChooseCatalogModel(index) => {
            let Some(preset) = picker
                .catalog
                .as_ref()
                .and_then(|r| r.as_ref().ok())
                .and_then(|rows| rows.get(picker.catalog_provider?))
            else {
                return Task::none();
            };
            let Some(model) = preset.models.get(index).cloned() else { return Task::none() };
            let preset = preset.clone();
            if !list::current(app, scope.app).is_ok_and(|(_, current)| current == scope) {
                return Task::none();
            }
            let task = create(app, scope.app, None);
            if let Some(form) = &mut app.app_providers.form {
                presets::models_dev::fill_editor(form, &preset, &model);
            }
            return task;
        }
        Event::OpenAuthorization => {
            #[cfg(not(target_arch = "wasm32"))]
            if picker.account_help
                && list::current(app, scope.app).is_ok_and(|(_, current)| current == scope)
            {
                return crate::app::components::third_party_settings::navigate(
                    app,
                    desktop_ui_controls::settings_navigation::ThirdPartyPage::Authorization,
                );
            }
        }
        Event::Custom | Event::Select(_) => {
            if !list::current(app, scope.app).is_ok_and(|(_, current)| current == scope) {
                return Task::none();
            }
            if let Event::Select(name) = event {
                let Some(preset) = presets::entries(scope.app)
                    .into_iter()
                    .find(|entry| entry.name == name)
                    .map(|entry| entry.preset)
                else {
                    return Task::none();
                };
                match preset.availability() {
                    Availability::Official(id) => {
                        let saved = app.app_providers.snapshot.as_ref().is_some_and(|snapshot| {
                            snapshot.catalog.rows.iter().any(|row| row.provider.id == id)
                        });
                        if saved {
                            app.app_providers.preset_picker = None;
                            return if scope.app == AppId::ClaudeDesktop {
                                desktop::update(app, desktop::Event::Open(Some(id.into())))
                            } else {
                                edit::update(
                                    app,
                                    edit::EditMessage::Open {
                                        id: id.into(),
                                        copy: false,
                                        delete: false,
                                    },
                                )
                            };
                        }
                    }
                    Availability::Direct => return create(app, scope.app, Some(preset)),
                    Availability::Routing if preset.account_label().is_some() => {
                        if let Some(picker) = &mut app.app_providers.preset_picker {
                            picker.account_help = true;
                        }
                        return to_top();
                    }
                    Availability::Routing | Availability::Unavailable => {}
                }
            } else {
                return create(app, scope.app, None);
            }
        }
        Event::Open => unreachable!("picker opening is handled above"),
    }
    Task::none()
}

fn load_catalog(app: &mut App, scope: desktop_provider_apps::Scope) -> Task<Message> {
    let client = list::current(app, scope.app)
        .ok()
        .filter(|(_, current)| *current == scope)
        .map(|(client, _)| client);
    let Some(picker) = &mut app.app_providers.preset_picker else {
        return Task::none();
    };
    let Some(client) = client else {
        picker.catalog = Some(Err(()));
        return Task::none();
    };
    let request = presets::new_id();
    picker.catalog_request.clone_from(&request);
    picker.catalog = None;
    let app_id = scope.app;
    Task::perform(
        async move {
            gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone())
                .app_provider_models_dev(app_id)
                .await
                .map_err(|_| ())
        },
        move |rows| {
            list::message(list::ListMessage::Presets(Event::CatalogLoaded {
                scope: scope.clone(),
                request: request.clone(),
                rows,
            }))
        },
    )
}

fn create(app: &mut App, id: AppId, preset: Option<Preset>) -> Task<Message> {
    app.app_providers.preset_picker = None;
    if id == AppId::ClaudeDesktop {
        let task = desktop::update(app, desktop::Event::Open(None));
        if let Some(form) = &mut app.app_providers.desktop {
            form.id = presets::new_id();
            if let Some(preset) = preset {
                preset.fill_desktop(form);
            }
        }
        task
    } else {
        let task = edit::update(app, edit::EditMessage::Create);
        if let Some(form) = &mut app.app_providers.form {
            form.id = presets::new_id();
            if let Some(preset) = preset {
                preset.fill_editor(form);
            }
        }
        task
    }
}

fn to_top() -> Task<Message> {
    operation::snap_to(RESULTS, iced::widget::scrollable::RelativeOffset::START)
}
