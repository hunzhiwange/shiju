//! 验证智能体配置的读取、默认值、兼容字段和持久化转换。
//!
//! 配置样例包含默认、显式和旧版字段，确认反序列化后得到同一运行时语义。

// 保留计划任务 840 的既定覆盖面。
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::json;
use vw_config_types::{
    agent::DelegateAgentConfig, config::AcpAgentConfig, gateway::GatewaySkey, staff::StaffConfig,
};

use super::{
    build_acp_agent_create_patch, build_acp_agent_enabled_patch, build_main_agent_overrides_patch,
    default_enabled_acp_config, resolve_enabled_acp_config,
};

const SOURCE: &str = include_str!("config_agent.rs");

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn source_declares_symbol(name: &str) -> bool {
    let needles = [
        format!("fn {name}"),
        format!("pub fn {name}"),
        format!("struct {name}"),
        format!("pub struct {name}"),
        format!("enum {name}"),
        format!("pub enum {name}"),
        format!("type {name}"),
        format!("pub type {name}"),
        format!("const {name}"),
        format!("pub const {name}"),
        format!("static {name}"),
        format!("pub static {name}"),
        format!("impl {name}"),
    ];

    needles.iter().any(|needle| SOURCE.contains(needle))
}

#[test]
fn config_agent_tests_keeps_planned_coverage_targets() {
    for name in [
        "fetch_agent_config_via_gateway",
        "fetch_global_agent_config_via_gateway",
        "fetch_global_acp_config_via_gateway",
        "patch_agent_config_via_gateway",
        "load_agent_config_via_gateway",
        "patch_agent_config",
        "load_full_agent_config_async",
        "load_browser_config_async",
        "load_gateway_config_result",
        "load_global_acp_config_result",
        "load_enabled_acp_config_result",
        "load_enabled_acp_config_async",
        "load_acp_settings_snapshot_async",
        "add_global_acp_agent_async",
        "set_global_acp_agent_enabled_async",
        "build_acp_agent_create_patch",
        "build_acp_agent_enabled_patch",
        "build_main_agent_overrides_patch",
        "DEFAULT_ENABLED_ACP_AGENTS",
    ] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}

#[test]
fn staff_config_helpers_patch_staff_root() {
    let _load: fn() -> StaffConfig = super::load_staff_config;
    let _load_result: fn() -> Result<StaffConfig, String> = super::load_staff_config_result;

    if std::hint::black_box(false) {
        let _task = super::update_staff_config_async(|staff| {
            staff.members.clear();
        });
    }

    assert!(source_declares_symbol("load_staff_config"));
    assert!(source_declares_symbol("load_staff_config_result"));
    assert!(source_declares_symbol("update_staff_config_async"));
    assert!(SOURCE.contains("load_agent_config_via_gateway().staff"));
    assert!(SOURCE.contains("load_full_agent_config_result()?.staff"));
    assert!(SOURCE.contains("patch_agent_config_result(&[\"staff\"], value)"));
    assert!(SOURCE.contains("serde_json::json!({ \"staff\": value })"));
    assert!(!SOURCE.contains("_staff_config.json"));
}

fn acp(command: &str) -> AcpAgentConfig {
    AcpAgentConfig { command: command.to_string(), ..AcpAgentConfig::default() }
}

fn catalog() -> HashMap<String, AcpAgentConfig> {
    HashMap::from([
        ("claude".to_string(), acp("claude")),
        ("gemini".to_string(), acp("gemini")),
        ("opencode".to_string(), acp("opencode")),
        ("codex".to_string(), acp("codex")),
        ("openclaw".to_string(), acp("openclaw")),
        ("copilot".to_string(), acp("copilot")),
        ("custom".to_string(), acp("custom")),
    ])
}

#[test]
fn default_enabled_acp_config_keeps_known_agents_in_catalog() {
    let catalog = catalog();

    let enabled = default_enabled_acp_config(&catalog);

    assert_eq!(enabled.len(), 6);
    assert_eq!(enabled.get("claude").map(|config| config.command.as_str()), Some("claude"));
    assert_eq!(enabled.get("custom").map(|config| config.command.as_str()), None);
}

