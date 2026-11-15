use super::AppInstallationReport;
use serde_json::json;

fn fixture() -> serde_json::Value {
    json!({"id":"instance-1", "appId":"codex", "targetHostId":"host-1",
        "environment":{"kind":"macos"}, "path":"/opt/bin/codex", "realPath":"/opt/lib/codex",
        "source":"homebrew", "isPathDefault":true, "version":"1.2.3",
        "runtimeStatus":{"status":"runnable"}})
}

#[test]
fn cc_apps_installation_roundtrip_preserves_each_environment_and_source() {
    let mut installs = Vec::new();
    for (index, (environment, source)) in [
        (json!({"kind":"macos"}), "homebrew"),
        (json!({"kind":"linux"}), "nvm"),
        (json!({"kind":"windows"}), "native"),
        (json!({"kind":"wsl", "distribution":"Ubuntu-24.04"}), "unknown"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut item = fixture();
        item["id"] = json!(format!("instance-{index}"));
        item["targetHostId"] = json!(format!("host-{index}"));
        item["environment"] = environment;
        item["source"] = json!(source);
        if index == 2 {
            item["path"] = json!(r"C:\Tools\codex.cmd");
            item["realPath"] = serde_json::Value::Null;
            item["version"] = serde_json::Value::Null;
            item["isPathDefault"] = json!(false);
        }
        installs.push(item);
    }
    let wire = json!({"installations":installs});
    let report: AppInstallationReport = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(report).unwrap(), wire);
}

#[test]
fn cc_apps_installation_roundtrip_rejects_unknown_or_incomplete_contracts() {
    for (field, invalid) in [
        ("appId", json!("grok")),
        ("source", json!("arbitrary-command")),
        ("environment", json!({"kind":"wsl"})),
        ("environment", json!({"kind":"remote"})),
        ("runtimeStatus", json!({"status":"exited_with_error"})),
        ("runtimeStatus", json!({"status":"probe_failed", "reason":"unknown"})),
    ] {
        let mut item = fixture();
        item[field] = invalid;
        assert!(
            serde_json::from_value::<AppInstallationReport>(json!({"installations":[item]}))
                .is_err(),
            "{field}"
        );
    }
    for field in ["id", "targetHostId", "environment", "source", "path"] {
        let mut item = fixture();
        item.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<AppInstallationReport>(json!({"installations":[item]}))
                .is_err()
        );
    }
}

#[test]
fn cc_apps_installation_roundtrip_distinguishes_execution_failure_from_probe_failure() {
    for status in [
        json!({"status":"exited_with_error", "exitCode":2}),
        json!({"status":"probe_failed", "reason":"spawn_failed"}),
        json!({"status":"probe_failed", "reason":"timed_out"}),
    ] {
        let mut item = fixture();
        item["runtimeStatus"] = status;
        let wire = json!({"installations":[item]});
        let report: AppInstallationReport = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(report).unwrap(), wire);
    }
    let report: AppInstallationReport =
        serde_json::from_value(json!({"installations":[]})).unwrap();
    assert!(report.installations.is_empty());
}
