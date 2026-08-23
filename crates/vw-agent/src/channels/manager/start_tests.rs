// 本文件验证渠道启动前的提供者初始化、工具装配与公开入口连线。

use super::*;
use crate::app::agent::tools::ToolResult;
use std::path::PathBuf;

struct NamedChannel(&'static str);

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for NamedChannel {
    fn name(&self) -> &str {
        self.0
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(message.to_string())
    }
}

struct AgentToolStub;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Tool for AgentToolStub {
    fn name(&self) -> &str {
        "AgentTool"
    }

    fn description(&self) -> &str {
        "agent tool"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: String::new(), error: None })
    }
}

fn telegram_config(interrupt_on_new_message: bool) -> crate::app::agent::config::TelegramConfig {
    crate::app::agent::config::TelegramConfig {
        bot_token: "token".to_string(),
        allowed_users: vec!["alice".to_string()],
        stream_mode: crate::app::agent::config::StreamMode::Off,
        draft_update_interval_ms: 1000,
        interrupt_on_new_message,
        mention_only: false,
        group_reply: None,
        base_url: None,
    }
}

fn tool_names(descriptions: &[(&'static str, &'static str)]) -> Vec<&'static str> {
    descriptions.iter().map(|(name, _)| *name).collect()
}

#[tokio::test]
// 提供者初始化失败时不得启动监听器，避免半初始化状态。
async fn start_channels_returns_provider_initialization_error_before_listener_startup() {
    let mut config = Config::default();
    config.default_provider = Some("__missing_provider_for_start_test__".to_string());
    config.default_model = Some("model".to_string());
    config.channels_config.telegram = None;
    config.channels_config.discord = None;
    config.channels_config.slack = None;
    config.channels_config.matrix = None;

    let err = start_channels(config).await.expect_err("missing provider should fail");

    assert!(!err.to_string().is_empty());
}

#[test]
fn start_module_exports_start_channels_symbol() {
    let _ = start_channels;
}

#[test]
fn provider_runtime_options_preserve_config_fields() {
    let mut config = Config::default();
    config.api_url = Some("https://api.example.test/v1".to_string());
    config.config_path = PathBuf::from("/tmp/vw-start-test/config/vibewindow.json");
    config.secrets.encrypt = false;
    config.runtime.reasoning_enabled = Some(true);
    config.provider_api = Some(vw_config_types::provider::ProviderApiMode::OpenAiResponses);
    config.model_support_vision = Some(true);

    let options = build_provider_runtime_options(&config);
    let expected_vibewindow_dir = config.workspace_dir.clone();

    assert_eq!(options.provider_api_url.as_deref(), Some("https://api.example.test/v1"));
    assert_eq!(options.vibewindow_dir.as_deref(), Some(expected_vibewindow_dir.as_path()));
    assert!(!options.secrets_encrypt);
    assert_eq!(options.reasoning_enabled, Some(true));
    assert_eq!(
        options.custom_provider_api_mode,
        Some(vw_config_types::provider::CompatibleApiMode::OpenAiResponses)
    );
    assert_eq!(options.model_support_vision, Some(true));
    assert!(options.auth_profile_override.is_none());
    assert!(options.max_tokens_override.is_none());
}

#[test]
fn channel_tool_descriptions_track_optional_tools_and_exclusions() {
    let mut config = Config::default();
    let base_names = tool_names(&build_channel_tool_descriptions(&config, false));

    assert!(base_names.contains(&"shell"));
    assert!(!base_names.contains(&"schedule"));
    assert!(!base_names.contains(&"Browser"));
    assert!(!base_names.contains(&"AgentTool"));

    config.browser.enabled = true;
    config.browser.browser_open = "disable".to_string();
    config.web_fetch.enabled = true;
    config.web_search.enabled = true;
    config.autonomy.non_cli_excluded_tools =
        vec!["shell".to_string(), "WebSearch".to_string(), "AgentTool".to_string()];

    let optional_names = tool_names(&build_channel_tool_descriptions(&config, true));

    assert!(optional_names.contains(&"Browser"));
    assert!(!optional_names.contains(&"BrowserOpen"));
    assert!(optional_names.contains(&"WebFetch"));
    assert!(!optional_names.contains(&"shell"));
    assert!(!optional_names.contains(&"WebSearch"));
    assert!(!optional_names.contains(&"AgentTool"));
}

#[test]
fn channel_tool_descriptions_include_browser_open_when_not_disabled() {
    let mut config = Config::default();
    config.browser.enabled = true;
    config.browser.browser_open = "system".to_string();

    let names = tool_names(&build_channel_tool_descriptions(&config, false));

    assert!(names.contains(&"BrowserOpen"));
    assert!(names.contains(&"Browser"));
}