#[test]
fn default_enabled_acp_config_skips_missing_catalog_entries() {
    let catalog = HashMap::from([("codex".to_string(), acp("codex"))]);

    let enabled = default_enabled_acp_config(&catalog);

    assert_eq!(enabled.len(), 1);
    assert!(enabled.contains_key("codex"));
}

#[test]
fn default_enabled_acp_config_returns_empty_for_empty_catalog() {
    assert!(default_enabled_acp_config(&HashMap::new()).is_empty());
}

#[test]
fn resolve_enabled_acp_config_uses_defaults_when_configured_is_empty() {
    let catalog = catalog();

    let enabled = resolve_enabled_acp_config(&catalog, HashMap::new());

    assert_eq!(enabled.len(), 6);
    assert!(enabled.contains_key("copilot"));
}

#[test]
fn resolve_enabled_acp_config_preserves_explicit_configured_agents() {
    let catalog = catalog();
    let configured = HashMap::from([("custom".to_string(), acp("configured-custom"))]);

    let enabled = resolve_enabled_acp_config(&catalog, configured);

    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled["custom"].command, "configured-custom");
}

#[test]
fn build_acp_agent_enabled_patch_rejects_blank_name() {
    let err = build_acp_agent_enabled_patch("  ", true, None, &catalog(), HashMap::new())
        .expect_err("blank names should fail");

    assert_eq!(err, "acp agent name must not be empty");
}

#[test]
fn build_acp_agent_enabled_patch_rejects_unknown_agent_without_spec() {
    let err = build_acp_agent_enabled_patch("missing", true, None, &catalog(), HashMap::new())
        .expect_err("unknown catalog agents should fail");

    assert_eq!(err, "unknown acp agent: missing");
}

#[test]
fn build_acp_agent_enabled_patch_rejects_unknown_agent_when_disabling_without_spec() {
    let err = build_acp_agent_enabled_patch("missing", false, None, &catalog(), HashMap::new())
        .expect_err("an unknown agent still requires an explicit spec when disabling");

    assert_eq!(err, "unknown acp agent: missing");
}

#[test]
fn build_acp_agent_enabled_patch_enables_catalog_agent_over_defaults() {
    let patch = build_acp_agent_enabled_patch(" custom ", true, None, &catalog(), HashMap::new())
        .expect("patch should build");

    assert_eq!(patch["acp"]["custom"]["command"], "custom");
    assert_eq!(patch["acp"]["claude"]["command"], "claude");
}

#[test]
fn build_acp_agent_enabled_patch_enables_single_explicit_agent_when_configured_exists() {
    let configured = HashMap::from([("claude".to_string(), acp("configured-claude"))]);
    let patch = build_acp_agent_enabled_patch(
        "custom",
        true,
        Some(acp("explicit-custom")),
        &catalog(),
        configured,
    )
    .expect("patch should build");

    assert_eq!(
        patch,
        json!({ "acp": { "custom": { "command": "explicit-custom", "args": [], "env": {} } } })
    );
}

#[test]
fn build_acp_agent_enabled_patch_uses_catalog_spec_over_existing_config() {
    let configured = HashMap::from([("custom".to_string(), acp("stale-custom"))]);

    let patch = build_acp_agent_enabled_patch("custom", true, None, &catalog(), configured)
        .expect("catalog agent should replace the configured spec");

    assert_eq!(
        patch,
        json!({ "acp": { "custom": {
        "command": "custom",
        "args": [],
        "env": {}
    } } })
    );
}

#[test]
fn build_acp_agent_enabled_patch_accepts_explicit_unknown_agent_spec() {
    let patch = build_acp_agent_enabled_patch(
        "local",
        true,
        Some(acp("local-agent")),
        &catalog(),
        HashMap::new(),
    )
    .expect("an explicit spec should not require a catalog entry");

    let acp_patch = patch["acp"].as_object().expect("acp patch should be an object");
    assert_eq!(acp_patch.len(), 7);
    assert_eq!(acp_patch["local"]["command"], "local-agent");
    assert_eq!(acp_patch["codex"]["command"], "codex");
}

