use serde_json::{Value, json};
use std::{path::Path, future::Future};
/// Caller invokes the real workflow tool provider, including its native process boundary.
pub async fn errors_and_unknown_states<F, Fut>(hub: &Path, mut call: F)
where
    F: FnMut(String, Value, Option<String>) -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    for mode in ["exit-1", "exit-2", "exit-3", "exit-4", "unknown-state"] {
        std::fs::write(hub.join("mode"), mode).unwrap();
        let error = call("generation_status".into(), json!({"taskId":"fixture-job"}), None)
            .await
            .unwrap_err();
        assert!(!error.contains("fixture-private-credential"));
        assert!(!error.contains("Signature="));
    }
    assert!(call("export_project".into(), json!({}), None).await.is_err());
    assert!(
        call("generation_status".into(), json!({"taskId":"job","saveToLibrary":{}}), None)
            .await
            .is_err()
    );
    assert!(call("get_my_permissions".into(), json!({}), Some("other-agent".into())).await.is_err());
}
