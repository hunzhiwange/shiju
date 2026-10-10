//! Config 工具行为测试。
//!
//! 覆盖完整快照读取、单项配置读写和结构化 call 结果。

use super::{
    Args, ConfigResponse, ConfigTool, SettingKind, SettingSpec, coerce_bool, coerce_integer,
    coerce_number, coerce_string, format_setting_value, normalize_setting_value, read_nested_value,
    read_setting_value, setting_spec, supported_settings_description, value_label,
    write_nested_value,
};
use crate::app::agent::config::Config;
use crate::app::agent::config::schema::ConfigExt;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::app::agent::tools::Tool;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;

async fn test_config() -> (TempDir, Arc<Config>) {
    let tmp = TempDir::new().expect("tempdir should create");
    let mut config = Config::default();
    config.workspace_dir = tmp.path().join("workspace");
    config.config_path = tmp.path().join("vibewindow.json");
    // 每个测试使用独立配置文件，避免设置写入影响其他用例。
    std::fs::create_dir_all(&config.workspace_dir).expect("workspace should create");
    config.save().await.expect("config file should write");
    (tmp, Arc::new(config))
}

async fn persisted_config(config: &Config) -> Config {
    crate::app::agent::config::get_project_without_env(&config.workspace_dir)
        .await
        .expect("persisted config should load")
}

fn parse_output(result: &crate::app::agent::tools::ToolResult) -> serde_json::Value {
    serde_json::from_str(&result.output).expect("config output should be json")
}

#[test]
fn config_args_default_to_empty_request() {
    let args: Args = serde_json::from_value(json!({})).expect("empty args should deserialize");

    assert!(args.setting.is_none());
    assert!(args.value.is_none());
}

#[test]
fn setting_lookup_accepts_trimmed_case_insensitive_aliases_and_rejects_unknown() {
    assert_eq!(setting_spec("  DEFAULT_PROVIDER  ").map(|spec| spec.key), Some("defaultProvider"));
    assert_eq!(
        setting_spec("App_UI.System_Settings.Terminal_Theme").map(|spec| spec.key),
        Some("appUi.terminalTheme")
    );
    assert!(setting_spec("browser.missing").is_none());
}

#[test]
fn nested_value_helpers_read_write_remove_and_replace_non_objects() {
    let mut value = json!(false);
    write_nested_value(&mut value, &["outer", "inner"], json!(7));

    assert_eq!(read_nested_value(&value, &["outer", "inner"]), Some(&json!(7)));
    assert!(read_nested_value(&value, &["outer", "missing"]).is_none());

    write_nested_value(&mut value, &["outer", "inner"], serde_json::Value::Null);
    assert!(read_nested_value(&value, &["outer", "inner"]).is_none());

    write_nested_value(&mut value, &[], json!(["replacement"]));
    assert_eq!(value, json!(["replacement"]));
}

#[test]
fn primitive_coercers_accept_supported_inputs_and_reject_invalid_values() {
    assert_eq!(coerce_bool(json!(true), "flag").unwrap(), json!(true));
    assert_eq!(coerce_bool(json!(" FALSE "), "flag").unwrap(), json!(false));
    assert_eq!(
        coerce_bool(json!("yes"), "flag").unwrap_err().to_string(),
        "flag requires true or false"
    );
    assert!(coerce_bool(json!(1), "flag").is_err());

    assert_eq!(coerce_integer(json!(7), "count").unwrap(), json!(7));
    assert_eq!(coerce_integer(json!(" 9 "), "count").unwrap(), json!(9));
    assert!(coerce_integer(json!(-1), "count").is_err());
    assert!(coerce_integer(json!(1.5), "count").is_err());
    assert!(coerce_integer(json!(true), "count").is_err());

    assert_eq!(coerce_number(json!(1.25), "ratio").unwrap(), json!(1.25));
    assert_eq!(coerce_number(json!(" 0.5 "), "ratio").unwrap(), json!(0.5));
    assert!(coerce_number(json!("warm"), "ratio").is_err());
    assert!(coerce_number(json!(false), "ratio").is_err());

    assert_eq!(coerce_string(json!(" shell "), "name").unwrap(), json!("shell"));
    assert!(coerce_string(json!("  "), "name").is_err());
    assert!(coerce_string(json!(3), "name").is_err());
}

