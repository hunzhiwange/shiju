//! Single submission; only a complete, matching executor result confirms application.
use super::{Identity, message};
use crate::project_profiles::{Application, Preview};
use iced::Task;
use gateway_project_profile::GatewayProjectProfileClient;
use {
    gateway_project_profile::ProjectProfileApiScope,
    vw_api_transport::ProjectScope,
    vw_api_project_profiles::*,
};

#[derive(Clone)]
pub enum Event<C> {
    Open(ProjectProfileId, u64),
    Close,
    Confirm,
    Recover,
    Cancel,
    Progress(Identity<C>, String, Result<serde_json::Value, ()>),
    Previewed(Identity<C>, Result<serde_json::Value, ()>),
    Applied(Identity<C>, Result<serde_json::Value, ()>),
}
pub(crate) fn update<C: Clone + PartialEq + Send + 'static>(state: &mut crate::project_profiles::State<Identity<C>>, catalog: &super::Catalog<'_, C>, client: GatewayProjectProfileClient, scope: ProjectProfileScope, event: Event<C>) -> Task<super::Event<C>> {
    let context = catalog.context.clone();
    let project = catalog.project.clone();
    let response = matches!(&event, Event::Applied(..) | Event::Previewed(..) | Event::Progress(..));
    if !response && !catalog.connected { return Task::none(); }
    let identity = Identity { context: context.clone(), project: project.clone(), host: catalog.host.to_owned(), catalog: catalog.generation, generation: state.generation };
    let Ok(project) = ProjectScope::new(catalog.tenant.clone(), project.into()) else { return Task::none(); };
    let query = ProjectProfileApiScope { project, target_host_id: identity.host.clone(), scope };
    if matches!(&event, Event::Recover | Event::Cancel | Event::Progress(..)) {
        return super::progress::update(state, catalog, client, scope, event, identity, query);
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
            super::progress::admitted(state, catalog, client, scope, response, result, identity)

        }
    }
}
fn matches_identity<C: PartialEq>(a: &Identity<C>, b: &Identity<C>) -> bool {
    a.context == b.context && a.project == b.project && a.host == b.host && a.catalog == b.catalog && a.generation == b.generation
}
