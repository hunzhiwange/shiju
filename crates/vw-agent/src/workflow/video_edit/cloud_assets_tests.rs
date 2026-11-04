use super::{assets::*, assets_tests::fixture};
use serde_json::json;

#[tokio::test]
async fn video_edit_asset_jobs_native_adapter_reconciles_pages_and_reads_reference() {
    let (_dir, ctx, skill) = super::cloud_tests::fixture("ok");
    let provider = super::ShijuCloudVideoToolProvider::discover(
        ctx,
        std::slice::from_ref(&skill),
        Some(json!(["bash", "bash_wait", "bash_cancel"])),
    )
    .await
    .unwrap();
    let root = skill.location.unwrap().parent().unwrap().join("cutvideo");
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
    let (_dir2, _, _, requests) = fixture(1);
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
