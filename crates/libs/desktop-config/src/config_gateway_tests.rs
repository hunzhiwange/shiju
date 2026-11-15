use super::*;
use crate::access::apply_main_agent_overrides;
use config_agent::AgentDefinitionConfig;
use vw_config_types::config::Config;
use serde_json::json;
#[test]
fn normalize_gateway_host_trims_and_defaults_empty_values() {
    assert_eq!(normalize_gateway_host(" gateway.internal "), "gateway.internal");
    assert_eq!(normalize_gateway_host(" \t "), "127.0.0.1");
}

#[test]
fn apply_main_agent_overrides_uses_non_empty_main_agent_fields() {
    let mut cfg = Config::default();
    let mut main = AgentDefinitionConfig {
        provider: " openai ".to_string(),
        model: " gpt-5 ".to_string(),
        temperature: Some(0.2),
        identity_format: Some("custom".to_string()),
        ..AgentDefinitionConfig::default()
    };
    cfg.agents.insert("main".to_string(), main.clone());

    apply_main_agent_overrides(&mut cfg);

    assert_eq!(cfg.default_provider.as_deref(), Some("openai"));
    assert_eq!(cfg.default_model.as_deref(), Some("openai/gpt-5"));
    assert_eq!(cfg.default_temperature, 0.2);
    assert_eq!(cfg.identity.format, "openclaw");

    main.provider = " ".to_string();
    main.model = "ignored".to_string();
    main.temperature = None;
    main.identity_format = None;
    cfg.default_provider = Some("previous".to_string());
    cfg.default_model = Some("previous/model".to_string());
    cfg.default_temperature = 0.9;
    cfg.agents.insert("main".to_string(), main);

    apply_main_agent_overrides(&mut cfg);

    assert_eq!(cfg.default_provider.as_deref(), Some("previous"));
    assert_eq!(cfg.default_model.as_deref(), Some("previous/model"));
    assert_eq!(cfg.default_temperature, 0.9);
}

#[test]
fn apply_main_agent_overrides_noops_without_main_agent() {
    let mut cfg = Config::default();
    cfg.default_provider = Some("kept".to_string());
    cfg.default_model = Some("kept/model".to_string());

    apply_main_agent_overrides(&mut cfg);

    assert_eq!(cfg.default_provider.as_deref(), Some("kept"));
    assert_eq!(cfg.default_model.as_deref(), Some("kept/model"));
}

#[test]
fn apply_main_agent_overrides_updates_provider_without_erasing_model_for_blank_model() {
    let mut cfg = Config::default();
    cfg.default_provider = Some("old-provider".to_string());
    cfg.default_model = Some("old-provider/old-model".to_string());
    cfg.agents.insert(
        "main".to_string(),
        AgentDefinitionConfig {
            provider: " new-provider ".to_string(),
            model: " \t ".to_string(),
            ..AgentDefinitionConfig::default()
        },
    );

    apply_main_agent_overrides(&mut cfg);

    assert_eq!(cfg.default_provider.as_deref(), Some("new-provider"));
    assert_eq!(cfg.default_model.as_deref(), Some("old-provider/old-model"));
}

#[test]
fn apply_main_agent_overrides_applies_independent_fields_for_blank_provider() {
    let mut cfg = Config::default();
    cfg.default_provider = Some("kept-provider".to_string());
    cfg.default_model = Some("kept-provider/kept-model".to_string());
    cfg.identity.format = "aieos".to_string();
    cfg.identity.aieos_path = Some("identity.json".to_string());
    cfg.identity.aieos_inline = Some("inline identity".to_string());
    cfg.agents.insert(
        "main".to_string(),
        AgentDefinitionConfig {
            provider: " ".to_string(),
            model: "ignored".to_string(),
            temperature: Some(0.45),
            identity_format: Some("aieos".to_string()),
            ..AgentDefinitionConfig::default()
        },
    );

    apply_main_agent_overrides(&mut cfg);

    assert_eq!(cfg.default_provider.as_deref(), Some("kept-provider"));
    assert_eq!(cfg.default_model.as_deref(), Some("kept-provider/kept-model"));
    assert_eq!(cfg.default_temperature, 0.45);
    assert_eq!(cfg.identity.format, "openclaw");
    assert!(cfg.identity.aieos_path.is_none());
    assert!(cfg.identity.aieos_inline.is_none());
}

