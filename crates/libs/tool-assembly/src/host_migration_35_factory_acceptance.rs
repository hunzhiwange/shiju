//! Factory acceptance uses the caller's actual concrete construction ports.
use agent_config::runtime_schema::{Config, BrowserConfig, MemoryConfig, DelegateAgentConfig};
use agent_memory::{self as memory, Memory};
use agent_security::SecurityPolicy;
use std::{sync::Arc, collections::HashMap};
use tempfile::TempDir;
fn test_config(tmp: &TempDir) -> Config {
    Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    }
}
pub fn all_tools_excludes_browser_when_disabled<H: crate::factory::FactoryHost>() {
    let tmp = TempDir::new().unwrap();
    let security = Arc::new(SecurityPolicy::default());
    let mem_cfg = MemoryConfig { backend: "markdown".into(), ..MemoryConfig::default() };
    let mem: Arc<dyn Memory> =
        Arc::from(memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

    let browser = BrowserConfig {
        enabled: false,
        allowed_domains: vec!["example.com".into()],
        session_name: None,
        ..BrowserConfig::default()
    };
    let http = agent_config::runtime_schema::HttpRequestConfig::default();
    let cfg = test_config(&tmp);

    let tools = crate::factory::all_tools::<H>(
        Arc::new(Config::default()),
        &security,
        mem,
        &browser,
        &http,
        &agent_config::runtime_schema::WebFetchConfig::default(),
        tmp.path(),
        &HashMap::new(),
        None,
        &cfg,
        None,
    );
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(!names.contains(&"browser_open"));
    assert!(names.contains(&"schedule"));
    assert!(names.contains(&"Config"));
    assert!(names.contains(&"Sleep"));
    assert!(!names.contains(&"model_routing_config"));
    assert!(!names.contains(&"proxy_config"));
}
pub fn all_tools_includes_browser_when_enabled<H: crate::factory::FactoryHost>() {
    let tmp = TempDir::new().unwrap();
    let security = Arc::new(SecurityPolicy::default());
    let mem_cfg = MemoryConfig { backend: "markdown".into(), ..MemoryConfig::default() };
    let mem: Arc<dyn Memory> =
        Arc::from(memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

    let browser = BrowserConfig {
        enabled: true,
        allowed_domains: vec!["example.com".into()],
        session_name: None,
        ..BrowserConfig::default()
    };
    let http = agent_config::runtime_schema::HttpRequestConfig::default();
    let cfg = test_config(&tmp);

    let tools = crate::factory::all_tools::<H>(
        Arc::new(Config::default()),
        &security,
        mem,
        &browser,
        &http,
        &agent_config::runtime_schema::WebFetchConfig::default(),
        tmp.path(),
        &HashMap::new(),
        None,
        &cfg,
        None,
    );
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(names.contains(&"browser_open"));
    assert!(names.contains(&"glob"));
    assert!(names.contains(&"grep"));
    assert!(!names.contains(&"process"));
    assert!(!names.contains(&"glob_search"));
    assert!(!names.contains(&"content_search"));
    assert!(!names.contains(&"model_routing_config"));
    assert!(!names.contains(&"proxy_config"));
    assert!(names.contains(&"Config"));
    assert!(names.contains(&"Sleep"));
}
pub fn default_tools_all_have_descriptions<H: crate::factory::FactoryHost>() {
    let security = Arc::new(SecurityPolicy::default());
    let tools = crate::factory::default_tools::<H>(security);
    for tool in &tools {
        assert!(!tool.description().is_empty(), "Tool {} has empty description", tool.name());
    }
}
pub fn default_tools_all_have_schemas<H: crate::factory::FactoryHost>() {
    let security = Arc::new(SecurityPolicy::default());
    let tools = crate::factory::default_tools::<H>(security);
    for tool in &tools {
        let schema = tool.parameters_schema();
        assert!(schema.is_object(), "Tool {} schema is not an object", tool.name());
        assert!(schema["properties"].is_object(), "Tool {} schema has no properties", tool.name());
    }
}
pub fn tool_spec_generation<H: crate::factory::FactoryHost>() {
    let security = Arc::new(SecurityPolicy::default());
    let tools = crate::factory::default_tools::<H>(security);
    for tool in &tools {
        let spec = tool.spec();
        let expected_id = if tool.name() == "shell" { "bash" } else { tool.name() };
        assert_eq!(spec.id, expected_id);
        assert_eq!(spec.description, tool.description());
        assert!(spec.input_schema.is_object());
    }
}
pub fn all_tools_includes_agent_tool_when_agents_configured<H: crate::factory::FactoryHost>() {

    let mut agents = HashMap::new();
    agents.insert(
        "researcher".to_string(),
        crate::host_migration_35_fixtures::agent("all", true),
    );

    let names = agent_factory_names::<H>(&agents, Some("delegate-test-credential"));
    assert!(names.iter().any(|name| name == "AgentTool"));
    assert!(!names.iter().any(|name| name == "delegate"));
}
pub fn all_tools_includes_default_agent_tool_when_no_agents<H: crate::factory::FactoryHost>() {

    let names = agent_factory_names::<H>(&HashMap::new(), None);
    assert!(names.iter().any(|name| name == "AgentTool"));
    assert!(!names.iter().any(|name| name == "delegate"));
}
pub fn all_tools_includes_default_agent_tool_when_only_primary_agents<H: crate::factory::FactoryHost>() {

    let mut agents = HashMap::new();
    agents.insert(
        "main".to_string(),
        crate::host_migration_35_fixtures::agent("primary", false),
    );

    let names = agent_factory_names::<H>(&agents, None);
    assert!(names.iter().any(|name| name == "AgentTool"));
    assert!(!names.iter().any(|name| name == "delegate"));
}
pub fn all_tools_registers_agent_tool_for_delegate_agents<H: crate::factory::FactoryHost>() {

    let mut agents = HashMap::new();
    agents.insert(
        "researcher".to_string(),
        crate::host_migration_35_fixtures::agent("all", true),
    );

    let names = agent_factory_names::<H>(&agents, Some("delegate-test-credential"));
    assert!(names.iter().any(|name| name == "AgentTool"));
    assert!(!names.iter().any(|name| name == "delegate"));
}
fn agent_factory_names<H: crate::factory::FactoryHost>(agents: &HashMap<String, DelegateAgentConfig>, credential: Option<&str>) -> Vec<String> {
    let tmp = TempDir::new().unwrap();
    let security = Arc::new(SecurityPolicy::default());
    let mem_cfg = MemoryConfig { backend: "markdown".into(), ..MemoryConfig::default() };
    let mem: Arc<dyn Memory> = Arc::from(memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());
    let cfg = test_config(&tmp);
    crate::factory::all_tools::<H>(Arc::new(Config::default()), &security, mem, &BrowserConfig::default(),
        &agent_config::runtime_schema::HttpRequestConfig::default(),
        &agent_config::runtime_schema::WebFetchConfig::default(), tmp.path(), agents, credential, &cfg, None)
        .iter().map(|tool| tool.name().to_string()).collect()
}
