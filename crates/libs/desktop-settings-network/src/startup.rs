//! Gateway startup snapshots and endpoint-scoped retry state.
use iced::Task;
use std::collections::HashMap;
use desktop_config::config_desktop::{preferences_scope, PreferencesScope};
use desktop_config as config;
use serde_json::Value;
use std::collections::HashSet;
use config_aggregate::Config;
use vw_config_types::{gateway::GatewayConfig, ui::AppSystemSettingsConfig};
use {gateway_endpoint::{GatewayEndpoint}, gateway_desktop_settings::ExternalAppsStateDto, vw_gateway_client::GatewayClient};

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
pub struct StartupAgentSnapshot {
    pub config: Config,
    pub gateway: GatewayConfig,
    pub acp_agents: Vec<String>,
}

#[derive(Clone)]
pub struct StartupGatewaySnapshot {
    pub preferences: Option<Result<Value, String>>,
    pub system: Option<Result<AppSystemSettingsConfig, String>>,
    pub agent: Option<Result<StartupAgentSnapshot, String>>,
    pub catalog: Option<Result<Box<Config>, String>>,
    pub tools: Option<Result<Vec<String>, String>>,
    pub external_apps: Option<Result<ExternalAppsStateDto, String>>,
}

impl std::fmt::Debug for StartupGatewaySnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StartupGatewaySnapshot").finish_non_exhaustive()
    }
}

#[derive(Default)]
pub struct StartupConfiguration {
    endpoint: Option<GatewayEndpoint>,
    generation: u64,
    pending: bool,
    sessions_pending: Option<u64>,
    sessions_loaded: bool,
    loaded: Loaded,
    edited_settings: HashMap<PreferencesScope, HashSet<&'static str>>,
    edited_worktrees: HashMap<PreferencesScope, HashSet<String>>,
    pub pending_acp: Option<String>,
}

fn same_endpoint(left: &GatewayEndpoint, right: &GatewayEndpoint) -> bool {
    preferences_scope(left) == preferences_scope(right)
        && left.auth.as_ref().and_then(|auth| auth.skey.as_ref())
            == right.auth.as_ref().and_then(|auth| auth.skey.as_ref())
}

impl StartupConfiguration {
    pub fn select_endpoint(&mut self, endpoint: &GatewayEndpoint) {
        if self.endpoint.as_ref().is_none_or(|previous| !same_endpoint(previous, endpoint)) {
            self.endpoint = Some(endpoint.clone());
            self.pending = false;
            self.sessions_pending = None;
            self.loaded = Loaded::default();
            self.pending_acp = None;
        }
    }

    pub fn load(&mut self, endpoint: GatewayEndpoint) -> Task<(u64, Box<StartupGatewaySnapshot>)> {
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
            (generation, Box::new(snapshot))
        })
    }

    pub fn begin_sessions(&mut self) -> Option<u64> {
        if self.sessions_loaded || self.sessions_pending.is_some() {
            return None;
        }
        let generation = self.generation;
        self.sessions_pending = Some(generation);
        Some(generation)
    }

    pub fn accept_sessions(&mut self, generation: u64, succeeded: bool, current: &GatewayEndpoint) -> bool {
        if self.sessions_pending != Some(generation) {
            return false;
        }
        self.sessions_pending = None;
        if !self.is_current_endpoint(current) {
            return false;
        }
        self.sessions_loaded = succeeded;
        true
    }

    fn is_current_endpoint(&self, current: &GatewayEndpoint) -> bool {
        self.endpoint.as_ref().is_some_and(|endpoint| same_endpoint(endpoint, current))
    }

    pub fn setting_edited(&self, scope: &PreferencesScope, field: &str) -> bool {
        self.edited_settings.get(scope).is_some_and(|fields| fields.contains(field))
    }

    pub fn agent_loaded(&self) -> bool {
        self.loaded.agent
    }

    pub fn accept(
        &mut self,
        generation: u64,
        snapshot: &StartupGatewaySnapshot,
        current: &GatewayEndpoint,
    ) -> bool {
        if !self.pending || self.generation != generation {
            return false;
        }
        self.pending = false;
        if !self.is_current_endpoint(current) {
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
            Ok(client) => gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_preferences_get().await,
            Err(error) => Err(error.clone()),
        })
    };
    let configuration = async {
        if loaded.system && loaded.agent && loaded.catalog {
            return (None, None, None);
        }
        let global = match &client {
            Ok(client) => gateway_config::GatewayConfigClient::new(client.transport().clone()).global_config_get().await,
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
            Ok(config::config_system_settings::normalize_system_settings_config(system))
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
                    let transport = client.as_ref().map_err(Clone::clone)?.transport().clone();
                    let profiles = gateway_config::GatewayConfigClient::new(transport)
                        .agent_profiles(None, true)
                        .await?;
                    desktop_settings_agents::agents_update::merge_catalog_profiles(
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
            Ok(client) => gateway_interaction::GatewayInteractionClient::new(client.transport().clone()).tools_list().await,
            Err(error) => Err(error.clone()),
        })
    };
    let external_apps = async {
        if loaded.external_apps {
            return None;
        }
        Some(match &client {
            Ok(client) => gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_external_apps_get().await,
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
    let config_client = gateway_config::GatewayConfigClient::new(client.transport().clone());
    let (runtime, acp_catalog) =
        tokio::join!(config_client.config_get(None), config_client.global_acp_config_get());
    let mut runtime: Config =
        serde_json::from_value(runtime?).map_err(|err| err.to_string())?;
    config::access::apply_main_agent_overrides(&mut runtime);
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

fn sort_acp_agents(config: &HashMap<String, config_aggregate::AcpAgentConfig>) -> Vec<String> {
    let mut agents = config.keys().cloned().collect::<Vec<_>>();
    agents.sort_by(|left, right| (left != "codex", left.as_str()).cmp(&(right != "codex", right.as_str())));
    agents
}

impl StartupConfiguration {
    pub fn note_startup_setting_edit(&mut self, scope: PreferencesScope, field: &'static str) {
        self.edited_settings.entry(scope).or_default().insert(field);
    }

    pub fn note_startup_worktree_edit(&mut self, scope: PreferencesScope, path: &str) {
        self
            .edited_worktrees
            .entry(scope)
            .or_default()
            .insert(path.to_owned());
    }

    pub fn merge_worktrees(&self, scope: &PreferencesScope, snapshot: &mut HashMap<String, bool>, current: &HashMap<String, bool>) {
        if let Some(paths) = self.edited_worktrees.get(scope) {
            for path in paths {
                match current.get(path) {
                    Some(enabled) => { snapshot.insert(path.clone(), *enabled); }
                    None => { snapshot.remove(path); }
                }
            }
        }
    }
}
