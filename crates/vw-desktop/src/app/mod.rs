//! 汇总应用层模块声明和对外导出，是桌面应用核心模块的入口。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

/// assets 子模块，拆分当前领域的局部职责。
pub mod assets;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod account;
/// components 子模块，拆分当前领域的局部职责。
pub mod components;
/// config 子模块，拆分当前领域的局部职责。
pub mod config;
/// desktop_models 子模块，拆分当前领域的局部职责。
pub mod desktop_models;
/// files 子模块，拆分当前领域的局部职责。
pub mod files;
/// git 子模块，拆分当前领域的局部职责。
pub mod git;
mod impls;
/// lsp 子模块，拆分当前领域的局部职责。
pub mod lsp;
/// message 子模块，拆分当前领域的局部职责。
pub mod message;
/// models 子模块，拆分当前领域的局部职责。
pub(crate) mod models;
/// preview 子模块，拆分当前领域的局部职责。
pub mod preview;
/// projects 子模块，拆分当前领域的局部职责。
pub mod projects;
/// provider 子模块，拆分当前领域的局部职责。
pub mod provider;
/// session 子模块，拆分当前领域的局部职责。
pub mod session;
/// session_gateway 子模块，拆分当前领域的局部职责。
pub mod session_gateway;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod shiju_apps;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code, unused_imports)]
pub(crate) mod skill_hub;
/// state 子模块，拆分当前领域的局部职责。
pub mod state;
/// task 子模块，拆分当前领域的局部职责。
pub mod task;
/// theme_catalog 子模块，拆分当前领域的局部职责。
pub mod theme_catalog;
/// terminal 子模块，拆分当前领域的局部职责。
pub mod terminal;
/// ui 子模块，拆分当前领域的局部职责。
pub mod ui;
mod ui_types;
/// views 子模块，拆分当前领域的局部职责。
pub mod views;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod webview_runtime;
#[cfg(target_arch = "wasm32")]
#[path = "impls/wasm_ime.rs"]
pub(crate) mod wasm_ime;

#[cfg(test)]
pub(crate) static TEST_PROCESS_STATE_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

/// 执行 project_dirs 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn project_dirs() -> Option<directories::ProjectDirs> {
    #[cfg(debug_assertions)]
    const APP_NAME: &str = "vibe-window-dev";
    #[cfg(not(debug_assertions))]
    const APP_NAME: &str = "vibe-window";

    directories::ProjectDirs::from("dev", "VibeWindow", APP_NAME)
}

