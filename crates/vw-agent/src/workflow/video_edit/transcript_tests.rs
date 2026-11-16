use super::transcript::*;
use serde_json::json;
#[tokio::test]
async fn video_edit_transcript_native_adapter_keeps_audio_ticket_private_and_queries_exact_task() {
    let (dir, context, skill) = super::cloud_tests::fixture("ok");
    let provider = super::cloud::discover(
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
