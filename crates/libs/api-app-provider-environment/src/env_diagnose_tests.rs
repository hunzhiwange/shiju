use super::env_diagnose::*;
#[test]
fn cc04_env_diagnose_dto_rejects_paths_values_and_foreign_scope() {
    let valid = serde_json::json!({"targetHostId":"host"});
    assert!(serde_json::from_value::<EnvDiagnoseRequest>(valid.clone()).is_ok());
    for field in ["path", "line", "environment", "tenant", "value"] {
        let mut value = valid.clone();
        value[field] = serde_json::json!("unsafe");
        assert!(serde_json::from_value::<EnvDiagnoseRequest>(value).is_err());
    }
    let entry = EnvDiagnostic {
        variable: "OPENAI_API_KEY".into(),
        source: EnvSource::ProcessInherited,
        cleanup_available: false,
        cleanup: None,
    };
    let json = serde_json::to_value(entry).unwrap();
    assert_eq!(json["source"]["kind"], "process_inherited");
    assert_eq!(json["cleanupAvailable"], false);
    assert!(json.get("value").is_none());
}
