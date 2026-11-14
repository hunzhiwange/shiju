use super::openclaw_settings::*;
use serde_json::{json, Value};

#[test]
fn openclaw_wire_preserves_extra_fields() {
    let model = json!({"primary":"供应商/model", "fallbacks":["b","b"], "future":{"enabled":true}});
    let decoded: OpenClawDefaultModel = serde_json::from_value(model.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), model);

    let catalog = json!({"供应商/model":{"alias":"中文", "future":[1,true]}});
    let decoded: OpenClawModelCatalog = serde_json::from_value(catalog.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), catalog);

    let defaults = json!({"model":model,"models":catalog,"timeout":30,"contextTokens":4096,"future":{"x":null}});
    let decoded: OpenClawAgentsDefaults = serde_json::from_value(defaults.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), defaults);

    let tools = json!({"profile":"future-profile","allow":["read"],"deny":["exec"],"future":{"x":true}});
    let decoded: OpenClawToolsConfig = serde_json::from_value(tools.clone()).unwrap();
    assert_eq!(decoded.profile.as_deref(), Some("future-profile"));
    assert_eq!(serde_json::to_value(decoded).unwrap(), tools);
}

#[test]
fn openclaw_wire_preserves_extra_fields_env_and_absence() {
    let env = json!({"vars":{"EXAMPLE_TOKEN":"fixture"},"shellEnv":{"enabled":true},"future":[null,1]});
    let decoded: OpenClawEnvConfig = serde_json::from_value(env.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), env);
    for invalid in [Value::Null, json!([]), json!("not an object")] {
        assert!(serde_json::from_value::<OpenClawEnvConfig>(invalid.clone()).is_err());
        assert!(serde_json::from_value::<OpenClawAgentsDefaults>(invalid).is_err());
    }
    let missing: Option<OpenClawEnvConfig> = serde_json::from_value(Value::Null).unwrap();
    assert!(missing.is_none());
    let empty: Option<OpenClawEnvConfig> = serde_json::from_value(json!({})).unwrap();
    assert!(empty.unwrap().0.is_empty());
}

#[test]
fn openclaw_wire_preserves_extra_fields_write_warnings() {
    let outcome = json!({"backupPath":"openclaw.json.bak","warnings":[{
        "code":"legacy_agents_timeout","message":"Deprecated timeout field.","path":"agents.defaults.timeout"
    }]});
    let decoded: OpenClawWriteOutcome = serde_json::from_value(outcome.clone()).unwrap();
    assert_eq!(decoded.warnings.len(), 1);
    assert_eq!(serde_json::to_value(decoded).unwrap(), outcome);
    assert!(serde_json::from_value::<OpenClawWriteOutcome>(json!({})).is_err());
    assert!(serde_json::from_value::<OpenClawWriteOutcome>(json!({"warnings":[],"command":"arbitrary"})).is_err());
    let warning: OpenClawHealthWarning = serde_json::from_value(json!({"code":"config_parse_failed","message":"Cannot parse configuration."})).unwrap();
    assert!(warning.path.is_none());
}