#[test]
fn setting_normalization_handles_defaults_options_and_option_type_errors() {
    let provider = setting_spec("defaultProvider").unwrap();
    let temperature = setting_spec("defaultTemperature").unwrap();
    let autonomy = setting_spec("autonomy.level").unwrap();

    assert_eq!(normalize_setting_value(provider, json!(" DEFAULT ")).unwrap(), json!(null));
    assert_eq!(
        normalize_setting_value(temperature, json!("default")).unwrap(),
        json!(Config::default().default_temperature)
    );
    assert_eq!(normalize_setting_value(autonomy, json!("full")).unwrap(), json!("full"));
    assert!(
        normalize_setting_value(autonomy, json!("owner"))
            .unwrap_err()
            .to_string()
            .contains("Options: read_only, supervised, full")
    );

    let boolean_with_options = SettingSpec {
        key: "customFlag",
        aliases: &[],
        path: &["custom_flag"],
        kind: SettingKind::Boolean,
        description: "test-only option type guard",
        options: &["true"],
    };
    assert_eq!(
        normalize_setting_value(&boolean_with_options, json!(true)).unwrap_err().to_string(),
        "customFlag requires one of: true"
    );
}

#[test]
fn setting_formatting_and_labels_cover_defaults_scalars_and_compounds() {
    let provider = setting_spec("defaultProvider").unwrap();
    let browser = setting_spec("browser.enabled").unwrap();
    let mut config = Config::default();
    config.default_provider = None;

    assert_eq!(format_setting_value(provider, json!(null)), json!("default"));
    assert_eq!(format_setting_value(provider, json!("openai")), json!("openai"));
    assert_eq!(format_setting_value(browser, json!(null)), json!(null));
    assert_eq!(read_setting_value(&config, provider).unwrap(), json!("default"));

    assert_eq!(value_label(&json!(null)), "default");
    assert_eq!(value_label(&json!("plain")), "plain");
    assert_eq!(value_label(&json!(true)), "true");
    assert_eq!(value_label(&json!(12.5)), "12.5");
    assert_eq!(value_label(&json!(["a", 2])), r#"["a",2]"#);
}

#[test]
fn supported_settings_description_reports_types_options_and_descriptions() {
    let description = supported_settings_description();

    assert!(
        description.contains("- browser.enabled: true/false - Whether browser_open is enabled.")
    );
    assert!(
        description
            .contains("- agents.main.maxIterations: integer - Maximum number of tool iterations allowed per message.")
    );
    assert!(description.contains("- defaultTemperature: number - Default temperature."));
    assert!(
        description.contains(
            "- autonomy.level: \"read_only\", \"supervised\", \"full\" - Autonomy level."
        )
    );
    assert!(description.contains("- defaultModel: string - Default model name."));
}

#[test]
fn config_response_formats_success_failure_and_fallback_messages() {
    let get = ConfigResponse {
        success: true,
        operation: Some("get"),
        setting: Some("browser.enabled".to_string()),
        value: Some(json!(true)),
        previous_value: None,
        new_value: None,
        error: None,
    };
    assert_eq!(get.summary(), "browser.enabled = true");
    assert_eq!(get.model_text(), "browser.enabled = true");
    assert_eq!(get.to_value().unwrap()["success"], json!(true));
    assert!(get.to_legacy_result().unwrap().output.contains("browser.enabled"));

    let set_without_details = ConfigResponse {
        success: true,
        operation: Some("set"),
        setting: None,
        value: None,
        previous_value: None,
        new_value: None,
        error: None,
    };
    assert_eq!(set_without_details.summary(), "配置已更新");
    assert_eq!(set_without_details.model_text(), "Configuration updated");

    let failed = ConfigResponse {
        success: false,
        operation: None,
        setting: None,
        value: None,
        previous_value: None,
        new_value: None,
        error: None,
    };
    assert_eq!(failed.summary(), "配置操作失败");
    assert_eq!(failed.model_text(), "Config failed");
    assert_eq!(failed.to_legacy_result().unwrap().error, None);
}

#[test]
fn config_response_summaries_cover_detailed_and_overview_variants() {
    let overview = ConfigResponse {
        success: true,
        operation: Some("get"),
        setting: None,
        value: None,
        previous_value: None,
        new_value: None,
        error: None,
    };
    assert_eq!(overview.summary(), "当前配置概览");
    assert_eq!(overview.model_text(), "Retrieved current configuration overview");

    let updated = ConfigResponse {
        success: true,
        operation: Some("set"),
        setting: Some("browser.enabled".to_string()),
        value: None,
        previous_value: Some(json!(false)),
        new_value: Some(json!(true)),
        error: None,
    };
    assert_eq!(updated.summary(), "browser.enabled -> true");
    assert_eq!(updated.model_text(), "Set browser.enabled to true");

    let generic = ConfigResponse {
        success: true,
        operation: None,
        setting: None,
        value: None,
        previous_value: None,
        new_value: None,
        error: None,
    };
    assert_eq!(generic.summary(), "配置");
    assert_eq!(generic.model_text(), "Configuration");

    let failed = ConfigResponse {
        success: false,
        operation: Some("set"),
        setting: Some("browser.enabled".to_string()),
        value: None,
        previous_value: None,
        new_value: None,
        error: Some("blocked".to_string()),
    };
    assert_eq!(failed.summary(), "blocked");
    assert_eq!(failed.model_text(), "blocked");
    assert_eq!(failed.to_legacy_result().unwrap().error.as_deref(), Some("blocked"));
}

#[tokio::test]
async fn config_returns_snapshot_for_all_section() {
    let (_tmp, config) = test_config().await;
    let tool = ConfigTool::new(config, Arc::new(SecurityPolicy::default()));
    let result = tool.execute(json!({})).await.expect("config should succeed");
    let output: serde_json::Value =
        serde_json::from_str(&result.output).expect("snapshot output should be json");

    assert!(result.success);
    assert_eq!(output["operation"].as_str(), Some("get"));
    assert!(output["value"]["workspace_dir"].is_string());
    assert!(output["value"]["config_path"].is_string());
}

#[tokio::test]
async fn config_reads_supported_setting() {
    let (_tmp, config) = test_config().await;
    let tool = ConfigTool::new(config, Arc::new(SecurityPolicy::default()));

    let result = tool
        .execute(json!({
            "setting": "browser.enabled"
        }))
        .await
        .expect("config get should succeed");

    let output: serde_json::Value =
        serde_json::from_str(&result.output).expect("config output should be json");
    assert!(result.success);
    assert_eq!(output["operation"].as_str(), Some("get"));
    assert_eq!(output["setting"].as_str(), Some("browser.enabled"));
    assert_eq!(output["value"], json!(false));
}

#[tokio::test]
async fn config_sets_supported_setting_and_persists() {
    let (_tmp, config) = test_config().await;
    let tool = ConfigTool::new(config.clone(), Arc::new(SecurityPolicy::default()));

    let result = tool
        .execute(json!({
            "setting": "browser.enabled",
            "value": true
        }))
        .await
        .expect("config set should succeed");

    let output: serde_json::Value =
        serde_json::from_str(&result.output).expect("config output should be json");
    let saved = persisted_config(&config).await;

    assert!(result.success);
    assert_eq!(output["operation"].as_str(), Some("set"));
    assert_eq!(output["setting"].as_str(), Some("browser.enabled"));
    assert_eq!(output["previousValue"], json!(false));
    assert_eq!(output["newValue"], json!(true));
    assert!(saved.browser.enabled);
}

#[tokio::test]
async fn config_call_returns_structured_result_for_setting_reads() {
    let (_tmp, config) = test_config().await;
    let tool = ConfigTool::new(config, Arc::new(SecurityPolicy::default()));

    let result = tool
        .call(json!({
            "setting": "defaultTemperature"
        }))
        .await
        .expect("structured call should succeed");

    assert_eq!(result.data["operation"].as_str(), Some("get"));
    assert_eq!(result.data["setting"].as_str(), Some("defaultTemperature"));
    assert!(result.render_hint.is_some());
}

#[test]
fn config_spec_exposes_strict_non_concurrent_write_contract() {
    let tool = ConfigTool::new(Arc::new(Config::default()), Arc::new(SecurityPolicy::default()));
    let spec = tool.spec();

    assert_eq!(spec.id, "Config");
    assert_eq!(spec.display_name, "Config");
    assert_eq!(spec.aliases, vec!["config".to_string()]);
    assert!(!spec.read_only);
    assert!(!spec.destructive);
    assert!(!spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
    assert_eq!(spec.input_schema["additionalProperties"], json!(false));
    assert!(spec.input_schema["properties"].get("section").is_none());
    assert!(
        spec.input_schema["properties"]["setting"]["enum"]
            .as_array()
            .is_some_and(|settings| settings.contains(&json!("webSearch.provider")))
    );
}

#[tokio::test]
async fn config_canonicalizes_alias_reads_and_reports_unknown_settings() {
    let (_tmp, config) = test_config().await;
    let tool = ConfigTool::new(config, Arc::new(SecurityPolicy::default()));

    let alias_result = tool
        .execute(json!({ "setting": "  BROWSER.ENABLED  " }))
        .await
        .expect("alias read should succeed");
    let alias_output = parse_output(&alias_result);
    assert!(alias_result.success);
    assert_eq!(alias_output["setting"], json!("browser.enabled"));
    assert_eq!(alias_output["value"], json!(false));

    let unknown_result = tool
        .execute(json!({ "setting": " browser.missing " }))
        .await
        .expect("unknown setting should produce a structured response");
    let unknown_output = parse_output(&unknown_result);
    assert!(!unknown_result.success);
    assert_eq!(unknown_output["operation"], json!("get"));
    assert_eq!(unknown_output["setting"], json!("browser.missing"));
    assert_eq!(unknown_output["error"], json!("Unknown setting: \"browser.missing\""));
}

#[tokio::test]
async fn config_rejects_invalid_setting_values_without_persisting_changes() {
    let (_tmp, config) = test_config().await;
    let original = serde_json::to_value(persisted_config(&config).await)
        .expect("persisted config should serialize");
    let tool = ConfigTool::new(config.clone(), Arc::new(SecurityPolicy::default()));

    let wrong_type = tool
        .execute(json!({ "setting": "browser.enabled", "value": 1 }))
        .await
        .expect("wrong type should produce a structured response");
    let wrong_type_output = parse_output(&wrong_type);
    assert!(!wrong_type.success);
    assert_eq!(wrong_type_output["previousValue"], json!(false));
    assert_eq!(wrong_type_output["error"], json!("browser.enabled requires true or false"));

    let invalid_option = tool
        .execute(json!({ "setting": "autonomy.level", "value": "owner" }))
        .await
        .expect("invalid option should produce a structured response");
    let invalid_option_output = parse_output(&invalid_option);
    assert!(!invalid_option.success);
    assert!(
        invalid_option_output["error"]
            .as_str()
            .is_some_and(|error| error.contains("Options: read_only, supervised, full"))
    );

    let after = serde_json::to_value(persisted_config(&config).await)
        .expect("persisted config should serialize");
    assert_eq!(after, original);
}

#[tokio::test]
async fn config_blocks_writes_in_read_only_mode_without_persisting_changes() {
    let (_tmp, config) = test_config().await;
    let original = serde_json::to_value(persisted_config(&config).await)
        .expect("persisted config should serialize");
    let mut security = SecurityPolicy::default();
    security.autonomy = AutonomyLevel::ReadOnly;
    let tool = ConfigTool::new(config.clone(), Arc::new(security));

    let result = tool
        .execute(json!({ "setting": "browser.enabled", "value": true }))
        .await
        .expect("blocked write should produce a structured response");
    let output = parse_output(&result);

    assert!(!result.success);
    assert_eq!(output["operation"], json!("set"));
    assert_eq!(output["setting"], json!("browser.enabled"));
    assert_eq!(output["error"], json!("Action blocked: autonomy is read-only"));
    assert_eq!(
        serde_json::to_value(persisted_config(&config).await)
            .expect("persisted config should serialize"),
        original
    );
}

#[tokio::test]
async fn config_blocks_writes_when_action_budget_is_exhausted() {
    let (_tmp, config) = test_config().await;
    let mut security = SecurityPolicy::default();
    security.max_actions_per_hour = 0;
    let tool = ConfigTool::new(config, Arc::new(security));

    let result = tool
        .execute(json!({ "setting": "browser.enabled", "value": true }))
        .await
        .expect("rate-limited write should produce a structured response");
    let output = parse_output(&result);

    assert!(!result.success);
    assert_eq!(output["error"], json!("Action blocked: rate limit exceeded"));
    assert!(output.get("previousValue").is_none());
    assert!(output.get("newValue").is_none());
}

#[tokio::test]
async fn config_resets_optional_setting_and_coerces_integer_string() {
    let (_tmp, config) = test_config().await;
    let tool = ConfigTool::new(config.clone(), Arc::new(SecurityPolicy::default()));

    let reset = tool
        .execute(json!({ "setting": "defaultProvider", "value": " DEFAULT " }))
        .await
        .expect("default provider reset should succeed");
    let reset_output = parse_output(&reset);
    assert!(reset.success);
    assert_eq!(reset_output["newValue"], json!("openrouter"));

    let integer = tool
        .execute(json!({ "setting": "agents.main.max_iterations", "value": " 7 " }))
        .await
        .expect("integer string should succeed");
    let integer_output = parse_output(&integer);
    assert!(integer.success);
    assert_eq!(integer_output["setting"], json!("agents.main.maxIterations"));
    assert_eq!(integer_output["newValue"], json!(7));

    let saved = persisted_config(&config).await;
    assert_eq!(saved.default_provider.as_deref(), Some("openrouter"));
    assert_eq!(saved.agents["main"].max_iterations, 7);
}

#[tokio::test]
async fn config_empty_setting_returns_snapshot_and_rejects_unknown_fields() {
    let (_tmp, config) = test_config().await;
    let tool = ConfigTool::new(config, Arc::new(SecurityPolicy::default()));

    let all = tool
        .execute(json!({ "setting": "   " }))
        .await
        .expect("blank setting should return the full snapshot");
    let all_output = parse_output(&all);
    assert!(all.success);
    assert!(all_output["setting"].is_null());
    assert!(all_output["value"]["workspace_dir"].is_string());

    let error = tool
        .execute(json!({ "section": "credentials" }))
        .await
        .expect_err("unknown fields should fail");
    assert!(error.to_string().contains("unknown field `section`"));
}

#[tokio::test]
async fn config_call_preserves_structured_failure_details() {
    let (_tmp, config) = test_config().await;
    let tool = ConfigTool::new(config, Arc::new(SecurityPolicy::default()));

    let result = tool
        .call(json!({ "setting": "unknown.setting" }))
        .await
        .expect("unknown setting should return structured call data");

    assert_eq!(result.data["success"], json!(false));
    assert_eq!(result.model_result, json!("Unknown setting: \"unknown.setting\""));
    assert_eq!(result.content_blocks.len(), 1);
    assert_eq!(result.telemetry.as_ref().map(|telemetry| telemetry.success), Some(false));
    let render_hint = result.render_hint.expect("render hint should be present");
    assert_eq!(render_hint.summary.as_deref(), Some("Unknown setting: \"unknown.setting\""));
    assert_eq!(render_hint.metadata["operation"], json!("get"));
    assert_eq!(render_hint.metadata["success"], json!(false));
}
