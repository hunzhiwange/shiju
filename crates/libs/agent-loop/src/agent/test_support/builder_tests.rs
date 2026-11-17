//! 测试代理构建器的必需依赖校验、默认配置与工具规格收集。
//! 最小伪实现按依赖装配顺序暴露缺失项，并确认可选配置不会改变默认契约。

use crate::agent::builder::AgentBuilder;
use crate::memory_loader::MemoryLoader;
use crate::prompt::SystemPromptBuilder;
use agent_approval::ApprovalManager;
use agent_config::runtime::{
    AutonomyConfig, IdentityConfig, MultimodalConfig, QueryClassificationConfig,
    SkillsPromptInjectionMode,
};
use agent_memory::NoneMemory;
use agent_observability::NoopObserver;
use model_types::Provider;
use agent_security::SecurityPolicy;
use agent_skills::types::Skill;
use tool_types::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use config_agent::AgentDefinitionConfig;

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
        _memory: &dyn agent_memory::Memory,
        _user_message: &str,
    ) -> anyhow::Result<String> {
        Ok("loaded".into())
    }
}

// 从最小可构建状态出发，缺失依赖测试再按装配顺序逐项移除。
fn required_builder<H: crate::agent::AgentHost>() -> AgentBuilder<H> {
    AgentBuilder::<H>::new()
        .tools(vec![Box::new(TestTool { name: "first_tool" })])
        .provider(Box::new(TestProvider))
        .memory(Arc::new(NoneMemory::new()))
        .observer(Arc::new(NoopObserver))
}

fn required_builder_with_tools<H: crate::agent::AgentHost>(tools: Vec<Box<dyn Tool>>) -> AgentBuilder<H> {
    AgentBuilder::<H>::new()
        .tools(tools)
        .provider(Box::new(TestProvider))
        .memory(Arc::new(NoneMemory::new()))
        .observer(Arc::new(NoopObserver))
}

pub fn build_reports_first_missing_required_dependency<H: crate::agent::AgentHost>() {
    let error = match AgentBuilder::<H>::new().build() {
        Ok(_) => panic!("builder should reject missing required dependencies"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("tools are required"));
}

pub fn build_reports_missing_provider_after_tools_are_present<H: crate::agent::AgentHost>() {
    let error = match AgentBuilder::<H>::new()
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

pub fn build_reports_missing_memory_after_provider_is_present<H: crate::agent::AgentHost>() {
    let error = match AgentBuilder::<H>::new()
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

pub fn build_reports_missing_observer_after_memory_is_present<H: crate::agent::AgentHost>() {
    let error = match AgentBuilder::<H>::new()
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

pub fn build_fills_defaults_when_optional_fields_are_absent<H: crate::agent::AgentHost>() {
    let agent = required_builder::<H>().build().unwrap();

    assert_eq!(agent.tools.len(), 1);
    assert_eq!(agent._tool_specs.len(), 1);
    assert_eq!(agent._tool_specs[0].id, "first_tool");
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

pub fn build_collects_tool_specs_in_tool_order<H: crate::agent::AgentHost>() {
    let agent = required_builder_with_tools::<H>(vec![
        Box::new(TestTool { name: "first_tool" }),
        Box::new(TestTool { name: "second_tool" }),
    ])
    .build()
    .unwrap();

    let tool_spec_ids = agent._tool_specs.iter().map(|spec| spec.id.as_str()).collect::<Vec<_>>();

    assert_eq!(tool_spec_ids, vec!["first_tool", "second_tool"]);
}

pub fn build_uses_configured_optional_fields<H: crate::agent::AgentHost>() {
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
    let config =
        AgentDefinitionConfig { max_history_messages: 7, ..AgentDefinitionConfig::default() };
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

    let agent = required_builder::<H>()
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

pub async fn build_uses_configured_memory_loader<H: crate::agent::AgentHost>() {
    let agent = required_builder::<H>().memory_loader(Box::new(TestMemoryLoader)).build().unwrap();

    let loaded = agent.memory_loader.load_context(agent.memory.as_ref(), "message").await.unwrap();

    assert_eq!(loaded, "loaded");
}
