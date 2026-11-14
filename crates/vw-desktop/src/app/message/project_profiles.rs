//! SDK reads are tied to the current project, execution host and request generation.
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;
#[path = "project_profiles_apply.rs"]
mod apply;
#[path = "project_profiles_progress.rs"]
mod progress;
pub use apply::Event as ApplyEvent;
use crate::app::state::project_profiles::{Action, Management};
use vw_gateway_client::{project_profile_api::ProjectProfileApiScope, vw_api_types::{ProjectScope, project_profiles::*}};
#[derive(Clone)]
pub struct Identity {
    context: AgentAppsContext,
    project: String,
    host: String,
    catalog: u64,
    generation: u64,
}
#[derive(Clone)]
pub enum Event {
    Apply(ProjectProfileScope, apply::Event),
    Open(ProjectProfileScope, Action),
    Name(ProjectProfileScope, String),
    Close(ProjectProfileScope),
    Submit(ProjectProfileScope),
    Saved(Identity, ProjectProfileScope, Result<(), ()>),
    Refresh(ProjectProfileScope),
    Query(ProjectProfileScope, String),
    Loaded(Identity, ProjectProfileScope, Result<(Vec<ProjectProfile>, Option<ProjectProfileId>, Result<Option<ProjectProfileApplyResult>, ()>), ()>),
}
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::ProjectProfiles(event)))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Apply(scope, event) = event { return apply::update(app, scope, event); }
    if let Event::Refresh(scope) = &event {
        if app.apps_catalog.project_profiles.application.as_ref().is_some_and(|f| f.scope == *scope && f.submitted && f.result.is_none()) {
            return apply::update(app, *scope, apply::Event::Recover);
        }
    }
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let Some(project) = app.project_id.clone().filter(|id| !id.trim().is_empty()) else { return Task::none(); };
    if app.apps_catalog.context.as_ref() != Some(&context) || !app.apps_catalog.connection_open || app.apps_catalog.token.is_empty() {
        return Task::none();
    }
    let scope = match &event { Event::Apply(..) => unreachable!(), Event::Open(s, _) | Event::Name(s, _) | Event::Close(s) | Event::Submit(s) | Event::Saved(_, s, _) | Event::Refresh(s) | Event::Query(s, _) | Event::Loaded(_, s, _) => *s };
    if app.apps_catalog.expanded != Some(scope.app_id()) { return Task::none(); }
    let state = &mut app.apps_catalog.project_profiles;
    match event {
        Event::Apply(..) => unreachable!(),
        Event::Open(_, action) => {
            if state.application.is_some() || state.management.is_some() || state.groups.get(scope).loading { return Task::none(); }
            let name = match &action {
                Action::Create | Action::Clear => String::new(),
                Action::Rename(id, version) | Action::Resnapshot(id, version) | Action::Delete(id, version) => {
                    let Some(profile) = state.groups.get(scope).profiles.iter().find(|p| &p.id == id && p.version == *version) else { return Task::none(); };
                    profile.name.clone()
                }
            };
            state.management = Some(Management { scope, action, name, busy: false });
            Task::none()
        }
        Event::Name(_, name) => {
            if let Some(form) = state.management.as_mut().filter(|f| f.scope == scope && !f.busy) { form.name = name; }
            Task::none()
        }
        Event::Close(_) => {
            if state.management.as_ref().is_some_and(|f| f.scope == scope && !f.busy) { state.management = None; }
            Task::none()
        }
        Event::Submit(_) => {
            let Some(form) = state.management.as_mut().filter(|f| f.scope == scope && !f.busy) else { return Task::none(); };
            if form.action.needs_name() && form.name.trim().is_empty() { return Task::none(); }
            let Ok(project_scope) = ProjectScope::new(context.tenant.clone(), project.clone().into()) else { return Task::none(); };
            form.busy = true;
            let action = form.action.clone();
            let name = form.name.trim().to_owned();
            let identity = Identity { context, project, host: app.apps_catalog.host.clone(), catalog: app.apps_catalog.generation, generation: state.generation };
            let query = ProjectProfileApiScope { project: project_scope, target_host_id: identity.host.clone(), scope };
            Task::perform(async move {
                let result = match action {
                    Action::Create => client.project_profile_create(&query, &name).await.map(|_| ()),
                    Action::Rename(id, version) => client.project_profile_rename(&query, &id, version, &name).await.map(|_| ()),
                    Action::Resnapshot(id, version) => client.project_profile_resnapshot(&query, &id, version).await.map(|_| ()),
                    Action::Delete(id, version) => client.project_profile_delete(&query, &id, version).await.and_then(|r| if r.deleted { Ok(()) } else { Err(vw_gateway_client::project_profile_api::ProjectProfileApiError::InvalidRequest("delete unconfirmed".into())) }),
                    Action::Clear => client.project_profile_clear(&query).await.and_then(|r| if r.profile_id.is_none() { Ok(()) } else { Err(vw_gateway_client::project_profile_api::ProjectProfileApiError::InvalidRequest("clear unconfirmed".into())) }),
                }.map_err(|_| ());
                message(Event::Saved(identity, scope, result))
            }, |message| message)
        }
        Event::Saved(identity, _, result) => {
            if identity.context != context || identity.project != project || identity.host != app.apps_catalog.host
                || identity.catalog != app.apps_catalog.generation || identity.generation != state.generation { return Task::none(); }
            let Some(form) = state.management.as_mut().filter(|f| f.scope == scope && f.busy) else { return Task::none(); };
            form.busy = false;
            if result.is_err() { return app.show_error_toast("组合维护失败，服务端未确认成功；请重试，版本冲突时取消并刷新"); }
            state.management = None;
            update(app, Event::Refresh(scope))
        }
        Event::Query(_, query) => { state.groups.get_mut(scope).query = query; Task::none() }
        Event::Loaded(identity, _, result) => {
            if identity.context != context || identity.project != project || identity.host != app.apps_catalog.host
                || identity.catalog != app.apps_catalog.generation || identity.generation != state.generation { return Task::none(); }
            let group = state.groups.get_mut(scope);
            if !group.loading { return Task::none(); }
            group.loading = false;
            match result {
                Ok((profiles, current, stored)) => {
                    group.profiles = profiles;
                    group.current = current;
                    group.loaded = true;
                    match stored {
                        Ok(result) if result.as_ref().is_none_or(|r| r.group == scope) => group.stored_result = result,
                        _ => group.result_error = true,
                    }
                }
                Err(()) => { group.error = true; return app.show_error_toast("组合读取失败，请重试"); }
            }
            Task::none()
        }
        Event::Refresh(_) => {
            if state.management.is_some() || state.application.as_ref().is_some_and(|f| f.loading) { return Task::none(); }
            state.generation = state.generation.wrapping_add(1);
            let group = state.groups.get_mut(scope);
            group.loading = true;
            group.loaded = false;
            group.error = false;
            group.stored_result = None;
            group.result_error = false;
            group.profiles.clear();
            group.current = None;
            let identity = Identity { context: context.clone(), project: project.clone(), host: app.apps_catalog.host.clone(), catalog: app.apps_catalog.generation, generation: state.generation };
            let Ok(project) = ProjectScope::new(context.tenant, project.into()) else { group.loading = false; return Task::none(); };
            let query = ProjectProfileApiScope { project, target_host_id: identity.host.clone(), scope };
            Task::perform(async move {
                let result = async {
                    let profiles = client.project_profile_list(&query).await.map_err(|_| ())?;
                    let current = client.project_profile_current(&query).await.map_err(|_| ())?;
                    let stored = client.project_profile_result(&query).await.map(|r| r.result).map_err(|_| ());
                    Ok((profiles, current.profile_id, stored))
                }.await;
                message(Event::Loaded(identity, scope, result))
            }, |message| message)
        }
    }
}
