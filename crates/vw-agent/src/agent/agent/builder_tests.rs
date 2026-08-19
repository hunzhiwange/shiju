use super::builder::AgentBuilder;
use crate::app::agent::agent::memory_loader::MemoryLoader;
use crate::app::agent::agent::prompt::SystemPromptBuilder;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::{
    AutonomyConfig, IdentityConfig, MultimodalConfig, QueryClassificationConfig,
    SkillsPromptInjectionMode,
};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::skills::Skill;
use crate::app::agent::tools::{Tool, ToolResult};
use async_trait::async_trait;
use vw_config_types::agent::AgentDefinitionConfig;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

struct TestProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for TestProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("ok".into())
    }
}

struct TestTool {
    name: &'static str,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for TestTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "test tool"
    }

    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: String::new(), error: None })
    }
}

struct TestMemoryLoader;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl MemoryLoader for TestMemoryLoader {
    async fn load_context(
        &self,
        _memory: &dyn crate::app::agent::memory::Memory,
        _user_message: &str,
    ) -> anyhow::Result<String> {
        Ok("loaded".into())
    }
}

fn required_builder() -> AgentBuilder {
    AgentBuilder::new()
        .tools(vec![Box::new(TestTool { name: "first_tool" })])
        .provider(Box::new(TestProvider))
        .memory(Arc::new(NoneMemory::new()))
        .observer(Arc::new(NoopObserver))
}

fn required_builder_with_tools(tools: Vec<Box<dyn Tool>>) -> AgentBuilder {
    AgentBuilder::new()
        .tools(tools)
        .provider(Box::new(TestProvider))
        .memory(Arc::new(NoneMemory::new()))
        .observer(Arc::new(NoopObserver))
}