/// 对外暴露当前模块需要复用的能力。
pub use self::config::{
    gateway_client, gateway_client_endpoint, load_agent_runtime_config, load_agents_ipc_config,
    load_app_config, load_autonomy_config, load_browser_config, load_browser_config_async,
    load_channels_config, load_composio_config, load_coordination_config, load_cron_config,
    load_delegate_agents_config, load_embedding_routes_config, load_full_agent_config,
    load_gateway_client_config, load_gateway_config, load_goal_loop_config, load_heartbeat_config,
    load_hooks_config, load_html_tool_content, load_http_request_config, load_identity_config,
    load_json_tool_content, load_memory_config, load_model_routes_config, load_multimodal_config,
    load_observability_config, load_project_chat_preferences, load_proxy_config,
    load_query_classification_config, load_reliability_config, load_research_config,
    load_runtime_config, load_scheduler_config, load_security_config, load_skills_config,
    load_sop_config, load_sql_tool_content, load_staff_config, load_staff_config_result,
    load_storage_config, load_system_settings_config, load_transcription_config,
    load_tunnel_config, load_web_search_config, save_app_config, save_html_tool_content,
    save_json_tool_content, save_project_chat_preferences, set_config_field,
    update_agent_runtime_config, update_agent_runtime_config_async,
    update_agent_runtime_config_result, update_agents_ipc_config, update_agents_ipc_config_async,
    update_agents_ipc_config_result, update_autonomy_config, update_autonomy_config_async,
    update_autonomy_config_result, update_browser_config, update_browser_config_async,
    update_browser_config_result, update_channels_config, update_channels_config_result,
    update_composio_config, update_composio_config_async, update_composio_config_result,
    update_coordination_config, update_coordination_config_async,
    update_coordination_config_result, update_cron_config, update_cron_config_async,
    update_cron_config_result, update_delegate_agents_config, update_delegate_agents_config_result,
    update_delegate_agents_config_result_async, update_embedding_routes_config,
    update_embedding_routes_config_async, update_embedding_routes_config_result,
    update_gateway_client_config, update_gateway_config, update_gateway_config_async,
    update_gateway_config_result, update_goal_loop_config, update_goal_loop_config_async,
    update_goal_loop_config_result, update_heartbeat_config, update_heartbeat_config_async,
    update_heartbeat_config_result, update_hooks_config, update_hooks_config_async,
    update_hooks_config_result, update_http_request_config, update_http_request_config_async,
    update_http_request_config_result, update_main_agent_overrides_from_delegate_agents,
    update_main_agent_overrides_from_delegate_agents_async, update_memory_config,
    update_memory_config_async, update_memory_config_result, update_model_routes_config,
    update_model_routes_config_async, update_model_routes_config_result, update_multimodal_config,
    update_multimodal_config_async, update_multimodal_config_result, update_observability_config,
    update_observability_config_async, update_observability_config_result, update_proxy_config,
    update_proxy_config_async, update_proxy_config_result, update_query_classification_config,
    update_query_classification_config_async, update_query_classification_config_result,
    update_reliability_config, update_reliability_config_async, update_reliability_config_result,
    update_research_config, update_research_config_async, update_research_config_result,
    update_runtime_config, update_runtime_config_async, update_runtime_config_result,
    update_scheduler_config, update_scheduler_config_async, update_scheduler_config_result,
    update_security_config, update_security_config_async, update_security_config_result,
    update_skills_config, update_skills_config_async, update_skills_config_result,
    update_sop_config, update_sop_config_async, update_sop_config_result,
    update_staff_config_async, update_storage_config, update_storage_config_async,
    update_storage_config_result, update_system_settings_config, update_transcription_config,
    update_transcription_config_result, update_tunnel_config, update_tunnel_config_async,
    update_tunnel_config_result, update_web_search_config, update_web_search_config_async,
    update_web_search_config_result,
};
/// 对外暴露当前模块需要复用的能力。
pub use self::files::{FileIndexLoadResult, load_file_index, refresh_file_index};
/// 对外暴露当前模块需要复用的能力。
pub use self::git::{
    git_discard_file, git_discard_hunk, git_revert_line_delete, git_revert_line_restore,
};
/// 对外暴露当前模块需要复用的能力。
pub use self::message::{after, spawn_blocking_opt};
#[cfg(not(target_arch = "wasm32"))]
/// 对外暴露当前模块需要复用的能力。
pub(crate) use self::preview::{
    LspHoverPending, LspProgress, path_to_file_uri,
};
/// 对外暴露当前模块需要复用的能力。
pub use self::preview::{PreviewTab, preview_open_error, safe_preview};
/// 对外暴露当前模块需要复用的能力。
pub use self::projects::{
    load_recent_projects, load_recent_projects_meta, push_recent_project, save_recent_projects,
    save_recent_projects_background, save_recent_projects_meta,
    save_recent_projects_meta_background,
};
/// 对外暴露当前模块需要复用的能力。
pub(crate) use self::state::{AgentRequest, QueueItem};
/// 对外暴露当前模块需要复用的能力。
pub use self::state::{App, AppTab, CookieConfig, RecentProjectMeta, WebBookmark};
/// 对外暴露当前模块需要复用的能力。
pub use self::terminal::{Shell, TerminalState, TerminalTheme, build_palette};
/// 对外暴露当前模块需要复用的能力。
pub(crate) use self::ui_types::FocusArea;
/// 对外暴露当前模块需要复用的能力。
pub use self::ui_types::{DiffTheme, Message, Screen, SettingsTab, TodoPanelPlacement};
/// 对外暴露当前模块需要复用的能力。
pub use vw_config_types::ui::PreviewAutoSaveMode;
/// 对外暴露当前模块需要复用的能力。
pub use vw_shared::time;

#[cfg(test)]
mod tests;
