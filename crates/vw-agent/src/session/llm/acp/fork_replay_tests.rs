use super::*;
use serde_json::json;

fn record() -> SessionRecord {
    serde_json::from_value(json!({
        "schema":"1", "vwacpRecordId":"record", "acpSessionId":"remote",
        "agentCommand":"peer", "cwd":"/workspace", "createdAt":"now", "lastUsedAt":"now",
        "lastSeq":0, "eventLog":{"active_path":"events", "segment_count":1,
            "max_segment_bytes":1024,"max_segments":2}, "messages":[], "updatedAt":"now",
        "cumulativeTokenUsage":{}, "requestTokenUsage":{}
    }))
    .unwrap()
}

#[test]
fn fork_acp_delivery_persists_uncertainty_and_success_with_exact_anchor() {
    let root = tempfile::tempdir().unwrap();
    let database = GlobalDatabase::for_home(root.path()).unwrap();
    let store = GatewaySettingsStore::new(database).unwrap();
    let mut state = ForkReplay {
        store: store.clone(),
        tenant: Default::default(),
        key: "scoped-fork".into(),
        delivery: None,
    };
    let mut record = record();
    assert!(state.needs_history());
    state.begin(&record).unwrap();
    let read = || -> Delivery {
        serde_json::from_str(
            &store.get(&Default::default(), NAMESPACE, "scoped-fork").unwrap().unwrap(),
        )
        .unwrap()
    };
    // A failure, cancellation or process exit before complete leaves durable uncertainty.
    assert!(!read().completed);
    record.acp_session_id = "changed".into();
    assert!(state.complete(&record).is_err());
    assert!(!read().completed);
    record.acp_session_id = "remote".into();
    record.last_prompt_at = Some("success".into());
    state.complete(&record).unwrap();
    assert!(read().completed);
    let delivery = Some(read());
    let reopened =
        ForkReplay { store, tenant: Default::default(), key: "scoped-fork".into(), delivery };
    assert!(!reopened.needs_history());
    assert!(reopened.validate_record(&record).is_ok());
    record.acp_session_id = "replacement-remote".into();
    assert!(reopened.validate_record(&record).is_err());
    record.acp_session_id = "remote".into();
    record.vwacp_record_id = "replacement".into();
    assert!(reopened.validate_record(&record).is_err());
}

#[test]
fn fork_acp_delivery_rejects_preexisting_untracked_prompts() {
    let root = tempfile::tempdir().unwrap();
    let state = ForkReplay {
        store: GatewaySettingsStore::new(GlobalDatabase::for_home(root.path()).unwrap()).unwrap(),
        tenant: Default::default(),
        key: "fork".into(),
        delivery: None,
    };
    let mut record = record();
    record.last_prompt_at = Some("previous".into());
    assert!(state.validate_record(&record).is_err());
}

#[tokio::test]
async fn fork_acp_empty_session_preserves_ephemeral_requests() {
    let context = crate::project::instance::Context {
        directory: "/workspace".into(),
        worktree: "/workspace".into(),
        project: serde_json::from_value(json!({
            "id":"project", "worktree":"/workspace", "time":{"created":0,"updated":0}, "sandboxes":[]
        })).unwrap(),
    };
    crate::project::instance::provide_context(context, async {
        assert!(ForkReplay::load("", "/workspace").await.unwrap().is_none());
    })
    .await;
}