#[test]
fn bootstrap_and_listener_backoff_use_runtime_defaults() {
    let mut config = Config::default();
    config.reliability.channel_initial_backoff_secs = 0;
    config.reliability.channel_max_backoff_secs = 0;

    assert_eq!(bootstrap_max_chars(&config), None);
    assert_eq!(
        channel_listener_backoff_secs(&config),
        (DEFAULT_CHANNEL_INITIAL_BACKOFF_SECS, DEFAULT_CHANNEL_MAX_BACKOFF_SECS)
    );

    vw_config_types::agent::ensure_main_agent_config(&mut config.agents).compact_context = true;
    config.reliability.channel_initial_backoff_secs = DEFAULT_CHANNEL_INITIAL_BACKOFF_SECS + 7;
    config.reliability.channel_max_backoff_secs = DEFAULT_CHANNEL_MAX_BACKOFF_SECS + 9;

    assert_eq!(bootstrap_max_chars(&config), Some(6000));
    assert_eq!(
        channel_listener_backoff_secs(&config),
        (DEFAULT_CHANNEL_INITIAL_BACKOFF_SECS + 7, DEFAULT_CHANNEL_MAX_BACKOFF_SECS + 9)
    );
}

#[test]
fn channels_by_name_and_provider_cache_seed_clone_shared_handles() {
    let alpha: Arc<dyn Channel> = Arc::new(NamedChannel("alpha"));
    let beta: Arc<dyn Channel> = Arc::new(NamedChannel("beta"));
    let channels = vec![Arc::clone(&alpha), Arc::clone(&beta)];

    let by_name = build_channels_by_name(&channels);

    assert_eq!(by_name.len(), 2);
    assert!(Arc::ptr_eq(by_name.get("alpha").expect("alpha channel"), &alpha));
    assert!(Arc::ptr_eq(by_name.get("beta").expect("beta channel"), &beta));

    let provider: Arc<dyn Provider> = Arc::new(StaticProvider);
    let cache = seed_provider_cache("default-provider", &provider);

    assert_eq!(cache.len(), 1);
    assert!(Arc::ptr_eq(cache.get("default-provider").expect("provider"), &provider));
}

#[test]
fn telegram_interrupt_flag_defaults_false_and_tracks_config() {
    let mut config = Config::default();

    assert!(!channel_interrupt_on_new_message(&config));

    config.channels_config.telegram = Some(telegram_config(false));
    assert!(!channel_interrupt_on_new_message(&config));

    config.channels_config.telegram = Some(telegram_config(true));
    assert!(channel_interrupt_on_new_message(&config));
}

#[test]
fn build_channel_hooks_respects_enabled_flag() {
    let mut config = Config::default();
    config.hooks.enabled = false;

    assert!(build_channel_hooks(&config).expect("hooks disabled").is_none());

    config.hooks.enabled = true;
    config.hooks.builtin.command_logger = false;

    assert!(build_channel_hooks(&config).expect("hooks enabled").is_some());

    config.hooks.builtin.command_logger = true;

    assert!(build_channel_hooks(&config).expect("hooks enabled with logger").is_some());
}

#[tokio::test]
async fn channel_hooks_use_shared_runtime_builder() {
    let mut config = Config::default();
    config.hooks.enabled = true;
    config.hooks.builtin.command_logger = true;
    config.hooks.plugins.enabled = false;

    let hooks = build_channel_hooks(&config)
        .expect("build channel hooks")
        .expect("enabled hook runner");
    hooks
        .fire_after_tool_call("channel-tool", &ToolResult::default(), Duration::from_millis(3))
        .await;

    let runtime = crate::app::agent::hooks::build_hook_runtime(&config)
        .expect("build shared hook runtime");
    let shared_hooks = runtime.runner_snapshot().expect("shared hook runner");
    shared_hooks
        .fire_after_tool_call("shared-tool", &ToolResult::default(), Duration::from_millis(4))
        .await;
    assert_eq!(runtime.audit_store().entries()[0].tool_name, "shared-tool");
}

#[test]
fn registered_agent_tool_flag_matches_registry_name_check() {
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(AgentToolStub)];
    let agent_tool_registered = tools.iter().any(|tool| tool.name() == "AgentTool");

    let descriptions = build_channel_tool_descriptions(&Config::default(), agent_tool_registered);
    let names = tool_names(&descriptions);

    assert!(names.contains(&"AgentTool"));
    let agent_tool = descriptions
        .iter()
        .find_map(|(name, description)| (*name == "AgentTool").then_some(*description))
        .expect("AgentTool description");
    for expected in [
        "complex",
        "simple",
        "parallel",
        "background",
        "run",
        "status",
        "wait",
        "stop",
        "synthesize",
    ] {
        assert!(agent_tool.contains(expected), "missing '{expected}': {agent_tool}");
    }
    for unsupported in ["action=list", "action=get"] {
        assert!(
            !agent_tool.contains(unsupported),
            "unsupported '{unsupported}' advertised: {agent_tool}"
        );
    }
}

#[test]
fn channel_tool_descriptions_disable_browser_open_case_insensitively() {
    let mut config = Config::default();
    config.browser.enabled = true;
    config.browser.browser_open = "DiSaBlE".to_string();

    let names = tool_names(&build_channel_tool_descriptions(&config, false));

    assert!(!names.contains(&"BrowserOpen"));
    assert!(names.contains(&"Browser"));
}

#[test]
fn channels_by_name_keep_last_channel_for_duplicate_name() {
    let first: Arc<dyn Channel> = Arc::new(NamedChannel("duplicate"));
    let second: Arc<dyn Channel> = Arc::new(NamedChannel("duplicate"));

    let by_name = build_channels_by_name(&[Arc::clone(&first), Arc::clone(&second)]);

    assert_eq!(by_name.len(), 1);
    assert!(Arc::ptr_eq(by_name.get("duplicate").expect("duplicate channel"), &second));
}
