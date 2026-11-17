//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

mod acp;
pub(in crate::app) mod agents;
pub(crate) mod aggregation_list;
pub(crate) mod aggregation_omo;
pub(crate) mod aggregation_pi;
pub(crate) mod apps_catalog;
pub(crate) mod apps_lifecycle;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod apps_usage_events;
pub(crate) mod archived_chats;
mod autonomy;
mod browser;
mod channels;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod computer_control;
mod cron;
pub(crate) mod cron_history;
pub(crate) mod backup_inventory;
mod dialogue_flow;
mod embedding_routes;
mod gateway;
mod gateway_client;
mod general;
mod heartbeat;
mod hooks;
mod http_request;
mod mcp;
pub(crate) mod mcp_management_bindings;
pub(crate) mod mcp_management_form;
pub(crate) mod mcp_management_import;
pub(crate) mod mcp_management_list;
pub(crate) mod mcp_management_probe;
pub(crate) mod mcp_management_resync;
mod memory;
mod messages;
mod model_routes;
mod models;
mod multimodal;
mod observability;
pub(crate) mod plugin_install;
mod projects;
pub(crate) mod providers;
mod proxy;
mod query_classification;
mod quick_messages;
mod reliability;
pub(crate) mod routing_persistence;
mod runtime;
mod scheduler;
mod security;
mod sessions;
pub(crate) mod skills;
mod storage;
mod tabs;
mod themes;
mod tunnel;
mod types;
mod util;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod voice;
mod web_search;

use crate::app::config::server_config_unreachable_error;
use crate::app::{App, Message};
pub use apps_catalog::AppsCatalogMessage;
use iced::Task;

pub(crate) use messages::{
    AcpMessage, AgentConfigTabMessage, AgentsMessage, ChannelsMessage, GatewayClientMessage,
    GeneralSettingsMessage, McpMessage,
    ModelConfigTabMessage, ModelRoutesMessage,
    QuickMessagesMessage, SettingsMessage, ThemesMessage,
};

pub(crate) use projects::set_worktree_enabled;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use projects::wait_for_worktree_saves;

#[cfg(target_arch = "wasm32")]
pub(crate) fn reload_agents_config_task(project_path: Option<String>) -> Task<Message> {
    agents::config_task(project_path)
}

fn apply_agent_config_saved(app: &mut App, tag: &'static str, result: Result<(), String>) {
    let raw_error = result.err();
    let save_error = raw_error.as_ref().map(server_config_unreachable_error);

    match tag {
        "heartbeat" => desktop_settings_cron::save_feedback::heartbeat_saved(
            &mut app.heartbeat_settings,
            save_error,
        ),
        "cron" => {
            desktop_settings_cron::save_feedback::cron_saved(&mut app.cron_settings, save_error)
        }
        "scheduler" => desktop_settings_cron::save_feedback::scheduler_saved(
            &mut app.scheduler_settings,
            save_error,
        ),
        "reliability" => desktop_settings_runtime::save_feedback::reliability_saved(
            &mut app.reliability_settings,
            save_error,
        ),
        "memory" => desktop_settings_memory::save_feedback::memory_saved(
            &mut app.memory_settings,
            save_error,
        ),
        "security" => desktop_settings_security::save_feedback::security_saved(
            &mut app.security_settings,
            save_error,
        ),
        "channels_config" => desktop_settings_channels::save_feedback::channels_config_saved(
            &mut app.channels_settings,
            save_error,
        ),
        "observability" => desktop_settings_runtime::save_feedback::observability_saved(
            &mut app.observability_settings,
            save_error,
        ),
        "storage" => desktop_settings_memory::save_feedback::storage_saved(
            &mut app.storage_settings,
            save_error,
        ),
        "proxy" => desktop_settings_network::save_feedback::proxy_saved(
            &mut app.proxy_settings,
            save_error,
        ),
        "browser" => desktop_settings_network::save_feedback::browser_saved(
            &mut app.browser_settings,
            save_error,
        ),
        "http_request" => desktop_settings_network::save_feedback::http_request_saved(
            &mut app.http_request_settings,
            save_error,
        ),
        "multimodal" => desktop_settings_runtime::save_feedback::multimodal_saved(
            &mut app.multimodal_settings,
            save_error,
        ),
        "web_search" => desktop_settings_network::save_feedback::web_search_saved(
            &mut app.web_search_settings,
            save_error,
        ),
        "tunnel" => desktop_settings_network::save_feedback::tunnel_saved(
            &mut app.tunnel_settings,
            save_error,
        ),
        "hooks" => desktop_settings_runtime::save_feedback::hooks_saved(
            &mut app.hooks_settings,
            save_error,
        ),
        "skills" => desktop_settings_skills::save_feedback::skills_saved(
            &mut app.skills_settings,
            save_error,
        ),
        "runtime" => desktop_settings_runtime::save_feedback::runtime_saved(
            &mut app.runtime_settings,
            save_error,
        ),
        "model_routes" => desktop_settings_providers::save_feedback::model_routes_saved(
            &mut app.model_routes_settings.save_error,
            raw_error,
        ),
        "embedding_routes" => desktop_settings_providers::save_feedback::embedding_routes_saved(
            &mut app.embedding_routes_settings,
            raw_error,
        ),
        "gateway" => desktop_settings_network::save_feedback::gateway_saved(
            &mut app.gateway_settings,
            raw_error,
        ),
        "query_classification" => {
            desktop_settings_runtime::save_feedback::query_classification_saved(
                &mut app.query_classification_settings,
                save_error,
            )
        }
        "autonomy" => desktop_settings_security::save_feedback::autonomy_saved(
            &mut app.autonomy_settings,
            save_error,
        ),
        _ => {}
    }
}

