//! 组织桌面应用初始化阶段的 startup.rs 逻辑。
//! 本模块把启动输入、配置加载和初始状态装配拆开，便于定位启动失败路径。

use iced::Task;

use super::*;

/// 模块内可见函数，执行 build_startup_task 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_startup_task(app: &mut App) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = app;
        Task::perform(
            crate::app::config::bootstrap_cli_service_async(),
            Message::StartupCliServiceBootstrapped,
        )
    }

    #[cfg(target_arch = "wasm32")]
    {
        Task::batch(vec![
            Task::done(Message::GatewayHealthTick),
            Task::perform(crate::app::config::load_app_config_async(), Message::BootstrapAppConfig),
            Task::perform(
                crate::app::config::load_system_settings_config_async(),
                Message::BootstrapSystemSettings,
            ),
            Task::perform(
                crate::app::config::load_browser_config_async(),
                Message::BootstrapBrowserConfig,
            ),
            Task::perform(
                async {
                    crate::app::config::load_enabled_acp_config_async()
                        .await
                        .map(|cfg| sort_acp_agents(&cfg))
                },
                Message::BootstrapAcpAgentsLoaded,
            ),
            Task::perform(
                crate::app::session_gateway::gateway_load_archived_session_ids_async(None),
                Message::BootstrapArchivedSessions,
            ),
            Task::perform(
                crate::app::message::project::helpers::load_gateway_recent_projects(),
                |result| {
                    Message::Project(crate::app::message::ProjectMessage::RecentProjectsLoaded(
                        result,
                    ))
                },
            ),
            Task::perform(
                crate::app::session_gateway::gateway_external_apps_async(),
                Message::ExternalAppsLoaded,
            ),
            app.reload_sessions_for_project(None),
        ])
    }
}
#[cfg(test)]
#[path = "startup_tests.rs"]
mod startup_tests;
