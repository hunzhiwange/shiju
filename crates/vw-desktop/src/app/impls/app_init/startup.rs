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
            crate::app::message::account::load_repository(),
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

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) use native::StartupConfiguration;
#[cfg(not(target_arch = "wasm32"))]
pub use desktop_settings_network::startup::StartupGatewaySnapshot;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;

    /// Adapt domain tasks and endpoint checks to the desktop's global messages.
    #[derive(Default)]
    pub(crate) struct StartupConfiguration(desktop_settings_network::startup::StartupConfiguration);

    impl std::ops::Deref for StartupConfiguration {
        type Target = desktop_settings_network::startup::StartupConfiguration;
        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl std::ops::DerefMut for StartupConfiguration {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl StartupConfiguration {
        pub(crate) fn load(&mut self) -> Task<Message> {
            self.0.load(config::gateway_client_endpoint()).map(|(generation, snapshot)| {
                Message::StartupGatewaySnapshotLoaded(generation, snapshot)
            })
        }

        pub(crate) fn accept(&mut self, generation: u64, snapshot: &StartupGatewaySnapshot) -> bool {
            self.0.accept(generation, snapshot, &config::gateway_client_endpoint())
        }

        pub(crate) fn accept_sessions(&mut self, generation: u64, succeeded: bool) -> bool {
            self.0.accept_sessions(generation, succeeded, &config::gateway_client_endpoint())
        }
    }

    impl App {
        pub(crate) fn note_startup_setting_edit(&mut self, field: &'static str) {
            let scope = config::preferences_scope(&config::gateway_client_endpoint());
            self.startup_configuration.note_startup_setting_edit(scope, field);
        }

        pub(crate) fn note_startup_worktree_edit(&mut self, path: &str) {
            let scope = config::preferences_scope(&config::gateway_client_endpoint());
            self.startup_configuration.note_startup_worktree_edit(scope, path);
        }

        pub(crate) fn merge_startup_worktree_settings(
            &mut self,
            mut snapshot: HashMap<String, bool>,
        ) {
            let scope = config::preferences_scope(&config::gateway_client_endpoint());
            self.startup_configuration.merge_worktrees(&scope, &mut snapshot, &self.project_worktree_enabled);
            if !self.project_editor.project_edit_worktree_dirty
                && let Some(path) = self.project_editor.project_edit_path.as_ref()
            {
                self.project_editor.project_edit_worktree_enabled = snapshot.get(path).copied().unwrap_or(false);
            }
            self.project_worktree_enabled = snapshot;
        }
    }
}