// These panels keep their form values in App state while the user edits them.
fn update_config_panel(
    app: &mut App,
    message: SettingsMessage,
    _field: &'static str,
    update: fn(&mut App, SettingsMessage) -> Task<Message>,
) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    app.note_startup_setting_edit(_field);
    update(app, message)
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    use desktop_settings_shell::messages::SettingsRoute;
    match message.route() {
        SettingsRoute::DeviceSyncWebdav(event) => device_sync_webdav::update(app, event),
        SettingsRoute::DeviceSyncS3(event) => device_sync_s3::update(app, event),
        SettingsRoute::MigrationPreview(event) => {
            crate::app::components::system_settings_data::migration_preview_ui::update(app, event)
        }
        SettingsRoute::DataBackups(event) => data_management::backup_list_ui::update(app, event),
        SettingsRoute::DataRecovery(event) => data_management::recovery_ui::update(app, event),
        SettingsRoute::DataExport(event) => data_management::export_ui::update(app, event),
        SettingsRoute::BackupInventory(event) => backup_inventory::update(app, event),
        #[cfg(not(target_arch = "wasm32"))]
        SettingsRoute::ComputerControl(event) => computer_control::update(app, event),
        #[cfg(not(target_arch = "wasm32"))]
        SettingsRoute::Voice(event) => voice::update(app, event),
        SettingsRoute::ArchivedChats(message) => archived_chats::update(app, message),
        SettingsRoute::Worktrees(message) => worktrees::update(app, message),
        SettingsRoute::AppsCatalog(message) => apps_catalog::update(app, message),
        #[cfg(not(target_arch = "wasm32"))]
        SettingsRoute::AgentAppsAuth(event) => {
            crate::app::message::agent_apps_auth::update(app, event)
        }
        SettingsRoute::AppProviderEffect(effect) => {
            crate::app::message::app_providers::provider_update::effect(app, effect)
        }
        SettingsRoute::AppProviderEdit(message) => {
            crate::app::message::app_providers::edit::update(app, message)
        }
        SettingsRoute::AppProviders(message) => {
            crate::app::message::app_providers::list::update(app, message)
        }
        SettingsRoute::TabSelected(message) => tabs::update(app, message),
        SettingsRoute::General(message) => general::update(app, message),
        SettingsRoute::EmbeddingRoutes(message) => {
            update_config_panel(app, message, "embedding_routes_settings", embedding_routes::update)
        }
        SettingsRoute::Hooks(message) => {
            update_config_panel(app, message, "hooks_settings", hooks::update)
        }
        SettingsRoute::Storage(message) => {
            update_config_panel(app, message, "storage_settings", storage::update)
        }
        SettingsRoute::ModelRoutes(message) => {
            update_config_panel(app, message, "model_routes_settings", model_routes::update)
        }
        SettingsRoute::QueryClassification(message) => update_config_panel(
            app,
            message,
            "query_classification_settings",
            query_classification::update,
        ),
        SettingsRoute::Runtime(message) => {
            update_config_panel(app, message, "runtime_settings", runtime::update)
        }
        SettingsRoute::Browser(message) => {
            update_config_panel(app, message, "browser_settings", browser::update)
        }
        SettingsRoute::WebSearch(message) => {
            update_config_panel(app, message, "web_search_settings", web_search::update)
        }
        SettingsRoute::HttpRequest(message) => {
            update_config_panel(app, message, "http_request_settings", http_request::update)
        }
        SettingsRoute::Memory(message) => {
            update_config_panel(app, message, "memory_settings", memory::update)
        }
        SettingsRoute::Channels(message) => {
            update_config_panel(app, message, "channels_settings", channels::update)
        }
        SettingsRoute::Multimodal(message) => {
            update_config_panel(app, message, "multimodal_settings", multimodal::update)
        }
        SettingsRoute::Acp(message) => acp::update(app, message),
        SettingsRoute::Mcp(message) => mcp::update(app, message),
        SettingsRoute::GatewayClient(message) => gateway_client::update(app, message),
        SettingsRoute::Gateway(message) => {
            update_config_panel(app, message, "gateway_settings", gateway::update)
        }
        SettingsRoute::Tunnel(message) => {
            update_config_panel(app, message, "tunnel_settings", tunnel::update)
        }
        SettingsRoute::Themes(message) => themes::update(app, message),
        SettingsRoute::Agents(message) => agents::update(app, message),
        SettingsRoute::QuickMessages(message) => quick_messages::update(app, message),
        SettingsRoute::AgentConfigSaved { tag, result } => {
            if tag == "agent" {
                agents::handle_agents_config_saved(app, result)
            } else {
                apply_agent_config_saved(app, tag, result);
                Task::none()
            }
        }
        SettingsRoute::ProvidersRefresh(message) => providers::update(app, message),
        SettingsRoute::ModelsRefresh(message) => models::update(app, message),
        SettingsRoute::PluginInstallEffect(message) => plugin_install::update(app, message),
        SettingsRoute::SkillsDirectoryProviderChanged(message) => {
            update_config_panel(app, message, "skills_settings", skills::update)
        }
        SettingsRoute::SkillsProjectSelected(message) => skills::update(app, message),
        SettingsRoute::DialogueFlowPermissionRefresh(message) => {
            dialogue_flow::update(app, message)
        }
        SettingsRoute::RecentProjectQueryChanged(message) => projects::update(app, message),
        SettingsRoute::SessionDelete(message) => sessions::update(app, message),
        SettingsRoute::HeartbeatEnabledToggled(message) => {
            update_config_panel(app, message, "heartbeat_settings", heartbeat::update)
        }
        SettingsRoute::CronHistory(event) => cron_history::update(app, event),
        SettingsRoute::CronFullAccessConfirm(message) => {
            update_config_panel(app, message, "cron_settings", cron::update)
        }
        SettingsRoute::SchedulerEnabledToggled(message) => {
            update_config_panel(app, message, "scheduler_settings", scheduler::update)
        }
        SettingsRoute::ReliabilityProviderRetriesChanged(message) => {
            update_config_panel(app, message, "reliability_settings", reliability::update)
        }
        SettingsRoute::SecurityBashSecurityEnabledToggled(message) => {
            update_config_panel(app, message, "security_settings", security::update)
        }
        SettingsRoute::AutonomyTabSelected(message) => {
            update_config_panel(app, message, "autonomy_settings", autonomy::update)
        }
        SettingsRoute::ObservabilityBackendChanged(message) => {
            update_config_panel(app, message, "observability_settings", observability::update)
        }
        SettingsRoute::ProxyEnabledToggled(message) => {
            update_config_panel(app, message, "proxy_settings", proxy::update)
        }
    }
}