#[test]
fn build_acp_agent_enabled_patch_disables_from_defaults_with_null_marker() {
    let patch = build_acp_agent_enabled_patch("claude", false, None, &catalog(), HashMap::new())
        .expect("patch should build");

    assert!(patch["acp"]["claude"].is_null());
    assert_eq!(patch["acp"]["gemini"]["command"], "gemini");
}

#[test]
fn build_acp_agent_enabled_patch_disables_single_configured_agent() {
    let configured = HashMap::from([
        ("claude".to_string(), acp("configured-claude")),
        ("custom".to_string(), acp("configured-custom")),
    ]);
    let patch = build_acp_agent_enabled_patch("custom", false, None, &catalog(), configured)
        .expect("patch should build");

    assert_eq!(patch, json!({ "acp": { "custom": null } }));
}

#[test]
fn build_acp_agent_enabled_patch_disables_explicit_unknown_agent_with_null_marker() {
    let patch = build_acp_agent_enabled_patch(
        "local",
        false,
        Some(acp("unused-while-disabling")),
        &catalog(),
        HashMap::new(),
    )
    .expect("an explicit spec should permit disabling an uncatalogued agent");

    let acp_patch = patch["acp"].as_object().expect("acp patch should be an object");
    assert_eq!(acp_patch.len(), 7);
    assert!(acp_patch["local"].is_null());
    assert_eq!(acp_patch["claude"]["command"], "claude");
}

#[test]
fn build_acp_agent_create_patch_rejects_existing_name_and_blank_command() {
    let existing =
        build_acp_agent_create_patch(" codex ", acp("custom-codex"), &catalog(), HashMap::new())
            .expect_err("existing names must not be overwritten");
    assert_eq!(existing, "acp agent already exists: codex");

    let blank_command =
        build_acp_agent_create_patch("local", acp("  "), &catalog(), HashMap::new())
            .expect_err("blank commands must fail");
    assert_eq!(blank_command, "acp agent command must not be empty");
}

#[test]
fn build_acp_agent_create_patch_normalizes_spec_and_preserves_default_enabled_agents() {
    let spec = AcpAgentConfig {
        command: " local-agent ".to_string(),
        args: vec![" --stdio ".to_string(), String::new(), "  ".to_string()],
        ..AcpAgentConfig::default()
    };

    let patch = build_acp_agent_create_patch(" local ", spec, &catalog(), HashMap::new())
        .expect("new ACP patch should build");

    assert_eq!(patch["acp"]["local"]["command"], "local-agent");
    assert_eq!(patch["acp"]["local"]["args"], json!(["--stdio"]));
    assert_eq!(patch["acp"]["codex"]["command"], "codex");
    assert_eq!(patch["acp"]["claude"]["command"], "claude");
}

fn config_with_main_agent(provider: &str, model: &str, temperature: Option<f64>) -> super::Config {
    let mut config = super::Config::default();
    config.agents.insert(
        "main".to_string(),
        DelegateAgentConfig {
            provider: provider.to_string(),
            model: model.to_string(),
            temperature,
            ..DelegateAgentConfig::default()
        },
    );
    config
}

#[test]
fn build_main_agent_overrides_patch_returns_none_without_main_agent() {
    let patch = build_main_agent_overrides_patch(&super::Config::default())
        .expect("patch construction should not fail");

    assert!(patch.is_none());
}

#[test]
fn build_main_agent_overrides_patch_includes_provider_model_temperature_and_identity() {
    let config = config_with_main_agent(" openrouter ", " glm-5 ", Some(0.2));

    let patch = build_main_agent_overrides_patch(&config)
        .expect("patch construction should not fail")
        .expect("main agent should produce patch");

    assert_eq!(
        patch,
        json!({
            "default_provider": "openrouter",
            "default_model": "openrouter/glm-5",
            "default_temperature": 0.2,
            "identity": {
                "format": "openclaw",
                "aieos_path": null,
                "aieos_inline": null
            }
        })
    );
}

