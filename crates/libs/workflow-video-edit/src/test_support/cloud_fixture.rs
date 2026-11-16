//! Official external receipt scripts, executed only by the real native tool host.
pub fn fixture(mode: &str) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let hub = root.join("client ' $(touch escaped) 中文");
    std::fs::create_dir_all(hub.join("scripts")).unwrap();
    std::fs::create_dir_all(hub.join("other")).unwrap();
    std::fs::create_dir_all(hub.join("cutvideo/scripts")).unwrap();
    std::fs::write(hub.join("SKILL.md"), "fixture").unwrap();
    std::fs::write(hub.join(".env"), "SHIJU_KEY=fixture-private-credential\n").unwrap();
    std::fs::write(hub.join("mode"), mode).unwrap();
    let script = r#"#!/usr/bin/env python3
import json, pathlib, sys, time
root = pathlib.Path.cwd()
mode = (root/'mode').read_text()
args = sys.argv[1:]
with (root/'capture.jsonl').open('a') as f: f.write(json.dumps(args)+'\n')
if args[0]=='hub:version':
 d={'package_root':str(root),'current_version':'8.5.1','package_skill_version':'8.5.1'}
 if mode=='wrong-client':d['package_root']=str(root/'other')
elif args[0]=='company:info':d={'company_id':'10011'}
elif args[0]=='hub:check':d={'force_upgrade_required':mode=='upgrade'}
elif args[:2]==['index','rebuild']:d={}
elif args[0]=='skill:me':
 item={'skill_id':712,'slug':'cutvideo','installed_version':'2.1','distribution_source':'tenant','consumer_company_id':'fixture'}
 if mode=='missing-provenance':del item['consumer_company_id']
 d={'items':[] if mode=='unauthorized' else [item,item] if mode=='ambiguous' else [item]}
elif args[0]=='skill:fetch':
 assert args==['skill:fetch','--skill-id','712','--in-use']
 d={'skill_id':'712','slug':'cutvideo','installed_version':'2.1','active_version':'2.1-dev','source':'dev','distribution_source':'tenant','consumer_company_id':'fixture','path':str(root/'cutvideo')}
 if mode=='wrong-source':d['distribution_source']='marketplace'
else:raise RuntimeError('unrecognized CLI operation')
print(json.dumps({'code':200,'data':d}))
"#;
    std::fs::write(hub.join("scripts/shiju"), script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(hub.join("scripts/shiju"), std::fs::Permissions::from_mode(0o700))
            .unwrap();
    }
    std::fs::write(hub.join("cutvideo/scripts/occ.py"), r#"import json, pathlib, sys, time
root=pathlib.Path.cwd()
a=sys.argv[1:]
mode=(root/'mode').read_text()
with (root/'capture.jsonl').open('a') as f:f.write(json.dumps(a)+'\n')
if a==['--help']:print('--args-file --out --no-wait');sys.exit(0)
if a==['tools']:print('get_my_permissions\tpermission read\ngeneration_status\tjob read');sys.exit(0)
if a[0]=='schema':
 print(json.dumps({'name':a[1],'inputSchema':{'type':'object','properties':{} if a[1]=='get_my_permissions' else {'taskId':{'type':'string'}}}}));sys.exit(0)
assert a[0]=='call' and '--no-wait' in a
args=json.loads(pathlib.Path(a[a.index('--args-file')+1]).read_text())
(root/'captured-args.json').write_text(json.dumps(args))
if mode=='slow':time.sleep(60)
if mode.startswith('exit-'):print('fixture-private-credential https://example.test/a?Signature=secret');sys.exit(int(mode[5:]))
result={'member':{'role':'fixture'},'padding':'x'*9000,'token':'fixture-private-credential','audioUrl':'https://example.test/a?Signature=secret'}
if a[1]=='generation_status':result.update(taskId=args['taskId'],status='unexpected' if mode=='unknown-state' else 'running')
pathlib.Path(a[a.index('--out')+1]).write_text(json.dumps(result))
print('{truncated output')
"#).unwrap();
    (dir, root, hub)
}
