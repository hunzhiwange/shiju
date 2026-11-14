//! SDK reads are tied to the current project, execution host and request generation.
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;
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
    Refresh(ProjectProfileScope),
    Query(ProjectProfileScope, String),
    Loaded(Identity, ProjectProfileScope, Result<(Vec<ProjectProfile>, Option<ProjectProfileId>), ()>),
}
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::ProjectProfiles(event)))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let Some(project) = app.project_id.clone().filter(|id| !id.trim().is_empty()) else { return Task::none(); };
    if app.apps_catalog.context.as_ref() != Some(&context) || !app.apps_catalog.connection_open || app.apps_catalog.token.is_empty() {
        return Task::none();
    }
    let scope = match &event { Event::Refresh(s) | Event::Query(s, _) | Event::Loaded(_, s, _) => *s };
    if app.apps_catalog.expanded != Some(scope.app_id()) { return Task::none(); }
    let state = &mut app.apps_catalog.project_profiles;
    match event {
        Event::Query(_, query) => { state.groups.get_mut(scope).query = query; Task::none() }
        Event::Loaded(identity, _, result) => {
            if identity.context != context || identity.project != project || identity.host != app.apps_catalog.host
                || identity.catalog != app.apps_catalog.generation || identity.generation != state.generation { return Task::none(); }
            let group = state.groups.get_mut(scope);
            if !group.loading { return Task::none(); }
            group.loading = false;
            match result {
                Ok((profiles, current)) => { group.profiles = profiles; group.current = current; group.loaded = true; }
                Err(()) => { group.error = true; return app.show_error_toast("组合读取失败，请重试"); }
            }
            Task::none()
        }
        Event::Refresh(_) => {
            state.generation = state.generation.wrapping_add(1);
            let group = state.groups.get_mut(scope);
            group.loading = true;
            group.loaded = false;
            group.error = false;
            group.profiles.clear();
            group.current = None;
            let identity = Identity { context: context.clone(), project: project.clone(), host: app.apps_catalog.host.clone(), catalog: app.apps_catalog.generation, generation: state.generation };
            let Ok(project) = ProjectScope::new(context.tenant, project.into()) else { group.loading = false; return Task::none(); };
            let query = ProjectProfileApiScope { project, target_host_id: identity.host.clone(), scope };
            Task::perform(async move {
                let result = async {
                    let profiles = client.project_profile_list(&query).await.map_err(|_| ())?;
                    let current = client.project_profile_current(&query).await.map_err(|_| ())?;
                    Ok((profiles, current.profile_id))
                }.await;
                message(Event::Loaded(identity, scope, result))
            }, |message| message)
        }
    }
}
