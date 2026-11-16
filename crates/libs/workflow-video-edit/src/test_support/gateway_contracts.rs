use super::scenario::Scenario;
use serde_json::Value;
use std::{path::Path, sync::atomic::Ordering};
pub fn source(root: &Path, tag: &str) -> std::path::PathBuf {
    let path = root.join(format!("中文 原片 {tag}.mp4"));
    std::fs::write(&path, format!("SYNTHETIC TEST DATA ONLY {tag}")).unwrap();
    path
}
pub fn runs(database: &agent_database::GlobalDatabase) -> Vec<Value> {
    let conn = database.connect_sqlite().unwrap();
    conn.prepare("SELECT payload FROM workflow_video_edit_runs")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| serde_json::from_str(&r.unwrap()).unwrap())
        .collect()
}
pub fn ledger(database: &agent_database::GlobalDatabase, path: &Path) -> Value {
    runs(database)
        .into_iter()
        .find(|r| r["source"]["path"] == path.canonicalize().unwrap().to_string_lossy().as_ref())
        .unwrap()
}
pub fn captured(s: &Scenario, kind: &str) -> Vec<String> {
    s.capture.lock().unwrap().iter().filter(|(k, _)| k == kind).map(|(_, id)| id.clone()).collect()
}

pub fn long_transcript(row: &Value, scenario: &Scenario, duration: u32) {
        assert_eq!(row["transcript"]["Ready"]["duration_sec"].as_f64(), Some(f64::from(duration)));
        let starts = captured(&scenario, "model.start");
        let done = captured(&scenario, "model.done");
        assert!(starts.len() >= 5);
        assert_eq!(done[..5], starts[..5].iter().rev().cloned().collect::<Vec<_>>());
        let jobs = row["asset_jobs"].as_array().unwrap();
        assert_eq!(jobs.len(), starts.len());
        assert!(jobs.iter().all(|j| j["state"] == "Succeeded" && j["attempts"] == 1));
        if duration == 600 {
            assert!(starts.len() > 5);
            assert!(jobs.len() > 5);
        }
        assert_eq!(captured(&scenario, "asset.submit").len(), jobs.len());
        let posts = captured(&scenario, "asset.submit");
        let ready = captured(&scenario, "asset.done");
        assert_eq!(ready[..5], posts[..5].iter().rev().cloned().collect::<Vec<_>>());
        assert_eq!(captured(&scenario, "project.create").len(), 1);
        assert_eq!(captured(&scenario, "upload.put").len(), 1);
        assert_eq!(captured(&scenario, "transcript.submit").len(), 1);
        assert_eq!(captured(&scenario, "preview.read").len(), 1);
        assert!(
            captured(&scenario, "project.write")
                .iter()
                .all(|w| !w.to_lowercase().contains("export"))
        );
}

pub fn accepted_timeout(scenario: &Scenario, row: &Value) {
    let submitted = captured(&scenario, "asset.submit");
    let reconciled = captured(&scenario, "asset.reconcile");
    assert_eq!(submitted.len(), submitted.iter().collect::<std::collections::BTreeSet<_>>().len());
    assert_eq!(
        submitted.iter().collect::<std::collections::BTreeSet<_>>(),
        reconciled.iter().collect()
    );
    assert!(
        row["asset_jobs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|j| j["state"] == "Succeeded" && j["attempts"] == 1)
    );
}

pub fn cancelled_restart(scenario: &Scenario, posted: Vec<String>, planned: Vec<String>, before: &Value, after: &Value) {
    assert_eq!(captured(&scenario, "asset.submit"), posted);
    assert_eq!(
        captured(&scenario, "model.start"),
        planned,
        "cancelled remote work must be reconciled before any new planning"
    );
    assert_eq!(scenario.peak.load(Ordering::SeqCst), 5);
    assert_eq!(captured(&scenario, "asset.reconcile").len(), 5);
    assert_eq!(after["run_id"], before["run_id"]);
    assert!(captured(&scenario, "project.write").is_empty());
}

pub fn terminal_success(root: &Path, message_id: Option<&str>, output: &str, events: &[Value]) {
    assert!(message_id.unwrap().starts_with("msg_"));
    assert!(output.contains("test-preview"));
    assert_eq!(events.last().unwrap()["type"], "chat.done");
    assert!(!root.join(".shiju/workflow_commands.json").exists());
}

pub fn shared_budget(s: &Scenario) {
    assert_eq!(s.peak.load(Ordering::SeqCst), 5, "shared model/asset external peak");
    assert_eq!(s.write_peak.load(Ordering::SeqCst), 1);
    assert_eq!(s.active.load(Ordering::SeqCst), 0);
}

pub fn terminal_failure(events: &[Value]) {
    assert!(!events.iter().any(|e| e["type"] == "chat.done"));
    assert!(events.iter().any(|e| e["type"] == "chat.error"));
}
