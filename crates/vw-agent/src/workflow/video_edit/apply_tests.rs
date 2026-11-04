use super::apply::*;
#[test]
fn video_edit_delivery_maps_effect_after_cut_without_moving_source_coordinates() {
    let windows = vec![
        VideoSourceWindow {
            source_start: 3.0,
            source_end: 8.0,
            timeline_start: 0.0,
            timeline_end: 5.0,
        },
        VideoSourceWindow {
            source_start: 10.0,
            source_end: 15.0,
            timeline_start: 5.0,
            timeline_end: 10.0,
        },
    ];
    validate_video_mapping(&windows, 20.0).unwrap();
    assert_eq!(video_timeline_range(&windows, 11.0, 13.0).unwrap(), (6.0, 8.0));
    assert!(video_timeline_range(&windows, 7.0, 11.0).is_err());
}
#[test]
fn video_edit_delivery_rejects_invalid_or_retimed_mapping() {
    for w in [
        VideoSourceWindow {
            source_start: 0.0,
            source_end: 8.0,
            timeline_start: 0.0,
            timeline_end: 5.0,
        },
        VideoSourceWindow {
            source_start: 3.0,
            source_end: 8.0,
            timeline_start: 1.0,
            timeline_end: 6.0,
        },
        VideoSourceWindow {
            source_start: f64::NAN,
            source_end: 8.0,
            timeline_start: 0.0,
            timeline_end: 5.0,
        },
    ] {
        assert!(validate_video_mapping(&[w], 20.0).is_err());
    }
}

use super::apply_test_cloud::*;
use super::{VideoProjectJournal, assets::*, planning::*, preview::*};
use crate::global_database::GlobalDatabase;
use anyhow::Result;
use std::{collections::BTreeMap, sync::Arc};
use tokio_util::sync::CancellationToken;

