use crate::{assets::*, manifest::*, prepare::*, transcript::*};
use crate::cloud::ShijuCloudVideoToolProvider;
use serde_json::json;
use std::path::{Path, PathBuf};

pub async fn video_edit_asset_jobs_native_adapter_reconciles_pages_and_reads_reference(provider: &ShijuCloudVideoToolProvider, _dir: &Path, root: PathBuf) {
    std::fs::write(root.join("scripts/occ.py"), r#"import json,sys,pathlib
args=sys.argv[1:]
if args[0]=='schema':
 fields={'video_generate':{'projectId':'string','idempotencyKey':'string','prompt':'string','kind':'string','provider':'string','model':'string','durationSec':'integer','ratio':'string','resolution':'string','audio':'boolean'},'generation_list':{'projectId':'string','mine':'boolean','tool':'string','limit':'integer','refresh':'boolean','before':'string'},'generation_status':{'taskId':'string'},'media_probe':{'projectId':'string','url':'string'}}
 print(json.dumps({'name':args[1],'inputSchema':{'type':'object','properties':{k:{'type':v} for k,v in fields[args[1]].items()}}}));sys.exit(0)
assert args[0]=='call' and args[2]=='-'
v=json.load(sys.stdin);op=args[1]
root=pathlib.Path(__file__).parent.parent
if op=='video_generate':
 assert v['audio']==False and v['provider']=='wan3'
 (root/'asset-key').write_text(v['idempotencyKey'])
 r={'taskId':'native-task','status':'queued'}
elif op=='generation_list':
 if 'before' not in v:r={'items':[],'nextBefore':'2026-11-01T00:00:00Z'}
 else:r={'items':[{'idempotencyKey':(root/'asset-key').read_text(),'projectId':'project','tool':'video_generate','taskId':'native-task'}],'nextBefore':None}
elif op=='generation_status':r={'taskId':v['taskId'],'status':'succeeded','ref':'gen:native-result','videoUrl':'https://private.invalid/file?Signature=secret'}
elif op=='media_probe':
 assert v['url']=='gen:native-result'
 r={'durationSec':5,'streams':[{'codec_type':'video'}]}
else:raise ValueError()
with open(args[args.index('--out')+1],'w') as f:json.dump(r,f)
"#).unwrap();
    let (_dir2, _, _, requests) = super::assets_fixture::fixture(1);
    let request = &requests[0];
    assert!(matches!(
        provider.submit(request, "native-key-123").await.unwrap(),
        VideoAssetRemoteState::Running(_)
    ));
    let job = VideoAssetJob {
        request: request.clone(),
        idempotency_key: "native-key-123".into(),
        task_id: None,
        state: VideoAssetJobState::Unknown,
        result_ref: None,
        attempts: 1,
        reconciliation_version: 0,
        failed_attempts: vec![],
    };
    let result = provider.reconcile(&job).await.unwrap();
    let VideoAssetRemoteState::Succeeded { reference, .. } = result else {
        panic!("expected confirmed asset");
    };
    assert_eq!(reference, "gen:native-result");
    provider.verify("project", &reference).await.unwrap();
    assert!(!serde_json::to_string(&job).unwrap().contains("Signature"));
}

pub async fn video_edit_transcript_native_adapter_keeps_audio_ticket_private_and_queries_exact_task(provider: &ShijuCloudVideoToolProvider, dir: &Path, root: PathBuf) {
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
    inspect(dir);
}

pub async fn video_edit_prepare_native_receipts_keep_signed_tickets_in_memory(provider: &ShijuCloudVideoToolProvider, dir: &Path, root: PathBuf, ctx: &dyn crate::context::VideoContext) {
    std::fs::write(root.join("company"), "10011").unwrap();
    std::fs::write(root.join("scripts/occ.py"),r#"import json,sys,pathlib
root=pathlib.Path(__file__).parent.parent
a=sys.argv[1:]
if a[0]=='schema':
 props={'get_my_permissions':{},'create_project':{'name':'string','brandId':'string','storeId':'string'},'list_projects':{'q':'string','mine':'boolean','limit':'integer','brandId':'string','storeId':'string','cursor':'string'},'import_media':{'projectId':'string','action':'string','files':'array','uploadIds':'array','jobId':'string'},'media_probe':{'projectId':'string','assetId':'string'}}
 print(json.dumps({'name':a[1],'inputSchema':{'type':'object','properties':{k:{'type':v} for k,v in props[a[1]].items()}}}));sys.exit(0)
assert a[0]=='call' and a[2]=='-' and '--no-wait' in a
v=json.load(sys.stdin);op=a[1]
with (root/'mutations').open('a') as f:
 if op=='create_project' or op=='import_media' and v['action']=='create_session':f.write(op+'\n')
if op=='get_my_permissions':r={'member':{'companyId':(root/'company').read_text(),'status':'active','effectivePermissions':['project.read','project.create','material.read','material.create'],'brandIds':['brand'],'storeIds':[]}}
elif op=='create_project':
 (root/'project').write_text(json.dumps({'id':'project','name':v['name'],'brandId':'brand'}));r={'projectId':'project'}
elif op=='list_projects':r={'items':[json.loads((root/'project').read_text())],'nextCursor':None}
elif op=='import_media':
 r={'uploads':[{'uploadId':'upload','uploadUrl':'https://fixture.oss.aliyuncs.com/private?Signature=memory-secret','headers':{}}]} if v['action']=='create_session' else {'items':[{'assetId':'asset'}]}
elif op=='media_probe':r={'durationSec':45,'streams':[{'codec_type':'video','width':1920,'height':1080},{'codec_type':'audio'}]}
else:raise ValueError()
with open(a[a.index('--out')+1],'w') as f:json.dump(r,f)
print('truncated stdout must never become the receipt')
"#).unwrap();
    let scopes = provider.authorized_scopes().await.unwrap();
    assert_eq!(scopes.len(), 1);
    let project = provider.create_project("run-fixture", &scopes[0]).await.unwrap();
    assert_eq!(
        provider.find_project("run-fixture", &scopes[0]).await.unwrap().as_deref(),
        Some(project.as_str())
    );
    let source_path = dir.join("source.mp4");
    std::fs::write(&source_path, b"external fixture").unwrap();
    let source = identify_video_source(&source_path, ctx).unwrap();
    let ticket = provider.create_upload(&project, &source).await.unwrap();
    assert_eq!(ticket.upload_id, "upload");
    assert!(ticket.url.contains("memory-secret"));
    let result = provider.finalize(&project, &ticket.upload_id, None).await.unwrap();
    assert!(matches!(result,VideoImportResult::Ready(ref id) if id=="asset"));
    assert!(provider.probe(&project, "asset").await.unwrap().has_audio);
    assert_eq!(std::fs::read_to_string(root.join("mutations")).unwrap().lines().count(), 2);
    assert!(
        provider
            .choose_scope(&[
                scopes[0].clone(),
                CloudVideoScope { brand_id: None, store_id: Some("store".into()) }
            ])
            .await
            .is_err(),
        "cannot bypass allowed_tools for the question"
    );
    fn inspect(path: &std::path::Path) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                inspect(&path);
            } else if entry.file_type().unwrap().is_file() {
                // The external fixture code contains the literal URL; all generated artifacts must not.
                if path.file_name().is_some_and(|n| n == "occ.py") {
                    continue;
                }
                let bytes = std::fs::read(path).unwrap();
                assert!(!bytes.windows(13).any(|w| w == b"memory-secret"));
            }
        }
    }
    inspect(dir);
    assert!(
        !std::fs::read_dir(dir).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".video-memory-"))
    );
}