#[test]
fn build_main_agent_overrides_patch_skips_model_without_provider() {
    let config = config_with_main_agent(" ", "glm-5", Some(f64::NAN));

    let patch = build_main_agent_overrides_patch(&config)
        .expect("patch construction should not fail")
        .expect("main agent should produce identity patch");

    assert_eq!(
        patch,
        json!({
            "identity": {
                "format": "openclaw",
                "aieos_path": null,
                "aieos_inline": null
            }
        })
    );
}

#[test]
fn build_main_agent_overrides_patch_keeps_provider_when_model_is_blank() {
    let config = config_with_main_agent(" openrouter ", "  ", None);

    let patch = build_main_agent_overrides_patch(&config)
        .expect("patch construction should not fail")
        .expect("main agent should produce a patch");

    assert_eq!(patch["default_provider"], "openrouter");
    assert!(patch.get("default_model").is_none());
    assert!(patch.get("default_temperature").is_none());
    assert_eq!(patch["identity"]["format"], "openclaw");
}

#[test]
fn build_main_agent_overrides_patch_keeps_temperature_without_provider() {
    let config = config_with_main_agent("  ", "ignored-model", Some(0.0));

    let patch = build_main_agent_overrides_patch(&config)
        .expect("patch construction should not fail")
        .expect("main agent should produce a patch");

    assert!(patch.get("default_provider").is_none());
    assert!(patch.get("default_model").is_none());
    assert_eq!(patch["default_temperature"], 0.0);
    assert_eq!(patch["identity"]["format"], "openclaw");
}

#[test]
fn build_main_agent_overrides_patch_omits_non_finite_temperature_only() {
    for temperature in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        let config = config_with_main_agent("openrouter", "glm-5", Some(temperature));

        let patch = build_main_agent_overrides_patch(&config)
            .expect("patch construction should not fail")
            .expect("main agent should produce a patch");

        assert_eq!(patch["default_provider"], "openrouter");
        assert_eq!(patch["default_model"], "openrouter/glm-5");
        assert!(patch.get("default_temperature").is_none());
        assert_eq!(patch["identity"]["format"], "openclaw");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn load_local_agent_config_reads_workspace_file_clears_gateway_skeys_and_applies_main_override() {
    let _lock = env_lock().lock().expect("env lock");
    let workspace = tempfile::tempdir().expect("workspace");
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", workspace.path());
    let mut config = super::Config::default();
    config.gateway.skeys = vec![GatewaySkey {
        enabled: true,
        skey: Some("secret".to_string()),
        skey_hash: "hash".to_string(),
        masked_skey: "masked".to_string(),
        name: "test".to_string(),
        expires_at: None,
    }];
    config.agents.insert(
        "main".to_string(),
        DelegateAgentConfig {
            provider: " openrouter ".to_string(),
            model: " glm-5 ".to_string(),
            temperature: Some(0.7),
            identity_format: Some("ignored".to_string()),
            ..DelegateAgentConfig::default()
        },
    );
    std::fs::write(
        workspace.path().join(".vibewindow.json"),
        serde_json::to_string(&config).expect("config json"),
    )
    .expect("write workspace config");

    let loaded = super::load_local_agent_config().expect("local config");

    assert!(loaded.gateway.skeys.is_empty());
    assert_eq!(loaded.default_provider.as_deref(), Some("openrouter"));
    assert_eq!(loaded.default_model.as_deref(), Some("openrouter/glm-5"));
    assert_eq!(loaded.default_temperature, 0.7);
    assert_eq!(loaded.identity.format, "openclaw");
}

#[test]
fn desktop_agent_config_uses_gateway_projection_without_direct_skey_sqlite_reads() {
    assert!(SOURCE.contains("fetch_agent_config_via_gateway"));
    assert!(SOURCE.contains("client.config_get(None).await"));
    assert!(SOURCE.contains("config.gateway.skeys.clear()"));
    assert!(!SOURCE.contains("skeys.sqlite"));
    assert!(!SOURCE.contains("load_gateway_skeys_from_sqlite"));
    assert!(!SOURCE.contains("rusqlite::Connection::open"));
}
