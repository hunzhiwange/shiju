use crate::app::state::app_providers::list::{Scope, Snapshot};
use vw_gateway_client::GatewayClient;
use vw_gateway_client::vw_api_types::app_providers::presentation::AppProviderPresentation;
pub(crate) async fn load(
    client: GatewayClient,
    scope: Scope,
    token: String,
) -> Result<Snapshot, String> {
    let (catalog, presentation) = client
        .app_provider_presented_catalog(scope.app, &scope.host, &token)
        .await
        .map_err(|_| "无法读取供应商列表与排序".to_string())?;
    Ok(Snapshot { catalog, presentation })
}
pub(crate) async fn save(
    client: GatewayClient,
    scope: Scope,
    id: String,
    value: AppProviderPresentation,
) -> Result<AppProviderPresentation, String> {
    client.app_provider_presentation_set(scope.app, &id, &value).await
}
#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;

use crate::app::state::app_providers::list::Editor;
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        dsh::DshLiveResult, hermes::HermesLiveResult, kimi::KimiLiveResult, mcode::McodeLiveResult,
        omp::OmpLiveResult, openclaw::OpenClawLiveResult, opencode::OpenCodeLiveResult,
        pi::PiLiveResult,
    },
};
#[derive(Clone, Debug)]
pub enum ListMessage {
    Leave(super::leave::Choice),
    Failover(crate::app::message::agent_apps_high_availability::Event),
    CodexSubagent(crate::app::message::settings::aggregation_subagent::Event),
    DesktopAggregation(crate::app::message::settings::aggregation_desktop::Event),
    OmpDirectory(crate::app::message::settings::aggregation_omp::Event),
    Omo(crate::app::message::settings::aggregation_omo::Event),
    #[cfg(not(target_arch = "wasm32"))]
    Usage(crate::app::components::apps_usage_cards::Event),
    #[cfg(not(target_arch = "wasm32"))]
    UsageSettings(crate::app::components::apps_usage_settings::Event),
    Aggregation(crate::app::message::settings::aggregation_list::Event),
    Desktop(super::desktop::Event),
    Endpoints(super::endpoints::Event),
    EndpointDiagnostics(crate::app::message::agent_apps_diagnostics::endpoints::Event),
    ModelDiagnostics(crate::app::message::agent_apps_diagnostics::model::Event),
    Icons(super::icons::Event),
    Universal(super::universal::Event),
    Snippets(super::snippets::Event),
    Preview(super::preview::Event),
    Env(super::env::Event),
    Mode(super::mode::Event),
    ModelCatalog(crate::app::message::settings::providers::catalog_api::Event),
    Paths(super::paths::Event),
    PiDirectory(crate::app::message::settings::aggregation_pi::Event),
    Open(AppId),
    Close,
    Refresh,
    QueryInput(String),
    Loaded {
        scope: Scope,
        generation: u64,
        result: Result<Snapshot, String>,
    },
    Edit(String),
    HermesMember {
        id: String,
        add: bool,
    },
    HermesMemberChanged {
        scope: Scope,
        generation: u64,
        add: bool,
        result: Result<HermesLiveResult, String>,
    },
    PiMember {
        id: String,
        add: bool,
    },
    PiMemberChanged {
        scope: Scope,
        generation: u64,
        add: bool,
        result: Result<PiLiveResult, String>,
    },
    KimiMember {
        id: String,
        add: bool,
    },
    KimiMemberChanged {
        scope: Scope,
        generation: u64,
        add: bool,
        result: Result<KimiLiveResult, String>,
    },
    McodeMember {
        id: String,
        add: bool,
    },
    DshMember {
        id: String,
        add: bool,
    },
    McodeMemberChanged {
        scope: Scope,
        generation: u64,
        add: bool,
        result: Result<McodeLiveResult, String>,
    },
    DshMemberChanged {
        scope: Scope,
        generation: u64,
        add: bool,
        result: Result<DshLiveResult, String>,
    },
    OmpMember {
        id: String,
        add: bool,
    },
    OmpMemberChanged {
        scope: Scope,
        generation: u64,
        add: bool,
        result: Result<OmpLiveResult, String>,
    },
    OpenClawMember {
        id: String,
        add: bool,
    },
    OpenClawMemberChanged {
        scope: Scope,
        generation: u64,
        add: bool,
        result: Result<OpenClawLiveResult, String>,
    },
    OpenCodeMember {
        id: String,
        add: bool,
    },
    OpenCodeMemberChanged {
        scope: Scope,
        generation: u64,
        add: bool,
        result: Result<OpenCodeLiveResult, String>,
    },
    Sort(String),
    Category(String),
    Notes(String),
    Cancel,
    Save,
    Saved {
        scope: Scope,
        generation: u64,
        result: Result<AppProviderPresentation, String>,
    },
}
pub(crate) fn message(value: ListMessage) -> Message {
    Message::Settings(SettingsMessage::AppProviders(value))
}
pub(crate) fn current(app: &App, id: AppId) -> Result<(GatewayClient, Scope), String> {
    let client = crate::app::config::gateway_client()?;
    let scope = Scope {
        context: crate::app::state::AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: app.project_path.clone(),
        },
        host: app.apps_catalog.host.clone(),
        app: id,
    };
    Ok((client, scope))
}
pub(crate) fn update(app: &mut App, value: ListMessage) -> Task<Message> {
    match value {
        ListMessage::Leave(choice) => return super::leave::choose(app, choice),
        ListMessage::Failover(event) => return crate::app::message::agent_apps_high_availability::update(app, event),
        ListMessage::CodexSubagent(event) => {
            return crate::app::message::settings::aggregation_subagent::update(app, event);
        }
        ListMessage::DesktopAggregation(event) => {
            return crate::app::message::settings::aggregation_desktop::update(app, event);
        }
        ListMessage::OmpDirectory(event) => {
            return crate::app::message::settings::aggregation_omp::update(app, event);
        }
        ListMessage::Omo(event) => {
            return crate::app::message::settings::aggregation_omo::update(app, event);
        }
        ListMessage::Aggregation(event) => {
            return crate::app::message::settings::aggregation_list::update(app, event);
        }
        #[cfg(not(target_arch = "wasm32"))]
        ListMessage::Usage(event) => {
            return crate::app::components::apps_usage_cards::update(app, event);
        }
        #[cfg(not(target_arch = "wasm32"))]
        ListMessage::UsageSettings(event) => {
            return crate::app::components::apps_usage_settings::update(app, event);
        }
        ListMessage::PiDirectory(event) => {
            return crate::app::message::settings::aggregation_pi::update(app, event);
        }
        ListMessage::Icons(event) => return super::icons::update(app, event),
        ListMessage::Universal(event) => return super::universal::update(app, event),
        ListMessage::Env(event) => return super::env::update(app, event),
        ListMessage::Mode(event) => return super::mode::update(app, event),
        ListMessage::ModelCatalog(event) => {
            return crate::app::message::settings::providers::catalog_api::update(app, event);
        }
        ListMessage::Paths(event) => return super::paths::update(app, event),
        ListMessage::Preview(event) => return super::preview::update(app, event),
        ListMessage::Snippets(event) => return super::snippets::update(app, event),
        ListMessage::Endpoints(event) => return super::endpoints::update(app, event),
        ListMessage::EndpointDiagnostics(event) => {
            return crate::app::message::agent_apps_diagnostics::endpoints::update(app, event);
        }
        ListMessage::ModelDiagnostics(event) => {
            return crate::app::message::agent_apps_diagnostics::model::update(app, event);
        }
        ListMessage::Desktop(event) => return super::desktop::update(app, event),
        ListMessage::Close => {
            let generation = app.app_providers.generation.wrapping_add(1);
            app.app_providers = Default::default();
            app.app_providers.generation = generation;
        }
        ListMessage::Open(id) => {
            let (client, scope) = match current(app, id) {
                Ok(v) => v,
                Err(e) => return app.show_error_toast(e),
            };
            if scope.host.is_empty() || app.apps_catalog.token.is_empty() {
                return app.show_error_toast("请先连接应用执行主机");
            }
            // Reloading the same list must not discard an unsaved or in-flight draft.
            // A different host/application still starts a fresh scope below.
            if app.app_providers.scope.as_ref() == Some(&scope)
                && (app.app_providers.form.is_some() || app.app_providers.editor.is_some())
            {
                return Task::none();
            }
            let generation = app.app_providers.begin(scope.clone());
            let token = app.apps_catalog.token.clone();
            return Task::perform(load(client, scope.clone(), token), move |result| {
                message(ListMessage::Loaded { scope: scope.clone(), generation, result })
            });
        }
        ListMessage::QueryInput(query) => {
            app.app_providers.query = query;
        }
        ListMessage::Refresh => {
            if let Some(scope) = app.app_providers.scope.clone() {
                return update(app, ListMessage::Open(scope.app));
            }
        }
        ListMessage::Loaded { scope, generation, result } => {
            if current(app, scope.app).ok().map(|(_, s)| s).as_ref() != Some(&scope) {
                return Task::none();
            }
            if app.app_providers.finish(&scope, generation, result) {
                if let Some(e) = app.app_providers.error.clone() {
                    return app.show_error_toast(e);
                }
                #[cfg(not(target_arch = "wasm32"))]
                return crate::app::components::apps_usage_cards::read_linked_providers(
                    app, scope.app,
                );
            }
        }
        ListMessage::HermesMember { id, add } => {
            return super::hermes::change_membership(app, id, add);
        }
        ListMessage::HermesMemberChanged { scope, generation, add, result } => {
            return super::hermes::membership_changed(app, scope, generation, add, result);
        }
        ListMessage::PiMember { id, add } => {
            return super::pi::change_membership(app, id, add);
        }
        ListMessage::PiMemberChanged { scope, generation, add, result } => {
            return super::pi::membership_changed(app, scope, generation, add, result);
        }
        ListMessage::KimiMember { id, add } => {
            return super::kimi::change_membership(app, id, add);
        }
        ListMessage::KimiMemberChanged { scope, generation, add, result } => {
            return super::kimi::membership_changed(app, scope, generation, add, result);
        }
        ListMessage::McodeMember { id, add } => {
            return super::mcode::change_membership(app, id, add);
        }
        ListMessage::DshMember { id, add } => {
            return super::dsh::change_membership(app, id, add);
        }
        ListMessage::McodeMemberChanged { scope, generation, add, result } => {
            return super::mcode::membership_changed(app, scope, generation, add, result);
        }
        ListMessage::DshMemberChanged { scope, generation, add, result } => {
            return super::dsh::membership_changed(app, scope, generation, add, result);
        }
        ListMessage::OmpMember { id, add } => {
            return super::omp::change_membership(app, id, add);
        }
        ListMessage::OmpMemberChanged { scope, generation, add, result } => {
            return super::omp::membership_changed(app, scope, generation, add, result);
        }
        ListMessage::OpenClawMember { id, add } => {
            return super::openclaw::change_membership(app, id, add);
        }
        ListMessage::OpenClawMemberChanged { scope, generation, add, result } => {
            return super::openclaw::membership_changed(app, scope, generation, add, result);
        }
        ListMessage::OpenCodeMember { id, add } => {
            return super::opencode::change_membership(app, id, add);
        }
        ListMessage::OpenCodeMemberChanged { scope, generation, add, result } => {
            return super::opencode::membership_changed(app, scope, generation, add, result);
        }
        ListMessage::Edit(id) => {
            if app.app_providers.loading || app.app_providers.editor.is_some() {
                return Task::none();
            }
            if !app
                .app_providers
                .snapshot
                .as_ref()
                .is_some_and(|s| s.catalog.rows.iter().any(|r| r.provider.id == id))
            {
                return Task::none();
            }
            let p = app.app_providers.presentation(&id);
            app.app_providers.editor = Some(Editor {
                id,
                sort: p.sort_index.map(|v| v.to_string()).unwrap_or_default(),
                category: p.category.unwrap_or_default(),
                notes: p.notes.unwrap_or_default(),
                saving: false,
            });
        }
        ListMessage::Sort(v) => {
            if let Some(f) = app.app_providers.editor.as_mut().filter(|f| !f.saving) {
                f.sort = v;
            }
        }
        ListMessage::Category(v) => {
            if let Some(f) = app.app_providers.editor.as_mut().filter(|f| !f.saving) {
                f.category = v;
            }
        }
        ListMessage::Notes(v) => {
            if let Some(f) = app.app_providers.editor.as_mut().filter(|f| !f.saving) {
                f.notes = v;
            }
        }
        ListMessage::Cancel => {
            if app.app_providers.editor.as_ref().is_some_and(|f| !f.saving) {
                app.app_providers.editor = None;
            }
        }
        ListMessage::Save => {
            let Some(scope) = app.app_providers.scope.clone() else { return Task::none() };
            let (client, now) = match current(app, scope.app) {
                Ok(v) => v,
                Err(e) => return app.show_error_toast(e),
            };
            if now != scope {
                return update(app, ListMessage::Open(scope.app));
            }
            let Some(form) = app.app_providers.editor.as_mut() else { return Task::none() };
            if form.saving {
                return Task::none();
            }
            let sort_index = if form.sort.trim().is_empty() {
                None
            } else {
                match form.sort.trim().parse::<u32>() {
                    Ok(v) => Some(v),
                    Err(_) => return app.show_error_toast("排序须为非负整数"),
                }
            };
            let value = AppProviderPresentation {
                sort_index,
                category: (!form.category.trim().is_empty())
                    .then(|| form.category.trim().to_string()),
                notes: (!form.notes.trim().is_empty()).then(|| form.notes.trim().to_string()),
            };
            form.saving = true;
            let id = form.id.clone();
            app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
            let generation = app.app_providers.generation;
            return Task::perform(save(client, scope.clone(), id, value), move |result| {
                message(ListMessage::Saved { scope: scope.clone(), generation, result })
            });
        }
        ListMessage::Saved { scope, generation, result } => {
            if app.app_providers.scope.as_ref() != Some(&scope)
                || app.app_providers.generation != generation
                || current(app, scope.app).ok().map(|(_, s)| s).as_ref() != Some(&scope)
            {
                return Task::none();
            }
            match result {
                Ok(_) => {
                    app.app_providers.editor = None;
                    return update(app, ListMessage::Refresh);
                }
                Err(e) => {
                    if let Some(f) = app.app_providers.editor.as_mut() {
                        f.saving = false;
                    }
                    return app.show_error_toast(e);
                }
            }
        }
    }
    Task::none()
}
