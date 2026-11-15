use agent_hooks::HookRunner;
use tokio_util::sync::CancellationToken;

use crate::app::agent::agent::memory_loader::DefaultMemoryLoader;
use crate::app::agent::agent::prompt::SystemPromptBuilder;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::Config;
use crate::app::agent::memory::{self, Memory};
use crate::app::agent::observability::{self, Observer};
use model_types::{ChatMessage, Provider};
use crate::app::agent::runtime;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::Tool;

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;


pub type Agent = agent_loop::agent::Agent<crate::agent::loop_::core::skills::NativeCoreSkillHost>;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl agent_loop::agent::AgentHost for crate::agent::loop_::core::skills::NativeCoreSkillHost {
    fn from_config(config: &Config) -> Result<Agent> {
        let timezone = crate::app::agent::util::locale::parse_timezone(
            &config.app_ui.system_settings.timezone,
        )?;
        let observer: Arc<dyn Observer> =
            Arc::from(observability::create_observer(&config.observability));
        let runtime: Arc<dyn runtime_types::RuntimeAdapter> =
            Arc::from(runtime::create_runtime(&config.runtime)?);
        let security = Arc::new(SecurityPolicy::from_configs(
            &config.autonomy,
            &config.security,
            &config.workspace_dir,
        ));

        let memory: Arc<dyn Memory> = Arc::from(memory::create_memory_with_storage_and_routes(
            &config.memory,
            &config.embedding_routes,
            Some(&config.storage.provider.config),
            &config.workspace_dir,
            config.api_key.as_deref(),
        )?);

        let tools = crate::app::agent::tools::all_tools_with_runtime(
            Arc::new(config.clone()),
            &security,
            runtime,
            memory.clone(),
            &config.browser,
            &config.http_request,
            &config.web_fetch,
            &config.workspace_dir,
            &config.agents,
            config.api_key.as_deref(),
            config,
            None,
        );

        let provider_name = config.default_provider.as_deref().unwrap_or("zhipuai-coding-plan");
        let model_name =
            config.default_model.as_deref().unwrap_or("zhipuai-coding-plan/glm-5").to_string();

        let provider_runtime_options = crate::app::agent::providers::ProviderRuntimeOptions {
            configured_provider_ids: config.providers.keys().cloned().collect(),
            provider_api_url: config.api_url.clone(),
            vibewindow_dir: config.config_path.parent().map(std::path::PathBuf::from),
            secrets_encrypt: config.secrets.encrypt,
            reasoning_enabled: config.runtime.reasoning_enabled,
            reasoning_level: config.effective_provider_reasoning_level(),
            custom_provider_api_mode: config.provider_api.map(|mode| mode.as_compatible_mode()),
            model_support_vision: config.model_support_vision,
            ..crate::app::agent::providers::ProviderRuntimeOptions::default()
        };
        let provider: Box<dyn Provider> =
            crate::app::agent::providers::create_routed_provider_with_options(
                provider_name,
                config.api_key.as_deref(),
                config.api_url.as_deref(),
                &config.reliability,
                &config.model_routes,
                &model_name,
                &provider_runtime_options,
            )?;

        let approval = ApprovalManager::from_config(&config.autonomy);

        let route_model_by_hint: HashMap<String, String> = config
            .model_routes
            .iter()
            .map(|route| (route.hint.clone(), format!("{}/{}", route.provider, route.model)))
            .collect();
        let available_hints: Vec<String> = route_model_by_hint.keys().cloned().collect();

        let mut agent = Agent::builder()
            .provider(provider)
            .tools(tools)
            .memory(memory)
            .observer(observer)
            .security(security.clone())
            .approval(approval)
            .memory_loader(Box::new(DefaultMemoryLoader::new(5, config.memory.min_relevance_score)))
            .prompt_builder(SystemPromptBuilder::with_defaults())
            .config(config_agent::effective_main_agent_config(&config.agents))
            .model_name(model_name)
            .temperature(config.default_temperature)
            .workspace_dir(config.workspace_dir.clone())
            .classification_config(config.query_classification.clone())
            .available_hints(available_hints)
            .route_model_by_hint(route_model_by_hint)
            .identity_config(config.identity.clone())
            .skills(crate::app::agent::skills::load_skills_with_config(
                &config.workspace_dir,
                config,
            ))
            .skills_prompt_mode(config.skills.prompt_injection_mode)
            .auto_save(config.memory.auto_save)
            .multimodal_config(config.multimodal.clone())
            .timezone(timezone)
            .build()?;
        agent.bind_skill_runtime_config(Arc::new(config.clone()));
        Ok(agent)

    }
    async fn models() -> Result<Vec<model_types::Model>> {
        Ok(crate::provider::provider::list().await?.values().flat_map(|provider| provider.models.values()).cloned().collect())
    }

async fn run_tool_call_loop_with_skills(
    provider: &dyn Provider,
    history: &mut Vec<ChatMessage>,
    tools_registry: &[Box<dyn Tool>],
    observer: &dyn Observer,
    provider_name: &str,
    model: &str,
    temperature: f64,
    silent: bool,
    approval: Option<Arc<ApprovalManager>>,
    channel_name: &str,
    multimodal_config: &crate::app::agent::config::MultimodalConfig,
    max_iterations: usize,
    cancellation_token: Option<CancellationToken>,
    on_delta: Option<tokio::sync::mpsc::Sender<String>>,
    hooks: Option<Arc<HookRunner>>,
    security: Option<Arc<SecurityPolicy>>,
    excluded_tools: &[String],
    skills: Option<&mut crate::agent::loop_::core::skills::CoreSkillSession>,
) -> Result<String> {
        crate::agent::loop_::core::run_tool_call_loop_with_skills(provider, history, tools_registry, observer, provider_name, model, temperature, silent, approval, channel_name, multimodal_config, max_iterations, cancellation_token, on_delta, hooks, security, excluded_tools, skills).await
    }
}