#[test]
fn build_reports_first_missing_required_dependency() {
    let error = match AgentBuilder::new().build() {
        Ok(_) => panic!("builder should reject missing required dependencies"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("tools are required"));
}

#[test]
fn build_reports_missing_provider_after_tools_are_present() {
    let error = match AgentBuilder::new()
        .tools(Vec::new())
        .memory(Arc::new(NoneMemory::new()))
        .observer(Arc::new(NoopObserver))
        .build()
    {
        Ok(_) => panic!("builder should reject a missing provider"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("provider is required"));
}

#[test]
fn build_reports_missing_memory_after_provider_is_present() {
    let error = match AgentBuilder::new()
        .tools(Vec::new())
        .provider(Box::new(TestProvider))
        .observer(Arc::new(NoopObserver))
        .build()
    {
        Ok(_) => panic!("builder should reject a missing memory"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("memory is required"));
}

#[test]
fn build_reports_missing_observer_after_memory_is_present() {
    let error = match AgentBuilder::new()
        .tools(Vec::new())
        .provider(Box::new(TestProvider))
        .memory(Arc::new(NoneMemory::new()))
        .build()
    {
        Ok(_) => panic!("builder should reject a missing observer"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("observer is required"));
}

#[test]
fn build_fills_defaults_when_optional_fields_are_absent() {
    let agent = required_builder().build().unwrap();

    assert_eq!(agent.tools.len(), 1);
    assert_eq!(agent.tool_specs.len(), 1);
    assert_eq!(agent.tool_specs[0].id, "first_tool");
    assert_eq!(agent.memory.name(), "none");
    assert_eq!(agent.observer.name(), "noop");
    assert_eq!(agent.config.max_iterations, 50_000);
    assert_eq!(agent.config.max_history_messages, 100);
    assert!(!agent.config.parallel_tools);
    assert_eq!(agent.model_name, "anthropic/claude-sonnet-4-20250514");
    assert_eq!(agent.temperature, 0.7);
    assert_eq!(agent.workspace_dir, PathBuf::from("."));
    assert_eq!(agent.identity_config.format, "openclaw");
    assert!(agent.security.is_none());
    assert!(agent.approval.is_none());
    assert_eq!(agent.skills_prompt_mode, SkillsPromptInjectionMode::Compact);
    assert!(!agent.auto_save);
    assert!(agent.history.is_empty());
    assert!(agent.skills.is_empty());
    assert!(!agent.classification_config.enabled);
    assert!(agent.available_hints.is_empty());
    assert!(agent.route_model_by_hint.is_empty());
    assert_eq!(agent.multimodal_config.max_images, 10);
    assert_eq!(agent.multimodal_config.max_image_size_mb, 5);
    assert!(agent.multimodal_config.allow_remote_fetch);
    assert_eq!(agent.timezone, chrono_tz::Asia::Shanghai);
}

#[test]
fn build_collects_tool_specs_in_tool_order() {
    let agent = required_builder_with_tools(vec![
        Box::new(TestTool { name: "first_tool" }),
        Box::new(TestTool { name: "second_tool" }),
    ])
    .build()
    .unwrap();

    let tool_spec_ids = agent.tool_specs.iter().map(|spec| spec.id.as_str()).collect::<Vec<_>>();

    assert_eq!(tool_spec_ids, vec!["first_tool", "second_tool"]);
}

#[test]
fn build_uses_configured_optional_fields() {
    let mut routes = HashMap::new();
    routes.insert("fast".to_string(), "test-model-fast".to_string());

    let skill = Skill {
        name: "test-skill".into(),
        description: "test skill".into(),
        version: "1.0.0".into(),
        author: None,
        tags: vec!["testcov-0075".into()],
        tools: Vec::new(),
        prompts: Vec::new(),
        location: None,
        ..Skill::default()
    };
    let classification_config = QueryClassificationConfig { enabled: true, rules: Vec::new() };
    let config = AgentDefinitionConfig {
        max_history_messages: 7,
        ..AgentDefinitionConfig::default()
    };
    let identity_config = IdentityConfig {
        format: "test-format".into(),
        aieos_path: None,
        aieos_inline: Some("inline identity".into()),
    };
    let multimodal_config = MultimodalConfig {
        max_images: 2,
        max_image_size_mb: 3,
        allow_remote_fetch: true,
        ocr: Default::default(),
    };

    let agent = required_builder()
        .security(Arc::new(SecurityPolicy::default()))
        .prompt_builder(SystemPromptBuilder::with_defaults())
        .approval(ApprovalManager::from_config(&AutonomyConfig::default()))
        .memory_loader(Box::new(TestMemoryLoader))
        .config(config)
        .model_name("test-model".into())
        .temperature(0.25)
        .workspace_dir(PathBuf::from("/tmp/testcov-0075"))
        .identity_config(identity_config)
        .skills(vec![skill])
        .skills_prompt_mode(SkillsPromptInjectionMode::Compact)
        .auto_save(true)
        .classification_config(classification_config)
        .available_hints(vec!["fast".into()])
        .route_model_by_hint(routes)
        .multimodal_config(multimodal_config)
        .timezone(chrono_tz::America::New_York)
        .build()
        .unwrap();

    assert!(agent.security.is_some());
    assert!(agent.approval.is_some());
    assert_eq!(agent.config.max_history_messages, 7);
    assert_eq!(agent.model_name, "test-model");
    assert_eq!(agent.temperature, 0.25);
    assert_eq!(agent.workspace_dir, PathBuf::from("/tmp/testcov-0075"));
    assert_eq!(agent.identity_config.format, "test-format");
    assert_eq!(agent.skills.len(), 1);
    assert_eq!(agent.skills[0].name, "test-skill");
    assert_eq!(agent.skills_prompt_mode, SkillsPromptInjectionMode::Compact);
    assert!(agent.auto_save);
    assert!(agent.classification_config.enabled);
    assert_eq!(agent.available_hints, vec!["fast"]);
    assert_eq!(agent.route_model_by_hint.get("fast").map(String::as_str), Some("test-model-fast"));
    assert_eq!(agent.multimodal_config.max_images, 2);
    assert_eq!(agent.multimodal_config.max_image_size_mb, 3);
    assert!(agent.multimodal_config.allow_remote_fetch);
    assert_eq!(agent.timezone, chrono_tz::America::New_York);
}

#[tokio::test]
async fn build_uses_configured_memory_loader() {
    let agent = required_builder().memory_loader(Box::new(TestMemoryLoader)).build().unwrap();

    let loaded = agent.memory_loader.load_context(agent.memory.as_ref(), "message").await.unwrap();

    assert_eq!(loaded, "loaded");
}