#[test]
fn set_config_value_at_path_replaces_root_when_path_is_empty() {
    let mut root = json!({"old": true});

    set_config_value_at_path(&mut root, &[], json!(["new"]));

    assert_eq!(root, json!(["new"]));
}

#[test]
fn set_config_value_at_path_creates_missing_objects_and_overwrites_scalars() {
    let mut root = json!({"app_ui": "not-object"});

    set_config_value_at_path(
        &mut root,
        &["app_ui", "system_settings", "gateway_client", "host"],
        json!("gateway.internal"),
    );

    assert_eq!(
        root,
        json!({"app_ui": {"system_settings": {"gateway_client": {"host": "gateway.internal"}}}})
    );
}

#[test]
fn set_config_value_at_path_replaces_non_object_root_with_object() {
    let mut root = json!(["not", "an", "object"]);

    set_config_value_at_path(&mut root, &["gateway"], json!("enabled"));

    assert_eq!(root, json!({"gateway": "enabled"}));
}

#[test]
fn set_config_value_at_path_preserves_existing_objects_and_siblings() {
    let mut root = json!({
        "app_ui": {
            "theme": "dark",
            "system_settings": {
                "gateway_client": {"host": "old", "port": 42618}
            }
        }
    });

    set_config_value_at_path(
        &mut root,
        &["app_ui", "system_settings", "gateway_client", "host"],
        json!("new"),
    );

    assert_eq!(root["app_ui"]["theme"], json!("dark"));
    assert_eq!(root["app_ui"]["system_settings"]["gateway_client"]["host"], json!("new"));
    assert_eq!(root["app_ui"]["system_settings"]["gateway_client"]["port"], json!(42618));
}

#[test]
fn server_config_unreachable_error_omits_blank_details() {
    assert_eq!(
        server_config_unreachable_error(" \t "),
        "服务端配置不可达，请检查 Gateway 连接状态。"
    );
    assert_eq!(
        server_config_unreachable_error("network down"),
        "服务端配置不可达，请检查 Gateway 连接状态。network down"
    );
}

#[test]
fn should_attempt_tools_list_request_allows_optional_skey() {
    assert!(should_attempt_tools_list_request(&GatewayEndpoint::new("gateway.internal", 1)));
    assert!(should_attempt_tools_list_request(&GatewayEndpoint::new("127.0.0.1", 1)));
}

#[test]
fn normalize_tool_ids_sorts_and_dedups() {
    let tools = normalize_tool_ids(vec![
        "bash".to_string(),
        String::new(),
        "file_read".to_string(),
        "bash".to_string(),
        "mcp__rustcodegraph__search".to_string(),
    ]);

    assert_eq!(tools, vec!["bash".to_string(), "file_read".to_string()]);
}

#[test]
fn normalize_tool_ids_drops_blank_entries_without_trimming_non_blank_ids() {
    let tools =
        normalize_tool_ids(vec![" zed ".to_string(), " \t ".to_string(), "alpha".to_string()]);

    assert_eq!(tools, vec![" zed ".to_string(), "alpha".to_string()]);
}

#[test]
fn run_gateway_call_runs_without_existing_runtime() {
    let value = run_gateway_call(async { Ok::<_, String>(42) }).expect("future should succeed");

    assert_eq!(value, 42);
}

#[test]
fn run_gateway_call_runs_inside_current_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("current-thread runtime should build");

    let value = runtime.block_on(async {
        run_gateway_call(async { Ok::<_, String>("ok".to_string()) })
            .expect("future should run on blocking runtime")
    });

    assert_eq!(value, "ok");
}

#[test]
fn run_gateway_call_runs_inside_multithread_runtime() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("multi-thread runtime should build");

    let value = runtime.block_on(async {
        run_gateway_call(async { Ok::<_, String>("ok".to_string()) })
            .expect("future should run through block_in_place")
    });

    assert_eq!(value, "ok");
}

#[test]
fn run_gateway_call_returns_future_error() {
    let err = run_gateway_call(async { Err::<(), _>("boom".to_string()) })
        .expect_err("future error should be returned");

    assert_eq!(err, "boom");
}
