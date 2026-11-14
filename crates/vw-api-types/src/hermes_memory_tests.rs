use crate::hermes_memory::*;
use serde_json::json;

#[test]
fn hermes_wire_rejects_unknown_kind() {
    for kind in ["other", "MEMORY", "../USER.md", "", "memory.md"] {
        assert!(serde_json::from_value::<HermesMemoryKind>(json!(kind)).is_err());
        assert!(
            serde_json::from_value::<HermesMemoryResponse>(json!({
                "kind": kind, "content": "", "exists": false
            }))
            .is_err()
        );
    }
    for (kind, wire) in [(HermesMemoryKind::Memory, "memory"), (HermesMemoryKind::User, "user")] {
        assert_eq!(serde_json::to_value(kind).unwrap(), json!(wire));
        assert_eq!(serde_json::from_value::<HermesMemoryKind>(json!(wire)).unwrap(), kind);
    }
}

#[test]
fn hermes_wire_limits_and_enabled_round_trip() {
    let wire = json!({"memory": 2200, "user": 1375, "memoryEnabled": false, "userEnabled": true});
    let limits: HermesMemoryLimits = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(limits).unwrap(), wire);
    for enabled in [true, false] {
        let request = HermesMemoryEnabledRequest { enabled };
        assert_eq!(
            serde_json::from_value::<HermesMemoryEnabledRequest>(
                serde_json::to_value(&request).unwrap()
            )
            .unwrap(),
            request
        );
    }
    assert!(
        serde_json::from_value::<HermesMemoryEnabledRequest>(json!({"enabled": true, "memory": 1}))
            .is_err()
    );
}

#[test]
fn hermes_wire_memory_preserves_text_and_missing_state() {
    for exists in [true, false] {
        let response =
            HermesMemoryResponse { kind: HermesMemoryKind::User, content: String::new(), exists };
        assert_eq!(
            serde_json::from_value::<HermesMemoryResponse>(
                serde_json::to_value(&response).unwrap()
            )
            .unwrap(),
            response
        );
    }
    let request = HermesMemoryWriteRequest { content: "中文🦀\n".repeat(3000) };
    assert_eq!(
        serde_json::from_value::<HermesMemoryWriteRequest>(serde_json::to_value(&request).unwrap())
            .unwrap(),
        request
    );
}

#[test]
fn hermes_wire_model_snapshot_rejects_credentials_and_extensions() {
    let snapshot = json!({"default": "model", "provider": "provider", "contextLength": 200000, "maxTokens": 8192});
    assert_eq!(
        serde_json::to_value(
            serde_json::from_value::<HermesModelSnapshot>(snapshot.clone()).unwrap()
        )
        .unwrap(),
        snapshot
    );
    for key in ["api_key", "apiKey", "token", "extra", "baseUrl", "base_url"] {
        let mut unsafe_snapshot = snapshot.clone();
        unsafe_snapshot[key] = json!("sensitive");
        assert!(serde_json::from_value::<HermesModelSnapshot>(unsafe_snapshot).is_err());
    }
}

#[test]
fn hermes_wire_write_outcome_preserves_backup() {
    for wire in [json!({}), json!({"backupPath": "config.yaml.bak"})] {
        let outcome: HermesWriteOutcome = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(outcome).unwrap(), wire);
    }
}
