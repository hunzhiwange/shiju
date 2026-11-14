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
    Previewed(Identity, Result<serde_json::Value, ()>),
    Applied(Identity, Result<serde_json::Value, ()>),
}
pub(crate) fn update(app: &mut App, scope: ProjectProfileScope, event: Event) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let Some(project) = app.project_id.clone().filter(|p| !p.trim().is_empty()) else { return Task::none(); };
    if app.apps_catalog.context.as_ref() != Some(&context) || !app.apps_catalog.connection_open
        || app.apps_catalog.token.is_empty() || app.apps_catalog.expanded != Some(scope.app_id()) { return Task::none(); }
    let state = &mut app.apps_catalog.project_profiles;
    let identity = Identity { context: context.clone(), project: project.clone(), host: app.apps_catalog.host.clone(), catalog: app.apps_catalog.generation, generation: state.generation };
    let Ok(project) = ProjectScope::new(context.tenant.clone(), project.into()) else { return Task::none(); };
    let query = ProjectProfileApiScope { project, target_host_id: identity.host.clone(), scope };
    match event {
        Event::Open(id, version) => {
            if state.management.is_some() || state.application.is_some() || state.groups.get(scope).loading { return Task::none(); }
            let Some(profile) = state.groups.get(scope).profiles.iter().find(|p| p.id == id && p.version == version) else { return Task::none(); };
            state.application = Some(Application { scope, id: id.clone(), version, name: profile.name.clone(), loading: true, submitted: false, preview: None, result: None, error: None });
            Task::perform(async move { message(super::Event::Apply(scope, Event::Previewed(identity, client.project_profile_preview(&query, &id, version).await.map_err(|_| ())))) }, |m| m)
        }
        Event::Close => {
            if state.application.as_ref().is_some_and(|f| f.scope == scope && !f.loading) { state.application = None; }
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
            if !matches_identity(&response, &identity) { return Task::none(); }
            let Some(form) = state.application.as_mut().filter(|f| f.scope == scope && f.loading && f.submitted) else { return Task::none(); };
            form.loading = false;
            // An admission/operation ID or an old stored group result is not a receipt.
            form.result = result.ok().and_then(|v| serde_json::from_value::<ProjectProfileApplyResult>(v).ok())
                .filter(|r| r.profile_id == form.id && r.group == scope
                    && (r.status != ProjectProfileApplyStatus::Applied || (r.selected && r.items.iter().all(|item| item.status == ProjectProfileItemStatus::Succeeded))));
            if form.result.is_none() { form.error = Some("应用结果尚未确认；请刷新执行主机状态。此请求不会自动重发。局部写入可能已发生。开放新操作前请核对结果。 "); }
            // Invalidate native observations even for an unknown write outcome.
            app.apps_catalog.loaded_at = None;
            app.apps_catalog.prompts.invalidate();
            let refresh = super::update(app, super::Event::Refresh(scope));
            Task::batch([refresh, app.show_info_toast("应用请求已返回，请核对逐项结果；原生状态需重新读取")])
        }
    }
}
fn matches_identity(a: &Identity, b: &Identity) -> bool {
    a.context == b.context && a.project == b.project && a.host == b.host && a.catalog == b.catalog && a.generation == b.generation
}
