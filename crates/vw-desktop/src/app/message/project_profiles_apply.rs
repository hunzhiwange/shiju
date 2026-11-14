//! Single submission; only a complete, matching executor result confirms application.
use super::{Identity, message};
use crate::app::{App, Message, state::{AgentAppsContext, project_profiles::{Application, Preview}}};
use iced::Task;
use vw_gateway_client::{project_profile_api::ProjectProfileApiScope, vw_api_types::{ProjectScope, project_profiles::*}};

#[derive(Clone)]
pub enum Event {
    Open(ProjectProfileId, u64),
    Close,
    Confirm,
    Recover,
    Cancel,
    Progress(Identity, String, Result<serde_json::Value, ()>),
    Previewed(Identity, Result<serde_json::Value, ()>),
    Applied(Identity, Result<serde_json::Value, ()>),
}
pub(crate) fn update(app: &mut App, scope: ProjectProfileScope, event: Event) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let Some(project) = app.project_id.clone().filter(|p| !p.trim().is_empty()) else { return Task::none(); };
    let response = matches!(&event, Event::Applied(..) | Event::Previewed(..) | Event::Progress(..));
    if app.apps_catalog.context.as_ref() != Some(&context)
        || (!response && !app.apps_catalog.connection_open) || app.apps_catalog.token.is_empty() { return Task::none(); }
    let state = &mut app.apps_catalog.project_profiles;
    let identity = Identity { context: context.clone(), project: project.clone(), host: app.apps_catalog.host.clone(), catalog: app.apps_catalog.generation, generation: state.generation };
    let Ok(project) = ProjectScope::new(context.tenant.clone(), project.into()) else { return Task::none(); };
    let query = ProjectProfileApiScope { project, target_host_id: identity.host.clone(), scope };
    if matches!(&event, Event::Recover | Event::Cancel | Event::Progress(..)) {
        return super::progress::update(app, scope, event, identity, client, query);
    }
    match event {
        Event::Recover | Event::Cancel | Event::Progress(..) => unreachable!(),
        Event::Open(id, version) => {
            if state.management.is_some() || state.application.is_some() || state.groups.get(scope).loading { return Task::none(); }
            let Some(profile) = state.groups.get(scope).profiles.iter().find(|p| p.id == id && p.version == version) else { return Task::none(); };
            state.application = Some(Application { scope, owner: identity.clone(), operation_id: None, cancelling: false, id: id.clone(), version, name: profile.name.clone(), loading: true, submitted: false, preview: None, result: None, error: None });
            Task::perform(async move { message(super::Event::Apply(scope, Event::Previewed(identity, client.project_profile_preview(&query, &id, version).await.map_err(|_| ())))) }, |m| m)
        }
        Event::Close => {
            if state.application.as_ref().is_some_and(|f| f.scope == scope && !f.loading && (!f.submitted || f.result.is_some())) { state.application = None; }
            Task::none()
        }
        Event::Confirm => {
            let Some(form) = state.application.as_mut().filter(|f| f.scope == scope && !f.loading && !f.submitted && f.preview.is_some()) else { return Task::none(); };
            form.submitted = true;
            form.loading = true;
            let (id, version) = (form.id.clone(), form.version);
            Task::perform(async move { message(super::Event::Apply(scope, Event::Applied(identity, client.project_profile_apply(&query, &id, version).await.map_err(|_| ())))) }, |m| m)
        }
        Event::Previewed(response, result) => {
            if !matches_identity(&response, &identity) { return Task::none(); }
            let Some(form) = state.application.as_mut().filter(|f| f.scope == scope && f.loading && !f.submitted) else { return Task::none(); };
            form.loading = false;
            match result.ok().and_then(|v| serde_json::from_value::<Preview>(v).ok()).filter(|p| p.group == scope && (p.captured || p.items.is_empty())) {
                Some(preview) => form.preview = Some(preview),
                None => form.error = Some("执行主机未提供可识别的完整预览，无法应用；请稍后刷新。"),
            }
            Task::none()
        }
        Event::Applied(response, result) => {
            super::progress::admitted(app, scope, response, result, identity)

        }
    }
}
fn matches_identity(a: &Identity, b: &Identity) -> bool {
    a.context == b.context && a.project == b.project && a.host == b.host && a.catalog == b.catalog && a.generation == b.generation
}