fn plan() -> VideoEditPlan {
    VideoEditPlan {
        asset_id: "source".into(),
        duration_sec: 20.0,
        effects: vec![VideoEditEffect {
            id: "effect".into(),
            chunk_id: super::segments::segment_video_transcript(&transcript(), Default::default())
                .unwrap()[0]
                .chunk_id
                .clone(),
            word_index: 0,
            end_word_index: 0,
            anchor: "word".into(),
            start: 11.0,
            end: 13.0,
            visual_type: VisualType::BRoll,
            assets: vec!["footage".into()],
        }],
    }
}
fn registry() -> VideoAssetRegistry {
    VideoAssetRegistry {
        references: BTreeMap::from([("digest".into(), "gen:remote".into())]),
        progress: VideoAssetProgress { completed: 1, ..Default::default() },
    }
}
async fn run(
    endpoint: &Endpoint,
    journal: &dyn VideoApplyJournal,
    cancel: &CancellationToken,
) -> Result<VideoEditDelivery> {
    let transcript = transcript();
    let segments =
        super::segments::segment_video_transcript(&transcript, Default::default()).unwrap();
    let requirements = requirements();
    let plan = plan();
    let registry = registry();
    let digest = video_plan_sha256(&plan)?;
    let materials = BTreeMap::from([(
        "effect".into(),
        VideoEffectMaterial::GeneratedVideo { digest: "digest".into() },
    )]);
    apply_video_edit(
        endpoint,
        journal,
        VideoApplyInput {
            project_id: "project",
            template_sha256: &template_digest(),
            plan_sha256: &digest,
            plan: &plan,
            transcript: &transcript,
            segments: &segments,
            requirements: &requirements,
            template_snapshot: "template",
            registry: &registry,
            materials: &materials,
            cancel,
        },
    )
    .await
}
fn journal(root: &tempfile::TempDir) -> VideoProjectJournal {
    VideoProjectJournal::new(
        GlobalDatabase::new(root.path().join("ledger.sqlite")).unwrap(),
        Default::default(),
    )
    .unwrap()
}
#[tokio::test]
async fn video_edit_delivery_external_endpoint_captures_order_and_actual_mapping() {
    let root = tempfile::tempdir().unwrap();
    let j = journal(&root);
    let e = Endpoint::new().await;
    let result = run(&e, &j, &CancellationToken::new()).await.unwrap();
    assert_eq!(result.source_mapping[1].timeline_start, 5.0);
    let state = e.state.lock().unwrap();
    assert!(matches!(
        state.writes.as_slice(),
        [
            VideoProjectWrite::Import { .. },
            VideoProjectWrite::Original { .. },
            VideoProjectWrite::Transcript { .. },
            VideoProjectWrite::Cleanup,
            VideoProjectWrite::Captions,
            VideoProjectWrite::Effect { start: 6.0, end: 8.0, .. }
        ]
    ));
}
#[tokio::test]
async fn video_edit_delivery_timeout_reconciles_without_duplicate_remote_writes() {
    let root = tempfile::tempdir().unwrap();
    let j = journal(&root);
    let e = Endpoint::new().await;
    e.state.lock().unwrap().lost_receipt = true;
    assert!(run(&e, &j, &CancellationToken::new()).await.is_err());
    let restarted = journal(&root);
    run(&e, &restarted, &CancellationToken::new()).await.unwrap();
    assert_eq!(e.state.lock().unwrap().writes.len(), 6);
}
#[tokio::test]
async fn video_edit_delivery_partial_failure_pending_preview_and_empty_page_never_deliver() {
    for mode in 0..4 {
        let root = tempfile::tempdir().unwrap();
        let j = journal(&root);
        let e = Endpoint::new().await;
        {
            let mut s = e.state.lock().unwrap();
            match mode {
                0 => s.no_change = true,
                1 => s.missing_audio = true,
                2 => s.pending_preview = true,
                _ => s.empty_page = true,
            }
        }
        assert!(run(&e, &j, &CancellationToken::new()).await.is_err());
    }
}
#[tokio::test]
async fn video_edit_delivery_cancellation_persists_and_blocks_later_writes() {
    let root = tempfile::tempdir().unwrap();
    let j = journal(&root);
    let e = Endpoint::new().await;
    let cancel = CancellationToken::new();
    e.state.lock().unwrap().canceller = Some(cancel.clone());
    assert!(run(&e, &j, &cancel).await.is_err());
    assert_eq!(e.state.lock().unwrap().writes.len(), 4);
    assert!(run(&e, &journal(&root), &CancellationToken::new()).await.is_err());
    assert_eq!(e.state.lock().unwrap().writes.len(), 4);
}
#[tokio::test]
async fn video_edit_delivery_two_writers_share_durable_project_fence() {
    let root = tempfile::tempdir().unwrap();
    let a = journal(&root);
    let b = journal(&root);
    let (x, y) = tokio::join!(a.claim("project"), b.claim("project"));
    assert_eq!(usize::from(x.is_ok()) + usize::from(y.is_ok()), 1);
    if x.is_ok() {
        a.release("project").await.unwrap();
        b.claim("project").await.unwrap();
        b.release("project").await.unwrap();
    } else {
        b.release("project").await.unwrap();
        a.claim("project").await.unwrap();
        a.release("project").await.unwrap();
    }
}
#[tokio::test]
async fn video_edit_delivery_missing_material_and_mutated_digest_reject_before_write() {
    let root = tempfile::tempdir().unwrap();
    let j = journal(&root);
    let e = Endpoint::new().await;
    let transcript = transcript();
    let segments =
        super::segments::segment_video_transcript(&transcript, Default::default()).unwrap();
    let requirements = requirements();
    let mut p = plan();
    let digest = video_plan_sha256(&p).unwrap();
    p.effects[0].end = 14.0;
    let r = registry();
    let cancel = CancellationToken::new();
    assert!(
        apply_video_edit(
            &e,
            &j,
            VideoApplyInput {
                project_id: "project",
                template_sha256: &template_digest(),
                plan_sha256: &digest,
                plan: &p,
                transcript: &transcript,
                segments: &segments,
                requirements: &requirements,
                template_snapshot: "template",
                registry: &r,
                materials: &BTreeMap::new(),
                cancel: &cancel
            }
        )
        .await
        .is_err()
    );
    p.effects[0].end = 13.0;
    let digest = video_plan_sha256(&p).unwrap();
    assert!(
        apply_video_edit(
            &e,
            &j,
            VideoApplyInput {
                project_id: "project",
                template_sha256: &template_digest(),
                plan_sha256: &digest,
                plan: &p,
                transcript: &transcript,
                segments: &segments,
                requirements: &requirements,
                template_snapshot: "template",
                registry: &r,
                materials: &BTreeMap::new(),
                cancel: &cancel
            }
        )
        .await
        .is_err()
    );
    assert!(e.state.lock().unwrap().writes.is_empty());
}

