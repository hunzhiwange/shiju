use crate::context::VideoContext;
use super::transcript::*;
use serde_json::{Value, json};
fn probe() -> Value {
    json!({"durationSec":10.0,"streams":[{"codec_type":"video","avg_frame_rate":"30000/1001"},{"codec_type":"audio"}]})
}
fn receipt() -> Value {
    json!({"status":"succeeded","engine":"fun-asr","text":"原话， 原话。","segments":[{"text":"原话，","start":1.0,"end":2.0,"words":[{"text":"原话","start":1.0,"end":2.0},{"text":"，","start":2.0,"end":2.0,"punctuation":true}]},{"text":" 原话。","start":5.0,"end":6.0,"words":[{"text":"原话。","start":5.0,"end":6.0}]}]})
}
#[test]
fn video_edit_transcript_preserves_facts_silence_and_fractional_fps() {
    let t = normalize_transcript("asset", &probe(), &receipt()).unwrap();
    assert_eq!(t.text, "原话， 原话。");
    assert_eq!(t.words.len(), 2);
    assert_eq!(t.words[1].word_index, 2);
    assert_eq!(t.words[1].start, 5.0);
    assert_eq!(t.words[1].text, "原话。");
    assert_eq!(t.fps, 30000.0 / 1001.0);
    assert_eq!(t.diagnostics.len(), 1);
}
#[test]
fn video_edit_transcript_rejects_invalid_times_and_duplicate_indices() {
    for bad in [
        json!(true),
        json!(-1),
        json!(11),
        json!("NaN"),
        json!("Infinity"),
        Value::from(f64::NAN),
        Value::from(f64::INFINITY),
    ] {
        let mut r = receipt();
        r["segments"][0]["words"][0]["start"] = bad;
        assert!(normalize_transcript("asset", &probe(), &r).is_err());
    }
    for (start, end) in [(2.0, 1.0), (1.0, 1.0), (9.0, 11.0)] {
        let mut r = receipt();
        r["segments"][0]["words"][0]["start"] = json!(start);
        r["segments"][0]["words"][0]["end"] = json!(end);
        assert!(normalize_transcript("asset", &probe(), &r).is_err());
    }
    let mut r = receipt();
    r["segments"][0]["words"][0]["word_index"] = json!(0);
    r["segments"][1]["words"][0]["word_index"] = json!(0);
    assert!(normalize_transcript("asset", &probe(), &r).is_err());
}
#[test]
fn video_edit_transcript_rejects_silent_failed_empty_and_unconfirmed_punctuation() {
    let mut p = probe();
    p["streams"].as_array_mut().unwrap().pop();
    assert!(normalize_transcript("asset", &p, &receipt()).is_err());
    for status in ["failed", "running", "unknown"] {
        let mut r = receipt();
        r["status"] = json!(status);
        assert!(normalize_transcript("asset", &probe(), &r).is_err());
    }
    let mut r = receipt();
    r["segments"] = json!([]);
    assert!(normalize_transcript("asset", &probe(), &r).is_err());
    let mut r = receipt();
    r["segments"][0]["words"][1].as_object_mut().unwrap().remove("punctuation");
    assert!(normalize_transcript("asset", &probe(), &r).is_err());
}

