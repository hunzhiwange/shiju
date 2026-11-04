//! External occ process fixture; native policy, schemas, pipes, sockets and typed adapter are real.
use super::{manifest::*, prepare::*, *};
use serde_json::json;

#[tokio::test]
async fn video_edit_prepare_native_receipts_keep_signed_tickets_in_memory() {
    let (dir, ctx, skill) = super::cloud_tests::fixture("ok");
    let provider = ShijuCloudVideoToolProvider::discover(
        ctx.clone(),
        std::slice::from_ref(&skill),
        Some(json!(["bash", "bash_wait", "bash_cancel"])),
    )
    .await
    .unwrap();
    let hub = skill.location.unwrap().parent().unwrap().to_path_buf();
    let root = hub.join("cutvideo");
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
    let source_path = dir.path().join("source.mp4");
    std::fs::write(&source_path, b"external fixture").unwrap();
    let source = identify_video_source(&source_path, &ctx).unwrap();
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
    inspect(dir.path());
    assert!(
        !std::fs::read_dir(dir.path()).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".video-memory-"))
    );
}

#[tokio::test]
async fn video_edit_prepare_cloud_identity_is_independent_of_local_tenant_and_rejects_mismatch() {
    let (_dir, ctx, skill) = super::cloud_tests::fixture("ok");
    assert_ne!(ctx.tool_use_context().tenant_scope().company_id().as_ref(), "10011");
    let provider = ShijuCloudVideoToolProvider::discover(ctx, std::slice::from_ref(&skill), None)
        .await
        .unwrap();
    let root = skill.location.unwrap().parent().unwrap().join("cutvideo");
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
