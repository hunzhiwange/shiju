use super::*;
use serde_json::json;
#[test]
fn aggregation_t41_publish_dto_rejects_untrusted_configuration_and_requires_confirmation() {
    let request = json!({"targetHostId":"host","projectId":"project","expectedRevision":"revision",
        "requestId":"request","previewToken":"signed","confirmed":true});
    assert!(serde_json::from_value::<AggregationPublishRequest>(request.clone()).is_ok());
    for field in ["path", "config", "token", "defaultProviderId", "catalogPolicy"] {
        let mut value = request.clone();
        value[field] = json!("injected");
        assert!(serde_json::from_value::<AggregationPublishRequest>(value).is_err());
    }
    let mut missing = request;
    missing.as_object_mut().unwrap().remove("confirmed");
    assert!(serde_json::from_value::<AggregationPublishRequest>(missing).is_err());
}

#[test]
fn aggregation_t41_operation_progress_preserves_legacy_wire_and_typed_failure() {
    use crate::{app_providers::operations::*, task::TaskStatus};
    let mut progress = ProviderOperationProgress {
        operation_id: "operation".into(),
        request_id: "request".into(),
        status: TaskStatus::Failed,
        phase: ProviderOperationPhase::Settling,
        completed: 0,
        total: 1,
        cancellation_requested: true,
        editor_outcome: None,
        aggregation_outcome: Some(AggregationOutcome::Failed {
            status: 502,
            code: AggregationErrorCode::TargetFailed,
            recovery: RecoveryState::Pending,
        }),
    };
    let value = serde_json::to_value(&progress).unwrap();
    assert_eq!(value["aggregationOutcome"]["kind"], "failed");
    assert_eq!(value["aggregationOutcome"]["status"], 502);
    assert_eq!(value["aggregationOutcome"]["code"], "target_failed");
    assert_eq!(serde_json::from_value::<ProviderOperationProgress>(value).unwrap(), progress);
    progress.aggregation_outcome = None;
    let legacy = serde_json::to_value(&progress).unwrap();
    assert!(legacy.get("aggregationOutcome").is_none());
    assert_eq!(serde_json::from_value::<ProviderOperationProgress>(legacy).unwrap(), progress);
}
