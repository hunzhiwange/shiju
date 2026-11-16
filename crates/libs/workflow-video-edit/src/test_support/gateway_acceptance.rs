use super::{gateway_contracts::*, scenario::{Scenario, Fault}};
use std::{path::Path, sync::Arc};
use serde_json::Value;
#[async_trait::async_trait]
pub trait GatewayAcceptance: Sync {
    fn install(&self, session: &str, scenario: Arc<Scenario>) -> Box<dyn Send>;
    fn ledger(&self, path: &Path) -> Value;
    async fn run_cancelled(&self, root: &Path, session: &str, path: &Path, scenario: &Scenario);
    async fn run(&self, root: &Path, session: &str, path: &Path, scenario: &Scenario, succeeds: bool);
}

pub async fn video_edit_e2e_long_transcripts_reach_done_and_reload(host: &impl GatewayAcceptance) {
    let mut run_ids = Vec::new();
    for duration in [300, 600] {
        let root = tempfile::tempdir().unwrap();
        let path = source(root.path(), &duration.to_string());
        let session = format!("ses_e2e_{}", uuid::Uuid::new_v4());
        let scenario = Scenario::new(duration, Fault::None);
        let _transport = host.install(&session, scenario.clone());
        host.run(root.path(), &session, &path, &scenario, true).await;
        let row = host.ledger(&path);
        run_ids.push(row["run_id"].clone());
        super::gateway_contracts::long_transcript(&row, &scenario, duration);
    }
    assert_ne!(run_ids[0], run_ids[1]);
}

pub async fn video_edit_e2e_quota_invalid_segment_and_fake_preview_never_done(host: &impl GatewayAcceptance) {
    for fault in [Fault::Quota, Fault::InvalidChunk, Fault::FakePreview] {
        let root = tempfile::tempdir().unwrap();
        let path = source(root.path(), &format!("{fault:?}"));
        let session = format!("ses_e2e_{}", uuid::Uuid::new_v4());
        let scenario = Scenario::new(600, fault);
        let _external = host.install(&session, scenario.clone());
        host.run(root.path(), &session, &path, &scenario, false).await;
        if fault != Fault::FakePreview {
            assert!(captured(&scenario, "project.write").is_empty());
            assert!(captured(&scenario, "asset.submit").is_empty());
        }
    }
}

pub async fn video_edit_e2e_accepted_timeout_reconciles_without_duplicate_post(host: &impl GatewayAcceptance) {
    let root = tempfile::tempdir().unwrap();
    let path = source(root.path(), "accepted timeout");
    let session = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let scenario = Scenario::new(600, Fault::AcceptedTimeout);
    let _external = host.install(&session, scenario.clone());
    host.run(root.path(), &session, &path, &scenario, true).await;
    super::gateway_contracts::accepted_timeout(&scenario, &host.ledger(&path));
}

pub async fn video_edit_e2e_independent_requests_have_distinct_run_budgets(host: &impl GatewayAcceptance) {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let pa = source(a.path(), "parallel-a");
    let pb = source(b.path(), "parallel-b");
    std::fs::write(&pb, std::fs::read(&pa).unwrap()).unwrap();
    let sa = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let sb = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let ca = Scenario::new(600, Fault::None);
    let cb = Scenario::new(600, Fault::None);
    let _ta = host.install(&sa, ca.clone());
    let _tb = host.install(&sb, cb.clone());
    tokio::join!(host.run(a.path(), &sa, &pa, &ca, true), host.run(b.path(), &sb, &pb, &cb, true));
    assert_ne!(host.ledger(&pa)["run_id"], host.ledger(&pb)["run_id"]);
    assert_ne!(host.ledger(&pa)["project_id"], host.ledger(&pb)["project_id"]);
}

pub async fn video_edit_e2e_cancelled_restart_reconciles_and_never_resubmits(host: &impl GatewayAcceptance) {
    let root = tempfile::tempdir().unwrap();
    let path = source(root.path(), "cancel restart");
    let session = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let scenario = Scenario::new(600, Fault::AcceptedTimeout);
    let _external = host.install(&session, scenario.clone());
    host.run_cancelled(root.path(), &session, &path, &scenario).await;
    // The graph owns async node tasks; wait for their durable cancellation cleanup
    // before simulating the next process. A lost lease/save must time out here.
    let before = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let row = host.ledger(&path);
            if row["asset_cancelled"] == true {
                break row;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cancelled graph must persist cleanup before restart");
    let planned = captured(&scenario, "model.start");
    let posted = captured(&scenario, "asset.submit");
    assert_eq!(posted.len(), 5);
    let restarted = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let _restart = host.install(&restarted, scenario.clone());
    host.run(root.path(), &restarted, &path, &scenario, false).await;
    super::gateway_contracts::cancelled_restart(&scenario, posted, planned, &before, &host.ledger(&path));
}