use super::{manifest::*, prepare::*};
use agent_database::GlobalDatabase;
use super::TestContext as ToolRuntimeContext;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
struct Cloud {
    context: ToolRuntimeContext,
    submissions: AtomicUsize,
    queries: AtomicUsize,
    mode: &'static str,
}
#[async_trait::async_trait]
impl VideoTranscriptCloud for Cloud {
    fn context(&self) -> crate::context::VideoContextHandle {
        std::sync::Arc::new(self.context.clone())
    }
    async fn probe_transcript_media(&self, _: &str, _: &str) -> anyhow::Result<Value> {
        let mut p = probe();
        if self.mode == "silent" {
            p["streams"].as_array_mut().unwrap().pop();
        }
        Ok(p)
    }
    async fn transcribe(
        &self,
        _: &str,
        _: &str,
        _: &str,
        task: Option<&str>,
    ) -> anyhow::Result<Value> {
        if let Some(task) = task {
            assert_eq!(task, "asr:original-job");
            self.queries.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            return Ok(receipt());
        }
        self.submissions.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            "unknown" => anyhow::bail!("external response lost"),
            "failed" => Ok(json!({"status":"failed"})),
            _ => Ok(json!({"status":"running","taskId":"asr:original-job"})),
        }
    }
}
fn prepared(db: &GlobalDatabase, context: &ToolRuntimeContext) -> PreparedCloudVideo {
    let store = VideoEditManifestStore::new(db.clone()).unwrap();
    let mut candidate = super::manifest_tests::candidate();
    candidate.tenant = context.tenant_scope();
    candidate.state = VideoPrepareState::Ready;
    candidate.project_id = Some("project".into());
    candidate.asset_id = Some("asset".into());
    let run = store.claim(candidate, "preparation", super::prepare::now_ms()).unwrap();
    store.release(&run).unwrap();
    PreparedCloudVideo {
        manifest: run,
        media: VideoMediaInfo {
            has_video: true,
            width: 1920,
            height: 1080,
            duration_sec: 10.0,
            has_audio: true,
        },
    }
}
#[tokio::test]
async fn video_edit_transcript_concurrent_resume_reuses_one_job_and_persisted_result() {
    let dir = tempfile::tempdir().unwrap();
    let context = ToolRuntimeContext::new("transcript", Some(dir.path().to_string_lossy().into()));
    let db = GlobalDatabase::new(dir.path().join("db")).unwrap();
    let p = prepared(&db, &context);
    let cloud = Arc::new(Cloud {
        context,
        submissions: AtomicUsize::new(0),
        queries: AtomicUsize::new(0),
        mode: "pending",
    });
    assert!(
        transcribe_cloud_video(db.clone(), cloud.as_ref(), &p)
            .await
            .unwrap_err()
            .to_string()
            .contains("pending")
    );
    let barrier = Arc::new(tokio::sync::Barrier::new(5));
    let mut handles = Vec::new();
    for _ in 0..5 {
        let db = db.clone();
        let cloud = cloud.clone();
        let p = p.clone();
        let barrier = barrier.clone();
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            transcribe_cloud_video(db, cloud.as_ref(), &p).await
        }));
    }
    for handle in handles {
        assert_eq!(handle.await.unwrap().unwrap().text, "原话， 原话。");
    }
    assert_eq!(cloud.submissions.load(Ordering::SeqCst), 1);
    assert_eq!(cloud.queries.load(Ordering::SeqCst), 1);
    drop(db);
    let reopened = GlobalDatabase::new(dir.path().join("db")).unwrap();
    assert_eq!(transcribe_cloud_video(reopened, cloud.as_ref(), &p).await.unwrap().words.len(), 2);
    assert_eq!(cloud.queries.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn video_edit_transcript_errors_never_resubmit_or_produce_fake_speech() {
    for mode in ["unknown", "failed", "silent"] {
        let dir = tempfile::tempdir().unwrap();
    let context = ToolRuntimeContext::new("transcript", Some(dir.path().to_string_lossy().into()));
        let db = GlobalDatabase::new(dir.path().join("db")).unwrap();
        let p = prepared(&db, &context);
        let cloud =
            Cloud { context, submissions: AtomicUsize::new(0), queries: AtomicUsize::new(0), mode };
        for _ in 0..2 {
            assert!(transcribe_cloud_video(db.clone(), &cloud, &p).await.is_err());
        }
        assert_eq!(cloud.submissions.load(Ordering::SeqCst), usize::from(mode != "silent"));
    }
}
#[test]
fn video_edit_transcript_rejects_invalid_media_and_segment_order() {
    for bad in [json!(true), json!(0), json!(-1), json!("Infinity"), json!(null)] {
        let mut p = probe();
        p["durationSec"] = bad;
        assert!(normalize_transcript("asset", &p, &receipt()).is_err());
    }
    for bad in [json!(true), json!(0), json!("30000/0"), json!("NaN/1"), json!("30000/Infinity")] {
        let mut p = probe();
        p["streams"][0]["avg_frame_rate"] = bad;
        assert!(normalize_transcript("asset", &p, &receipt()).is_err());
    }
    let mut r = receipt();
    r["segments"].as_array_mut().unwrap().reverse();
    assert!(normalize_transcript("asset", &probe(), &r).is_err());
}
