use super::*;
use crate::id::{ProjectId, ProjectScope, TenantScope};
use serde_json::json;

#[test]
fn cc_data_02_dto_serializes_redacted_preview() {
    let target = DataManagementTarget {
        scope: ProjectScope::new(
            TenantScope::new("fixture-platform", "fixture-company").unwrap(),
            ProjectId::from("fixture-project"),
        )
        .unwrap(),
        target_host_id: "fixture-host".into(),
        principal_id: "fixture-principal".into(),
    };
    let preview = DataImportPreview {
        target,
        preview_id: DataPreviewId("preview-1".into()),
        revision: "revision-1".into(),
        source: DataSourceSummary {
            transfer_id: DataTransferId("transfer-1".into()),
            kind: DataSourceKind::CcSwitch,
            application_version: "4.0.25".into(),
            schema_version: 24,
            format_version: 1,
            format: DataArchiveFormat::Sql,
            digest_sha256: "a".repeat(64),
        },
        categories: vec![DataCategorySummary {
            category: DataCategory::Providers,
            selected: 2,
            eligible: 1,
            excluded: 1,
        }],
        conflicts: vec![DataConflictSummary {
            conflict_id: "conflict-1".into(),
            category: DataCategory::Providers,
            entry_id: "entry-1".into(),
            allowed_decisions: vec![DataConflictDecision::Skip, DataConflictDecision::KeepBoth],
        }],
        exclusions: vec![DataExclusionSummary {
            category: DataCategory::Providers,
            reason: DataExclusionReason::Credentials,
            count: 1,
        }],
    };
    let value = serde_json::to_value(&preview).unwrap();
    assert_eq!(value["source"]["schemaVersion"], 24);
    assert_eq!(value["exclusions"], json!([{
        "category": "providers", "reason": "credentials", "count": 1
    }]));
    assert_eq!(value["categories"], json!([{
        "category": "providers", "selected": 2, "eligible": 1, "excluded": 1
    }]));
    assert_eq!(serde_json::from_value::<DataImportPreview>(value.clone()).unwrap(), preview);

    // Source payloads cannot be injected into this response contract. No actual
    // source parser/redactor is exercised here; that belongs to later tasks.
    for field in ["apiKey", "rawPayload", "settingsConfig", "filePath"] {
        let mut injected = value.clone();
        injected["source"][field] = json!("synthetic-sensitive-value");
        assert!(serde_json::from_value::<DataImportPreview>(injected).is_err());
    }
    let serialized = serde_json::to_string(&preview).unwrap();
    assert!(!serialized.contains("synthetic-sensitive-value"));
}

#[test]
fn cc_data_02_dto_rejects_unknown_operation() {
    for operation in ["import", "export", "create_backup", "restore_backup"] {
        let parsed: DataOperation = serde_json::from_value(json!(operation)).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), json!(operation));
    }
    for operation in ["execute_sql", "shell", "unknown", "Import"] {
        assert!(serde_json::from_value::<DataOperation>(json!(operation)).is_err());
    }
    assert!(serde_json::from_value::<DataCategory>(json!("accounts")).is_err());
    assert!(serde_json::from_value::<DataConflictDecision>(json!("overwrite_all")).is_err());
    assert!(serde_json::from_value::<DataOperationResult>(json!({"kind": "unknown"})).is_err());
    let mut source = json!({
        "transferId": "transfer-1", "kind": "cc_switch", "applicationVersion": "4.0.25",
        "schemaVersion": 24, "formatVersion": 1, "format": "sql", "digestSha256": "a".repeat(64)
    });
    source["command"] = json!("execute_sql");
    assert!(serde_json::from_value::<DataSourceSummary>(source).is_err());
}