pub(crate) mod aggregation_omp;

pub(crate) mod aggregation_desktop;

pub(crate) mod aggregation_subagent;

#[cfg(test)]
mod mcp_management_resync_tests;

#[cfg(test)]
mod mcp_management_tools_tests;

pub(crate) mod mcp_management_tools;

pub(crate) mod worktrees;

pub(crate) mod prompts;

pub(crate) mod data_management;

pub(crate) mod device_sync_preview;

pub(crate) mod device_sync_s3;
pub(crate) mod device_sync_webdav;

#[cfg(test)]
pub(crate) use cron::validate_delivery as validate_cron_delivery;
#[cfg(test)]
pub(crate) use messages::BashPolicyCheck;
#[cfg(test)]
pub(crate) use messages::BashValidatorCheck;
#[cfg(test)]
pub(crate) use messages::BrowserMessage;
#[cfg(test)]
pub(crate) use messages::EmbeddingRoutesMessage;
#[cfg(test)]
pub(crate) use messages::GatewayMessage;
#[cfg(test)]
pub(crate) use messages::HooksMessage;
#[cfg(test)]
pub(crate) use messages::HttpRequestMessage;
#[cfg(test)]
pub(crate) use messages::MemoryMessage;
#[cfg(test)]
pub(crate) use messages::MultimodalMessage;
#[cfg(test)]
pub(crate) use messages::QueryClassificationMessage;
#[cfg(test)]
pub(crate) use messages::RuntimeMessage;
#[cfg(test)]
pub(crate) use messages::StorageMessage;
#[cfg(test)]
pub(crate) use messages::ThemeChromeField;
#[cfg(test)]
pub(crate) use messages::ThemeColorPickerTarget;
#[cfg(test)]
pub(crate) use messages::ThemePaletteField;
#[cfg(test)]
pub(crate) use messages::TunnelMessage;
#[cfg(test)]
pub(crate) use messages::WebSearchMessage;
