use super::{apply::*, assets::*, manifest::*, planning::*, transcript::*};
use crate::global_database::GlobalDatabase;
use serde_json::json;
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;

// Response shapes are taken from neutral, isolated live calls. Only the official
// external process is substituted; native capability checks, memory transport,
// plan validation, project journal and independent delivery checks are real.
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
async fn native_scenario(
    mode: &str,
) -> (anyhow::Result<super::preview::VideoEditDelivery>, String) {
    let (_root, ctx, skill) = super::cloud_tests::fixture("ok");
    let provider = super::ShijuCloudVideoToolProvider::discover(
        ctx,
        std::slice::from_ref(&skill),
        Some(json!(["bash", "bash_wait", "bash_cancel"])),
    )
    .await
    .unwrap();
    let cut = skill.location.as_ref().unwrap().parent().unwrap().join("cutvideo");
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
        super::segments::segment_video_transcript(&transcript, Default::default()).unwrap();
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
        GlobalDatabase::new(_root.path().join("journal.sqlite")).unwrap(),
    )
    .unwrap();
    let journal = store.project_journal(Default::default()).unwrap();
    let delivery = provider.deliver_video_edit(&journal, input, &recipes).await;
    let capture = std::fs::read_to_string(cut.join("delivery-state.json")).unwrap();
    assert!(!capture.contains("submit_export"));
    assert!(!capture.contains("Signature"));
    (delivery, capture)
}

#[test]
fn video_edit_delivery_normalizes_live_media_probe_projection_without_losing_audio() {
    let probe=super::cloud::probe::normalize_media_probe(json!({"durationSec":2.262,"video":{"width":640,"height":360,"fps":10},"streams":[{"type":"video","codec":"h264"},{"type":"audio","codec":"aac"}]})).unwrap();
    assert_eq!(probe["streams"][0]["codec_type"], "video");
    assert_eq!(probe["streams"][0]["width"], 640);
    assert_eq!(probe["streams"][0]["avg_frame_rate"], 10);
    assert_eq!(probe["streams"][1]["codec_type"], "audio");
}

#[tokio::test]
async fn video_edit_delivery_native_adapter_applies_three_types_and_reads_real_contract_shapes() {
    let (delivery, _) = native_scenario("normal").await;
    let delivery = delivery.unwrap();
    assert_eq!(delivery.timeline.effect_ids.len(), 3);
    assert_eq!(delivery.source_mapping[1].timeline_start, 5.0);
}
#[tokio::test]
async fn video_edit_delivery_native_adapter_rejects_partial_caption_failure_and_pending_draft() {
    for mode in ["partial", "pending"] {
        let (result, _) = native_scenario(mode).await;
        assert!(result.is_err(), "accepted {mode}");
    }
}
#[tokio::test]
async fn video_edit_delivery_native_adapter_records_verified_empty_cleanup_separately() {
    let (result, _) = native_scenario("noop").await;
    let delivery = result.unwrap();
    assert_eq!(delivery.source_mapping.len(), 1);
    assert_eq!(delivery.timeline.duration_sec, 20.0);
}

#[tokio::test]
async fn video_edit_delivery_native_finalize_accepts_observed_complete_import_receipt() {
    let (_root, ctx, skill) = super::cloud_tests::fixture("ok");
    let provider = super::ShijuCloudVideoToolProvider::discover(
        ctx,
        std::slice::from_ref(&skill),
        Some(json!(["bash", "bash_wait", "bash_cancel"])),
    )
    .await
    .unwrap();
    let cut = skill.location.as_ref().unwrap().parent().unwrap().join("cutvideo");
    std::fs::write(cut.join("scripts/occ.py"), OCC).unwrap();
    let result =
        super::prepare::VideoPrepareCloud::finalize(&provider, "project", "original-upload", None)
            .await
            .unwrap();
    assert!(matches!(result,super::prepare::VideoImportResult::Ready(id) if id=="source"));
}
