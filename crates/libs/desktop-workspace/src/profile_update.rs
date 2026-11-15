//! SDK reads are tied to the current project, execution host and request generation.
use super::project_profiles::{Action, Management};
use vw_api_transport::TenantScope;
use iced::Task;
#[path = "profile_apply.rs"]
mod apply;
#[path = "profile_progress.rs"]
mod progress;
pub use apply::Event as ApplyEvent;

use {
    gateway_project_profile::ProjectProfileApiScope,
    vw_api_transport::ProjectScope,
    vw_api_project_profiles::*,
};
#[derive(Clone)]
pub struct Identity<C> {
    context: C,
    project: String,
    host: String,
    catalog: u64,
    generation: u64,
}
#[derive(Clone)]
pub enum Event<C> {
    Toast(bool, &'static str),
    Apply(ProjectProfileScope, apply::Event<C>),
    Open(ProjectProfileScope, Action),
    Name(ProjectProfileScope, String),
    Close(ProjectProfileScope),
    Submit(ProjectProfileScope),
    Saved(Identity<C>, ProjectProfileScope, Result<(), ()>),
    Refresh(ProjectProfileScope),
    Query(ProjectProfileScope, String),
    Loaded(Identity<C>, ProjectProfileScope, Result<(Vec<ProjectProfile>, Option<ProjectProfileId>, Result<Option<ProjectProfileApplyResult>, ()>), ()>),
}
fn message<C>(event: Event<C>) -> Event<C> { event }

/// Current catalog identity and admission flags, separate from profile state.
pub struct Catalog<'a, C> {
    pub context: C,
    pub tenant: TenantScope,
    pub project: String,
    pub host: &'a str,
    pub generation: u64,
    pub connected: bool,
    pub expanded: Option<vw_api_app_identity::AppId>,
}
pub fn update<C: Clone + PartialEq + Send + 'static>(
    state: &mut super::project_profiles::State<Identity<C>>,
    catalog: &Catalog<'_, C>,
    client: gateway_project_profile::GatewayProjectProfileClient,
    event: Event<C>,
) -> Task<Event<C>> {
    if matches!(event, Event::Toast(..)) { return Task::none(); }
    if let Event::Apply(scope, event) = event { return apply::update(state, catalog, client, scope, event); }
    if let Event::Refresh(scope) = &event {
        if state.application.as_ref().is_some_and(|f| f.scope == *scope && f.submitted && f.result.is_none()) {
            return apply::update(state, catalog, client, *scope, apply::Event::Recover);
        }
    }
    let context = catalog.context.clone();
    let project = catalog.project.clone();
    if !catalog.connected { return Task::none(); }
    let scope = match &event { Event::Toast(..) | Event::Apply(..) => unreachable!(), Event::Open(s, _) | Event::Name(s, _) | Event::Close(s) | Event::Submit(s) | Event::Saved(_, s, _) | Event::Refresh(s) | Event::Query(s, _) | Event::Loaded(_, s, _) => *s };
    if catalog.expanded != Some(scope.app_id()) { return Task::none(); }

    match event {
        Event::Toast(..) | Event::Apply(..) => unreachable!(),
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
            let Ok(project_scope) = ProjectScope::new(catalog.tenant.clone(), project.clone().into()) else { return Task::none(); };
            form.busy = true;
            let action = form.action.clone();
            let name = form.name.trim().to_owned();
            let identity = Identity { context, project, host: catalog.host.to_owned(), catalog: catalog.generation, generation: state.generation };
            let query = ProjectProfileApiScope { project: project_scope, target_host_id: identity.host.clone(), scope };
            Task::perform(async move {
                let result = match action {
                    Action::Create => client.project_profile_create(&query, &name).await.map(|_| ()),
                    Action::Rename(id, version) => client.project_profile_rename(&query, &id, version, &name).await.map(|_| ()),
                    Action::Resnapshot(id, version) => client.project_profile_resnapshot(&query, &id, version).await.map(|_| ()),
                    Action::Delete(id, version) => client.project_profile_delete(&query, &id, version).await.and_then(|r| if r.deleted { Ok(()) } else { Err(gateway_project_profile::ProjectProfileApiError::InvalidRequest("delete unconfirmed".into())) }),
                    Action::Clear => client.project_profile_clear(&query).await.and_then(|r| if r.profile_id.is_none() { Ok(()) } else { Err(gateway_project_profile::ProjectProfileApiError::InvalidRequest("clear unconfirmed".into())) }),
                }.map_err(|_| ());
                message(Event::Saved(identity, scope, result))
            }, |message| message)
        }
        Event::Saved(identity, _, result) => {
            if identity.context != context || identity.project != project || identity.host != catalog.host
                || identity.catalog != catalog.generation || identity.generation != state.generation { return Task::none(); }
            let Some(form) = state.management.as_mut().filter(|f| f.scope == scope && f.busy) else { return Task::none(); };
            form.busy = false;
            if result.is_err() { return Task::done(Event::Toast(true, "组合维护失败，服务端未确认成功；请重试，版本冲突时取消并刷新")); }
            state.management = None;
            update(state, catalog, client, Event::Refresh(scope))
        }
        Event::Query(_, query) => { state.groups.get_mut(scope).query = query; Task::none() }
        Event::Loaded(identity, _, result) => {
            if identity.context != context || identity.project != project || identity.host != catalog.host
                || identity.catalog != catalog.generation || identity.generation != state.generation { return Task::none(); }
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
                Err(()) => { group.error = true; return Task::done(Event::Toast(true, "组合读取失败，请重试")); }
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
            let identity = Identity { context: context.clone(), project: project.clone(), host: catalog.host.to_owned(), catalog: catalog.generation, generation: state.generation };
            let Ok(project) = ProjectScope::new(catalog.tenant.clone(), project.into()) else { group.loading = false; return Task::none(); };
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