pub async fn video_edit_prepare_cloud_identity_is_independent_of_local_tenant_and_rejects_mismatch(provider: &ShijuCloudVideoToolProvider, _dir: &Path, root: PathBuf) {
    std::fs::write(root.join("company"), "10011").unwrap();
    std::fs::write(root.join("scripts/occ.py"), r#"import json,sys,pathlib
root=pathlib.Path(__file__).parent.parent
a=sys.argv[1:]
if a[0]=='schema':
 print(json.dumps({'name':a[1],'inputSchema':{'type':'object','properties':{}}}));sys.exit(0)
assert a[0]=='call' and a[1]=='get_my_permissions'
json.load(sys.stdin)
r={'member':{'companyId':(root/'company').read_text(),'status':'active','effectivePermissions':['project.read','project.create','material.read','material.create'],'brandIds':['brand'],'storeIds':[]}}
with open(a[a.index('--out')+1],'w') as f:json.dump(r,f)
"#).unwrap();
    assert_eq!(provider.authorized_scopes().await.unwrap().len(), 1);
    std::fs::write(root.join("company"), "10012").unwrap();
    assert!(
        provider.authorized_scopes().await.unwrap_err().to_string().contains("membership mismatch")
    );
}

const OCC: &str = r#"import json,sys,pathlib
args=sys.argv[1:];op=args[1]
fields={'read_transcript':{'editorProjectId':'string','itemId':'string','maxWordsPerPhrase':'integer','limit':'integer','offset':'integer'},'watch_timeline':{'projectId':'string','range':'object','times':'array','proxy':'boolean'},'read_captions':{'projectId':'string'},'read_project':{'editorProjectId':'string'},'media_probe':{'projectId':'string','assetId':'string','url':'string'},'import_media':{'projectId':'string','action':'string','urls':'array','uploadIds':'array','jobId':'string'},'edit_item':{'editorProjectId':'string','adds':'array','updates':'array'},'manage_transcript':{'projectId':'string','action':'string','assetId':'string','words':'array'},'clean_script':{'projectId':'string','mode':'string'},'edit_captions':{'projectId':'string','action':'string'},'edit_track':{'editorProjectId':'string','action':'string','json':'object'},'add_motion_graphic':{'editorProjectId':'string','templateName':'string','track':'string','startFrame':'number'},'search_templates':{'editorProjectId':'string','query':'string'},'get_share_link':{'projectId':'string','draft':'boolean'},'track_progress':{'jobId':'string'}}
if args[0]=='schema':
 def schema(t):return dict(type=t,items={'type':'object'}) if t=='array' else dict(type=t)
 print(json.dumps({'name':op,'inputSchema':{'type':'object','properties':{k:({'type':'array','items':{'type':'number'}} if k=='times' else schema(v)) for k,v in fields[op].items()}}}));sys.exit(0)
v=json.load(sys.stdin);root=pathlib.Path(__file__).parent.parent;f=root/'delivery-state.json'
mode=(root/'delivery-mode').read_text() if (root/'delivery-mode').exists() else 'normal'
s=json.loads(f.read_text()) if f.exists() else {'tracks':[{'id':'v1','alias':'V1','trackType':'video','hidden':False,'locked':False}],'items':[],'assets':[],'captions':None,'transcript':False,'posts':[]}
assert (v.get('projectId') or v.get('editorProjectId') or 'project')=='project'
result={'ok':True,'autoEditSession':{'committed':True,'status':'applied','revision':'v3-live-shape'}}
def video(start,end,at):return {'id':'source-'+str(start),'kind':'video','trackId':'v1','track':'V1','resolvedSourceAssetId':'source','sourceAssetId':'source','sourceStartFrame':start*30,'sourceDurationInFrames':(end-start)*30,'startFrame':at*30,'durationInFrames':(end-start)*30,'offline':False,'linkStatus':'linked','volume':0 if s['transcript'] else 1,'hasTranscript':s['transcript']}
if op=='read_project':result={'ok':True,'projectId':'project','timeline':{'id':'timeline','fps':30,'tracks':s['tracks'],'items':s['items'],'captions':s['captions'],'captionTracks':[]},'mediaPool':{'assets':s['assets']}}
elif op=='media_probe':result={'durationSec':(20 if mode=='noop' else 10) if 'url' in v else 20,'video':{'width':640,'height':360,'fps':30},'streams':[{'type':'video'},{'type':'audio'}]}
elif op=='read_transcript':result={'ok':True,'available':True,'timeUnit':'milliseconds','hasMore':False,'phrases':[{'text':'word','wordCount':1,'start':11000,'end':13000}]}
elif op=='read_captions':result={'enabled':True,'complete':True,'archived':False,'cards':[{'text':'word','startMs':0,'endMs':10000,'hidden':False}]}
elif op=='search_templates':result={'result':[{'name':v['query'],'category':'title-cards'}]}
elif op=='track_progress':result={'jobId':v['jobId'],'kind':'draft','status':'running' if mode=='pending' else 'succeeded','result':{'kind':'video','durationSeconds':20 if mode=='noop' else 10}}
elif op=='watch_timeline':result={'projectId':'project','proxyUrl':'https://private.invalid/proxy?Signature=fixture-secret','proxy':{'status':'succeeded','revision':'v3-live-shape','notAnExport':True}}
elif op=='get_share_link':result={'kind':'live','url':'https://example.test/s/real-contract-fixture','draft':{'jobId':'preview-job','status':'succeeded','revision':'v3-live-shape'}}
else:
 s['posts'].append(op)
 if op=='import_media' and v['action']=='finalize':
  assert v['uploadIds']==['original-upload'];result={'ok':True,'assetIds':['source'],'results':[{'ok':True,'assets':[{'assetId':'source'}]}]}
 elif op=='import_media':
  a={'id':'generated','name':v['urls'][0]['name'],'kind':'video','offline':False};s['assets'].append(a);result={'ok':True,'assetIds':['generated'],'results':[{'ok':True,'assets':[{'assetId':'generated'}]}]}
 elif op=='edit_track':s['tracks'].append({'id':'track-'+str(len(s['tracks'])),'name':v['json']['name'],'trackType':'video','hidden':False,'locked':False})
 elif op=='edit_item':
  for item in v.get('adds',[]):
   if item.get('assetId')=='source':s['items'].append(video(0,20,0));continue
   i={'id':'effect-'+str(len(s['items'])),'kind':item['type'],'trackId':item.get('trackId',item.get('track')),'startFrame':item['fromFrame'],'durationInFrames':item['durationInFrames'],'offline':False,'props':{'text':item.get('text')},'resolvedSourceAssetId':item.get('assetId'),'linkStatus':'linked'};s['items'].append(i)
  for update in v.get('updates',[]):
   item=next(i for i in s['items'] if i['id']==update['itemId']);item['durationInFrames']=update['durationInFrames']
 elif op=='manage_transcript':
  s['transcript']=True;s['items'][0]['hasTranscript']=True;s['items'][0]['volume']=0
  s['items'].append({'id':'speech','kind':'audio','trackId':'a1','startFrame':0,'durationInFrames':600,'resolvedSourceAssetId':'source','hasTranscript':True,'volume':1});s['tracks'].append({'id':'a1','hidden':False,'trackType':'audio'})
 elif op=='clean_script' and mode=='noop':result={'ok':True,'mode':'safe','applied':[],'candidates':[],'previewOnly':False,'stats':{'candidates':0,'applied':0}}
 elif op=='clean_script':
  s['items']=[video(3,8,0),video(10,15,5),{'id':'speech','kind':'audio','trackId':'a1','startFrame':0,'durationInFrames':300,'resolvedSourceAssetId':'source','hasTranscript':True,'volume':1}]
 elif op=='edit_captions':
  s['captions']={'enabled':True,'sourceItemId':'speech'}
  if mode=='partial':result['automaticCaptionAvoidance']={'ok':False,'error':'geometry unavailable'}
 elif op=='add_motion_graphic':s['items'].append({'id':'mg','name':v['templateName'],'kind':'motion-graphic','trackId':v['track'],'startFrame':v['startFrame'],'durationInFrames':90,'offline':False})
 else:raise ValueError(op)
f.write_text(json.dumps(s))
with open(args[args.index('--out')+1],'w') as out:json.dump(result,out)
"#;

pub async fn delivery_scenario(provider: &ShijuCloudVideoToolProvider, cut: PathBuf, _root: &Path, mode: &str) -> (anyhow::Result<crate::preview::VideoEditDelivery>, String) {
    std::fs::write(cut.join("scripts/occ.py"), OCC).unwrap();
    std::fs::write(cut.join("delivery-mode"), mode).unwrap();
    let transcript = VideoTranscript {
        asset_id: "source".into(),
        duration_sec: 20.0,
        fps: 30.0,
        text: "word".into(),
        words: vec![VideoTranscriptWord {
            word_index: 0,
            text: "word".into(),
            start: 11.0,
            end: 13.0,
        }],
        source: "verified-asr".into(),
        diagnostics: vec![],
    };
    let segments =
        crate::segments::segment_video_transcript(&transcript, Default::default()).unwrap();
    let effects = [VisualType::BRoll, VisualType::Keyword, VisualType::MotionGraphic]
        .into_iter()
        .enumerate()
        .map(|(i, visual_type)| VideoEditEffect {
            id: format!("e{i}"),
            chunk_id: segments[0].chunk_id.clone(),
            word_index: 0,
            end_word_index: 0,
            anchor: "word".into(),
            start: 11.0,
            end: 13.0,
            visual_type,
            assets: vec![
                if visual_type == VisualType::MotionGraphic {
                    "Progress Bar Title Card"
                } else {
                    "footage"
                }
                .into(),
            ],
        })
        .collect();
    let plan = VideoEditPlan { asset_id: "source".into(), duration_sec: 20.0, effects };
    let recipes = BTreeMap::from([(
        "footage".into(),
        VideoAssetRecipe {
            prompt: "neutral validation".into(),
            kind: "environment".into(),
            model: "wan3.0-video".into(),
            duration_sec: 2,
            ratio: "16:9".into(),
            resolution: "480P".into(),
        },
    )]);
    let one = VideoEditPlan { effects: vec![plan.effects[0].clone()], ..plan.clone() };
    let digest = video_asset_requests(&one, "project", &recipes).unwrap()[0].digest.clone();
    let registry = VideoAssetRegistry {
        references: BTreeMap::from([(digest, "gen:ready".into())]),
        progress: VideoAssetProgress { completed: 1, ..Default::default() },
    };
    let materials =
        provider.resolve_video_materials("project", &plan, &recipes, &registry).await.unwrap();
    let requirements = VideoPlanRequirements {
        instructions: "source edit".into(),
        required_types: vec![VisualType::BRoll, VisualType::Keyword, VisualType::MotionGraphic],
        max_effects_per_word: 3,
    };
    let cancel = CancellationToken::new();
    let template = "native snapshot";
    use sha2::{Digest, Sha256};
    let template_sha256 = hex::encode(Sha256::digest(template));
    let plan_sha256 = video_plan_sha256(&plan).unwrap();
    let input = VideoApplyInput {
        project_id: "project",
        template_sha256: &template_sha256,
        plan_sha256: &plan_sha256,
        plan: &plan,
        transcript: &transcript,
        segments: &segments,
        requirements: &requirements,
        template_snapshot: template,
        registry: &registry,
        materials: &materials,
        cancel: &cancel,
    };

    let store = VideoEditManifestStore::new(
        GlobalDatabase::new(_root.join("journal.sqlite")).unwrap(),
    )
    .unwrap();
    let journal = store.project_journal(Default::default()).unwrap();
    let delivery = provider.deliver_video_edit(&journal, input, &recipes).await;
    let capture = std::fs::read_to_string(cut.join("delivery-state.json")).unwrap();
    assert!(!capture.contains("submit_export"));
    assert!(!capture.contains("Signature"));
    (delivery, capture)
}

use crate::{apply::*, planning::*};
use agent_database::GlobalDatabase;
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;
pub async fn finalize_receipt(provider: &ShijuCloudVideoToolProvider, cut: PathBuf) {
    std::fs::write(cut.join("scripts/occ.py"), OCC).unwrap();
    let result = VideoPrepareCloud::finalize(provider, "project", "original-upload", None).await.unwrap();
    assert!(matches!(result, VideoImportResult::Ready(id) if id == "source"));
}
