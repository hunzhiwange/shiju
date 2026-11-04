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
use crate::{global_database::GlobalDatabase, tools::ToolRuntimeContext};
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
    fn context(&self) -> &ToolRuntimeContext {
        &self.context
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
    candidate.tenant = context.tool_use_context().tenant_scope();
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
    let (dir, context, _) = super::cloud_tests::fixture("ok");
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
        let (dir, context, _) = super::cloud_tests::fixture("ok");
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

#[tokio::test]
async fn video_edit_transcript_native_adapter_keeps_audio_ticket_private_and_queries_exact_task() {
    let (dir, context, skill) = super::cloud_tests::fixture("ok");
    let provider = super::ShijuCloudVideoToolProvider::discover(
        context,
        std::slice::from_ref(&skill),
        Some(json!(["bash", "bash_wait", "bash_cancel"])),
    )
    .await
    .unwrap();
    let root = skill.location.unwrap().parent().unwrap().join("cutvideo");
    std::fs::write(root.join("scripts/occ.py"),r#"import json,sys,pathlib
root=pathlib.Path(__file__).parent.parent
a=sys.argv[1:]
if a[0]=='schema':
 props={'media_probe':{'projectId':'string','assetId':'string'},'media_audio_extract':{'projectId':'string','assetId':'string','sampleRate':'number','channels':'number'},'asr_transcribe':{'audioUrl':'string','idempotencyKey':'string','words':'boolean','taskId':'string'}}
 print(json.dumps({'name':a[1],'inputSchema':{'type':'object','properties':{k:{'type':v} for k,v in props[a[1]].items()}}}));sys.exit(0)
assert a[0]=='call' and a[2]=='-' and '--no-wait' in a
v=json.load(sys.stdin);op=a[1]
if op=='media_probe':r={'durationSec':10,'fps':29.97,'streams':[{'codec_type':'video'},{'codec_type':'audio'}]}
elif op=='media_audio_extract':
 assert v=={'projectId':'project','assetId':'asset','sampleRate':16000,'channels':1}
 r={'audioUrl':'https://fixture.oss.aliyuncs.com/audio?Signature=private-audio-ticket'}
elif op=='asr_transcribe':
 if 'taskId' in v:
  assert v=={'taskId':'asr:original-job'}
  r={'status':'succeeded','engine':'fun-asr','text':'原话','segments':[{'start':1,'end':2,'text':'原话','words':[{'start':1,'end':2,'text':'原话'}]}]}
 else:
  assert v=={'audioUrl':'https://fixture.oss.aliyuncs.com/audio?Signature=private-audio-ticket','idempotencyKey':'video-asr:fixture','words':True}
  r={'status':'running','taskId':'asr:original-job'}
  with (root/'submissions').open('a') as f:f.write('asr\n')
else:raise ValueError()
with open(a[a.index('--out')+1],'w') as f:json.dump(r,f)
"#).unwrap();
    let probe = provider.probe_transcript_media("project", "asset").await.unwrap();
    let pending = provider.transcribe("project", "asset", "video-asr:fixture", None).await.unwrap();
    let response = provider
        .transcribe("project", "asset", "video-asr:fixture", pending["taskId"].as_str())
        .await
        .unwrap();
    assert_eq!(normalize_transcript("asset", &probe, &response).unwrap().text, "原话");
    assert_eq!(std::fs::read_to_string(root.join("submissions")).unwrap().lines().count(), 1);
    fn inspect(path: &std::path::Path) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                inspect(&path);
            } else if entry.file_type().unwrap().is_file() && path.file_name().unwrap() != "occ.py"
            {
                assert!(
                    !std::fs::read(&path)
                        .unwrap()
                        .windows(20)
                        .any(|s| s == b"private-audio-ticket"),
                    "ticket leaked to {}",
                    path.display()
                );
            }
        }
    }
    inspect(dir.path());
}