fn transcript() -> super::transcript::VideoTranscript {
    super::transcript::VideoTranscript {
        asset_id: "source".into(),
        duration_sec: 20.0,
        fps: 30.0,
        text: "word".into(),
        words: vec![super::transcript::VideoTranscriptWord {
            word_index: 0,
            text: "word".into(),
            start: 11.0,
            end: 13.0,
        }],
        source: "asr".into(),
        diagnostics: vec![],
    }
}
fn requirements() -> VideoPlanRequirements {
    VideoPlanRequirements {
        instructions: "source-preserving edit".into(),
        required_types: vec![VisualType::BRoll],
        max_effects_per_word: 1,
    }
}
fn template_digest() -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(b"template"))
}

#[tokio::test]
async fn video_edit_delivery_competing_writer_cannot_reach_external_endpoint() {
    let root = tempfile::tempdir().unwrap();
    let a = journal(&root);
    let b = journal(&root);
    let e = Endpoint::new().await;
    let gate = Arc::new(Gate::default());
    e.state.lock().unwrap().gate = Some(gate.clone());
    let cancel = CancellationToken::new();
    let first = run(&e, &a, &cancel);
    tokio::pin!(first);
    tokio::select! { result=&mut first => panic!("write should be paused: {result:?}"), _=gate.entered.notified()=>{} }
    assert!(run(&e, &b, &CancellationToken::new()).await.is_err());
    assert_eq!(e.state.lock().unwrap().writes.len(), 1, "second writer reached remote endpoint");
    gate.release.notify_one();
    first.await.unwrap();
    assert_eq!(e.state.lock().unwrap().writes.len(), 6);
}
#[tokio::test]
async fn video_edit_delivery_no_change_is_not_promoted_to_applied_by_retry() {
    let root = tempfile::tempdir().unwrap();
    let j = journal(&root);
    let e = Endpoint::new().await;
    e.state.lock().unwrap().no_change = true;
    assert!(run(&e, &j, &CancellationToken::new()).await.is_err());
    e.state.lock().unwrap().no_change = false;
    assert!(run(&e, &journal(&root), &CancellationToken::new()).await.is_err());
    assert!(e.state.lock().unwrap().writes.is_empty());
}
#[tokio::test]
async fn video_edit_delivery_project_fence_is_tenant_scoped_and_rejects_stale_save() {
    let root = tempfile::tempdir().unwrap();
    let a = journal(&root);
    let other = VideoProjectJournal::new(
        GlobalDatabase::new(root.path().join("ledger.sqlite")).unwrap(),
        vw_api_types::TenantScope::new("platform", "other").unwrap(),
    )
    .unwrap();
    let ledger = a.claim("project").await.unwrap();
    other.claim("project").await.unwrap();
    a.release("project").await.unwrap();
    assert!(a.save("project", &ledger).await.is_err());
    other.release("project").await.unwrap();
}
#[test]
fn video_edit_delivery_rejects_unsafe_preexisting_project_fence_schema() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::new(root.path().join("unsafe.sqlite")).unwrap();
    db.connect_sqlite().unwrap().execute_batch("CREATE TABLE workflow_video_project_writes (platform_id TEXT, company_id TEXT, project_id TEXT, payload TEXT, owner TEXT)").unwrap();
    assert!(VideoProjectJournal::new(db, Default::default()).is_err());
}
