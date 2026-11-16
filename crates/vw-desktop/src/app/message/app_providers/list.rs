use crate::app::state::app_providers::list::{Scope, Snapshot};
use vw_api_app_provider_edit::presentation::AppProviderPresentation;
use vw_gateway_client::GatewayClient;

use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;
use vw_api_app_provider_extended::{
    hermes::HermesLiveResult, openclaw::OpenClawLiveResult, opencode::OpenCodeLiveResult,
};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        dsh::DshLiveResult, kimi::KimiLiveResult, mcode::McodeLiveResult, omp::OmpLiveResult,
        pi::PiLiveResult,
    },
};
#[derive(Clone, Debug)]
pub enum ListMessage {
    Leave(super::leave::Choice),
    FailoverHealth(crate::app::message::agent_apps_health::Event),
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
        ListMessage::FailoverHealth(event) => {
            return crate::app::message::agent_apps_health::update(app, event);
        }
        ListMessage::Failover(event) => {
            return crate::app::message::agent_apps_high_availability::update(app, event);
        }
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
            return catalog(app, None, desktop_provider_apps::state::list::CatalogEvent::Close);
        }
        ListMessage::Open(id) => {
            return catalog(app, Some(id), desktop_provider_apps::state::list::CatalogEvent::Open);
        }
        ListMessage::QueryInput(query) => {
            return catalog(
                app,
                None,
                desktop_provider_apps::state::list::CatalogEvent::Query(query),
            );
        }
        ListMessage::Refresh => {
            if let Some(scope) = &app.app_providers.scope {
                return catalog(
                    app,
                    Some(scope.app),
                    desktop_provider_apps::state::list::CatalogEvent::Open,
                );
            }
        }
        ListMessage::Loaded { scope, generation, result } => {
            let id = scope.app;
            return catalog(
                app,
                Some(id),
                desktop_provider_apps::state::list::CatalogEvent::Loaded {
                    scope,
                    generation,
                    result,
                },
            );
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
        ListMessage::Edit(v) => {
            return super::provider_update::list(
                app,
                desktop_provider_apps::message::list::Event::Edit(v),
            );
        }
        ListMessage::Sort(v) => {
            return super::provider_update::list(
                app,
                desktop_provider_apps::message::list::Event::Sort(v),
            );
        }
        ListMessage::Category(v) => {
            return super::provider_update::list(
                app,
                desktop_provider_apps::message::list::Event::Category(v),
            );
        }
        ListMessage::Notes(v) => {
            return super::provider_update::list(
                app,
                desktop_provider_apps::message::list::Event::Notes(v),
            );
        }
        ListMessage::Cancel => {
            return super::provider_update::list(
                app,
                desktop_provider_apps::message::list::Event::Cancel,
            );
        }
        ListMessage::Save => {
            return super::provider_update::list(
                app,
                desktop_provider_apps::message::list::Event::Save,
            );
        }
        ListMessage::Saved { scope, generation, result } => {
            return super::provider_update::list(
                app,
                desktop_provider_apps::message::list::Event::Saved { scope, generation, result },
            );
        }
    }
    Task::none()
}

fn catalog(
    app: &mut App,
    id: Option<AppId>,
    event: desktop_provider_apps::state::list::CatalogEvent,
) -> Task<Message> {
    use desktop_provider_apps::state::list::CatalogEvent;
    let current = id.map(|id| current(app, id)).unwrap_or_else(|| Err("无法连接网关".into()));
    let mut clear = false;
    let mut read = false;
    let mut error = None;
    let task = app
        .app_providers
        .update_catalog(current, &app.apps_catalog.token, event, &mut clear, &mut read, &mut error)
        .map(|event| {
            message(match event {
                CatalogEvent::Loaded { scope, generation, result } => {
                    ListMessage::Loaded { scope, generation, result }
                }
                _ => unreachable!("catalog tasks only return read results"),
            })
        });
    #[cfg(not(target_arch = "wasm32"))]
    if clear {
        app.app_providers.usage.clear();
    }
    let feedback = error.map_or_else(Task::none, |text| app.show_error_toast(text));
    #[cfg(not(target_arch = "wasm32"))]
    let usage = if read {
        app.app_providers.scope.as_ref().map(|s| s.app).map_or_else(Task::none, |id| {
            crate::app::components::apps_usage_cards::read_linked_providers(app, id)
        })
    } else {
        Task::none()
    };
    #[cfg(target_arch = "wasm32")]
    let usage = Task::none();
    Task::batch([task, feedback, usage])
}
