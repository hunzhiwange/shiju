use super::*;
#[test]
fn aggregation_t41_signed_preview_expiry_and_subject_are_enforced() {
    use crate::gateway::api::handlers::aggregation_read_snapshot::{sign, verify_token};
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let payload = json!({"kind":"aggregation_preview_v1","principal":"writer","scope":scope("project"),
        "targetHostId":"host","app":"claude","revision":"revision","defaultProviderId":null,
        "catalogPolicy":"preserve","expiresAt":chrono::Utc::now().timestamp()-1});
    let bytes = serde_json::to_vec(&payload).unwrap();
    let expired = format!("{}.{}", URL_SAFE_NO_PAD.encode(&bytes), sign(&bytes).unwrap());
    assert_eq!(
        verify_token(&expired, "writer", &scope("project"), "host", AppId::Claude, "revision")
            .unwrap_err(),
        StatusCode::CONFLICT
    );
    let request = AggregationPreviewRequest {
        target_host_id: "host".into(),
        project_id: ProjectId::from("project"),
        default_provider_id: None,
        catalog_policy: CatalogPolicy::Preserve,
    };
    let token = crate::gateway::api::handlers::aggregation_read_snapshot::token(
        "writer",
        &scope("project"),
        "host",
        AppId::Claude,
        "revision",
        &request,
    )
    .unwrap();
    assert!(
        verify_token(&token, "writer", &scope("project"), "host", AppId::Claude, "revision")
            .is_ok()
    );
    assert!(
        verify_token(&token, "other", &scope("project"), "host", AppId::Claude, "revision")
            .is_err()
    );
}
#[tokio::test]
async fn aggregation_t41_cancellation_respects_original_publication_boundary() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    use vw_agent_apps::providers::operations::{ProviderOperation, publication};
    use vw_api_types::{id::TaskId, task::TaskStatus};
    for published in [false, true] {
        let operation =
            ProviderOperation::new(TaskId::from(uuid::Uuid::new_v4().to_string()), "cancel".into());
        let (ready, wait) = mpsc::channel();
        let (resume, receive) = mpsc::channel();
        let effect = Arc::new(AtomicBool::new(false));
        let output = effect.clone();
        operation
            .start(move || {
                if published {
                    publication().map_err(str::to_owned)?;
                }
                ready.send(()).unwrap();
                receive.recv().unwrap();
                publication().map_err(str::to_owned)?;
                output.store(true, Ordering::SeqCst);
                Ok(())
            })
            .unwrap();
        tokio::task::spawn_blocking(move || wait.recv().unwrap()).await.unwrap();
        let cancelled = operation.cancel();
        assert!(cancelled.cancellation_requested);
        resume.send(()).unwrap();
        for _ in 0..300 {
            if !matches!(operation.snapshot().status, TaskStatus::Queued | TaskStatus::Running) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(effect.load(Ordering::SeqCst), published);
        assert_eq!(
            operation.snapshot().status,
            if published { TaskStatus::Completed } else { TaskStatus::Cancelled }
        );
    }
}

#[tokio::test]
async fn aggregation_t41_concurrent_request_payload_changes_return_request_conflict() {
    let mut f = Fixture::new();
    f.prepare_native();
    let listener = f.listener();
    let first = f.publish_request().await;
    let (status, preview) = f
        .request(
            "POST",
            "/agent-apps/claude/aggregation/preview",
            Some(TOKEN),
            json!({"targetHostId":"host","projectId":"project","defaultProviderId":"alpha"}),
        )
        .await;
    assert_eq!(status, 200, "{preview}");
    let mut other = first.clone();
    other["previewToken"] = preview["previewToken"].clone();
    assert_ne!(first["previewToken"], other["previewToken"]);
    let (one, two) = tokio::join!(f.publish(first), f.publish(other));
    let (accepted, rejected) = if one.0 == 202 { (one, two) } else { (two, one) };
    assert_eq!(accepted.0, 202, "{}", accepted.1);
    assert_eq!(rejected.0, 409, "{}", rejected.1);
    assert_eq!(rejected.1["error"]["code"], "request_conflict");
    f.settled(accepted.1["operationId"].as_str().unwrap()).await;
    listener.stop().await.unwrap();
}
