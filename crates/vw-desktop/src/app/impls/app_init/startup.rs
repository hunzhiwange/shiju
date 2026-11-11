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
pub use native::StartupGatewaySnapshot;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use serde_json::Value;
    use std::collections::HashSet;
    use vw_config_types::{config::Config, gateway::GatewayConfig, ui::AppSystemSettingsConfig};
    use vw_gateway_client::{ExternalAppsStateDto, GatewayClient, GatewayEndpoint};

    #[derive(Default, Clone, Copy)]
    struct Loaded {
        preferences: bool,
        system: bool,
        agent: bool,
        catalog: bool,
        tools: bool,
        external_apps: bool,
    }

    #[derive(Clone)]
    pub(in crate::app) struct StartupAgentSnapshot {
        pub config: Config,
        pub gateway: GatewayConfig,
        pub acp_agents: Vec<String>,
    }

    #[derive(Clone)]
    pub struct StartupGatewaySnapshot {
        pub(in crate::app) preferences: Option<Result<Value, String>>,
        pub(in crate::app) system: Option<Result<AppSystemSettingsConfig, String>>,
        pub(in crate::app) agent: Option<Result<StartupAgentSnapshot, String>>,
        pub(in crate::app) catalog: Option<Result<Box<Config>, String>>,
        pub(in crate::app) tools: Option<Result<Vec<String>, String>>,
        pub(in crate::app) external_apps: Option<Result<ExternalAppsStateDto, String>>,
    }

    impl std::fmt::Debug for StartupGatewaySnapshot {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("StartupGatewaySnapshot").finish_non_exhaustive()
        }
    }

    #[derive(Default)]
    pub(crate) struct StartupConfiguration {
        endpoint: Option<GatewayEndpoint>,
        generation: u64,
        pending: bool,
        sessions_pending: Option<u64>,
        sessions_loaded: bool,
        loaded: Loaded,
        edited_settings: HashMap<config::PreferencesScope, HashSet<&'static str>>,
        edited_worktrees: HashMap<config::PreferencesScope, HashSet<String>>,
        pub(crate) pending_acp: Option<String>,
    }

    impl App {
        pub(crate) fn note_startup_setting_edit(&mut self, field: &'static str) {
            let scope = config::preferences_scope(&config::gateway_client_endpoint());
            self.startup_configuration.edited_settings.entry(scope).or_default().insert(field);
        }

        pub(crate) fn note_startup_worktree_edit(&mut self, path: &str) {
            let scope = config::preferences_scope(&config::gateway_client_endpoint());
            self.startup_configuration
                .edited_worktrees
                .entry(scope)
                .or_default()
                .insert(path.to_owned());
        }

        pub(crate) fn merge_startup_worktree_settings(
            &mut self,
            mut snapshot: HashMap<String, bool>,
        ) {
            let scope = config::preferences_scope(&config::gateway_client_endpoint());
            if let Some(paths) = self.startup_configuration.edited_worktrees.get(&scope) {
                for path in paths {
                    match self.project_worktree_enabled.get(path) {
                        Some(enabled) => {
                            snapshot.insert(path.clone(), *enabled);
                        }
                        None => {
                            snapshot.remove(path);
                        }
                    }
                }
            }
            if !self.project_edit_worktree_dirty
                && let Some(path) = self.project_edit_path.as_ref()
            {
                self.project_edit_worktree_enabled = snapshot.get(path).copied().unwrap_or(false);
            }
            self.project_worktree_enabled = snapshot;
        }
    }

    fn same_endpoint(left: &GatewayEndpoint, right: &GatewayEndpoint) -> bool {
        config::preferences_scope(left) == config::preferences_scope(right)
            && left.auth.as_ref().and_then(|auth| auth.skey.as_ref())
                == right.auth.as_ref().and_then(|auth| auth.skey.as_ref())
    }

    impl StartupConfiguration {
        pub(crate) fn select_endpoint(&mut self, endpoint: &GatewayEndpoint) {
            if self.endpoint.as_ref().is_none_or(|previous| !same_endpoint(previous, endpoint)) {
                self.endpoint = Some(endpoint.clone());
                self.pending = false;
                self.sessions_pending = None;
                self.loaded = Loaded::default();
                self.pending_acp = None;
            }
        }

        pub(crate) fn load(&mut self) -> Task<Message> {
            let endpoint = config::gateway_client_endpoint();
            self.select_endpoint(&endpoint);
            let loaded = self.loaded;
            if self.pending
                || (loaded.preferences
                    && loaded.system
                    && loaded.agent
                    && loaded.catalog
                    && loaded.tools
                    && loaded.external_apps)
            {
                return Task::none();
            }
            self.pending = true;
            self.generation += 1;
            let generation = self.generation;
            Task::perform(load_snapshot(endpoint, loaded), move |snapshot| {
                Message::StartupGatewaySnapshotLoaded(generation, Box::new(snapshot))
            })
        }

        pub(crate) fn begin_sessions(&mut self) -> Option<u64> {
            if self.sessions_loaded || self.sessions_pending.is_some() {
                return None;
            }
            let generation = self.generation;
            self.sessions_pending = Some(generation);
            Some(generation)
        }

        pub(crate) fn accept_sessions(&mut self, generation: u64, succeeded: bool) -> bool {
            if self.sessions_pending != Some(generation) {
                return false;
            }
            self.sessions_pending = None;
            if !self.is_current_endpoint() {
                return false;
            }
            self.sessions_loaded = succeeded;
            true
        }

        fn is_current_endpoint(&self) -> bool {
            let current = config::gateway_client_endpoint();
            self.endpoint.as_ref().is_some_and(|endpoint| same_endpoint(endpoint, &current))
        }

        pub(crate) fn setting_edited(&self, scope: &config::PreferencesScope, field: &str) -> bool {
            self.edited_settings.get(scope).is_some_and(|fields| fields.contains(field))
        }

        pub(crate) fn agent_loaded(&self) -> bool {
            self.loaded.agent
        }

        pub(crate) fn accept(
            &mut self,
            generation: u64,
            snapshot: &StartupGatewaySnapshot,
        ) -> bool {
            if !self.pending || self.generation != generation {
                return false;
            }
            self.pending = false;
            if !self.is_current_endpoint() {
                return false;
            }
            self.loaded.preferences |= snapshot.preferences.as_ref().is_some_and(Result::is_ok);
            self.loaded.system |= snapshot.system.as_ref().is_some_and(Result::is_ok);
            self.loaded.agent |= snapshot.agent.as_ref().is_some_and(Result::is_ok);
            self.loaded.catalog |= snapshot.catalog.as_ref().is_some_and(Result::is_ok);
            self.loaded.tools |= snapshot.tools.as_ref().is_some_and(Result::is_ok);
            self.loaded.external_apps |= snapshot.external_apps.as_ref().is_some_and(Result::is_ok);
            true
        }
    }

    async fn load_snapshot(endpoint: GatewayEndpoint, loaded: Loaded) -> StartupGatewaySnapshot {
        let client = GatewayClient::new(endpoint);
        let preferences = async {
            if loaded.preferences {
                return None;
            }
            Some(match &client {
                Ok(client) => client.desktop_preferences_get().await,
                Err(error) => Err(error.clone()),
            })
        };
        let configuration = async {
            if loaded.system && loaded.agent && loaded.catalog {
                return (None, None, None);
            }
            let global = match &client {
                Ok(client) => client.global_config_get().await,
                Err(error) => Err(error.clone()),
            };
            let system = (!loaded.system).then(|| {
                let global = global.as_ref().map_err(Clone::clone)?;
                let mut settings = global.pointer("/app_ui/system_settings").cloned();
                if let Some(fields) = settings.as_mut().and_then(Value::as_object_mut) {
                    // The desktop connects using its local bootstrap file.
                    fields.remove("gateway_client");
                }
                let system = settings
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(|error| error.to_string())?
                    .unwrap_or_default();
                Ok(config::normalize_system_settings_config(system))
            });
            let global_config = || {
                let global = global.as_ref().map_err(Clone::clone)?;
                serde_json::from_value::<Config>(global.clone()).map_err(|error| error.to_string())
            };
            let agent = async {
                if loaded.agent {
                    return None;
                }
                Some(match (client.as_ref(), global_config()) {
                    (Ok(client), Ok(global)) => load_agent_snapshot(client, global).await,
                    (_, Err(error)) => Err(error),
                    (Err(error), _) => Err(error.clone()),
                })
            };
            let catalog = async {
                if loaded.catalog {
                    return None;
                }
                Some(
                    async {
                        let mut global = global_config()?;
                        let profiles = client
                            .as_ref()
                            .map_err(Clone::clone)?
                            .agent_profiles(None, true)
                            .await?;
                        crate::app::message::settings::agents::merge_catalog_profiles(
                            &mut global,
                            profiles,
                        )?;
                        Ok(Box::new(global))
                    }
                    .await,
                )
            };
            let (agent, catalog) = tokio::join!(agent, catalog);
            (system, agent, catalog)
        };
        let tools = async {
            if loaded.tools {
                return None;
            }
            Some(match &client {
                Ok(client) => client.tools_list().await,
                Err(error) => Err(error.clone()),
            })
        };
        let external_apps = async {
            if loaded.external_apps {
                return None;
            }
            Some(match &client {
                Ok(client) => client.desktop_external_apps_get().await,
                Err(error) => Err(error.clone()),
            })
        };
        let (preferences, (system, agent, catalog), tools, external_apps) =
            tokio::join!(preferences, configuration, tools, external_apps);
        StartupGatewaySnapshot { preferences, system, agent, catalog, tools, external_apps }
    }

    async fn load_agent_snapshot(
        client: &GatewayClient,
        global: Config,
    ) -> Result<StartupAgentSnapshot, String> {
        // The global API encrypts secrets. Keep the runtime config API for Agent settings,
        // and use the global Gateway subtree as the Gateway settings loader did.
        let (runtime, acp_catalog) =
            tokio::join!(client.config_get(None), client.global_acp_config_get());
        let mut runtime: Config =
            serde_json::from_value(runtime?).map_err(|err| err.to_string())?;
        config::apply_main_agent_overrides(&mut runtime);
        let acp = match acp_catalog {
            Ok(available) => config::resolve_enabled_acp_config(&available, global.acp),
            Err(_) => runtime.acp.clone(),
        };
        Ok(StartupAgentSnapshot {
            config: runtime,
            gateway: global.gateway,
            acp_agents: sort_acp_agents(&acp),
        })
    }
}
