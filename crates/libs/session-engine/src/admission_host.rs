//! Only actual scope-sensitive actions called by session admission.
use std::{collections::HashMap, future::Future, path::Path};
use anyhow::Result;
use agent_config::runtime::Config;
use config_agent::AgentDefinitionConfig;
use config_agent_profile::{AgentRuntime, AgentModelReference};
use crate::agent_binding::AgentBinding;

pub trait AdmissionHost {
    fn config() -> impl Future<Output = Result<Config>> + Send;
    fn project() -> Option<String>;
    fn fork_selectable(project: &str, session: &str) -> impl Future<Output = Result<bool>> + Send;
    fn load_binding(session: &str) -> impl Future<Output = Result<Option<AgentBinding>>> + Send;
    fn save_binding(project: &str, session: &str, binding: &AgentBinding) -> impl Future<Output = Result<()>> + Send;
    fn history_agents(session: &str) -> impl Future<Output = Result<Vec<String>>> + Send;
    fn agent(key: &str) -> impl Future<Output = Option<AgentDefinitionConfig>> + Send;
    fn skills(workspace: &Path, config: &Config) -> HashMap<String, bool>;
    fn mcp(workspace: &Path, runtime: &AgentRuntime) -> impl Future<Output = Result<HashMap<String, bool>>> + Send;
    fn permission_ceiling(key: &str) -> impl Future<Output = Result<permission_runtime::next::Ruleset>> + Send;
    fn default_model() -> impl Future<Output = Result<AgentModelReference>> + Send;
}
