use super::cloud::*;
use crate::skills::Skill;
use crate::tools::ToolRuntimeContext;
use crate::workflow::{WorkflowToolProvider, WorkflowToolRequest};
use serde_json::{Value, json};
use std::path::Path;

#[test]
fn video_edit_cloud_transport_hidden_skill_is_rejected() {
    assert!(
        visible_shiju_root(&[]).is_err(),
        "hidden shiju must never be discovered from another installation"
    );
}

fn skill(path: &Path) -> Skill {
    Skill { name: "shiju".into(), location: Some(path.join("SKILL.md")), ..Default::default() }
}

// Only the external CLI/occ process is replaced. Discovery, argv construction, private
// files, native registry, permission checks, and process lifecycle remain production code.
pub(super) fn fixture(mode: &str) -> (tempfile::TempDir, ToolRuntimeContext, Skill) {
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
    let root_text = Some(root.to_string_lossy().into_owned());
    let mut config = crate::app::agent::config::Config::default();
    config.workspace_dir = root.clone();
    config.config_path = root.join("config.toml");
    config.autonomy.allowed_commands = vec!["python3".into()];
    config.autonomy.workspace_only = true;
    let ctx = ToolRuntimeContext::new(format!("cloud-{}", uuid::Uuid::new_v4()), root_text)
        .with_test_config(config);
    let native = ctx.tool_use_context().as_ref().clone().with_full_access_enabled(true);
    let ctx = ctx.with_tool_use_context(native);
    (dir, ctx, skill(&hub))
}

fn request(name: &str, inputs: Value) -> WorkflowToolRequest {
    WorkflowToolRequest {
        provider: "shiju".into(),
        tool_name: name.into(),
        action: None,
        credential_id: None,
        inputs: serde_json::from_value(inputs).unwrap(),
    }
}

#[tokio::test]
async fn video_edit_cloud_transport_discovers_current_instance_and_full_private_receipt() {
    let (dir, ctx, skill) = fixture("ok");
    let provider = ShijuCloudVideoToolProvider::discover(ctx, std::slice::from_ref(&skill), None)
        .await
        .unwrap();
    assert_eq!(provider.versions(), ("2.1", "2.1-dev"));
    assert_eq!(provider.schemas().len(), 2);
    let id = "job-' $(touch escaped) `touch escaped` 中文";
    let result = provider.call(request("generation_status", json!({"taskId":id}))).await.unwrap();
    assert_eq!(result.result, json!({"taskId":id,"status":"running"}));
    let hub = skill.location.unwrap().parent().unwrap().to_path_buf();
    let captured: Value =
        serde_json::from_slice(&std::fs::read(hub.join("captured-args.json")).unwrap()).unwrap();
    assert_eq!(captured["taskId"], id);
    assert!(!dir.path().join("escaped").exists());
    let text = serde_json::to_string(&result.result).unwrap();
    assert!(!text.contains("fixture-private-credential"));
    assert!(!text.contains("Signature="));
    assert!(
        !std::fs::read_dir(dir.path()).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".video-cloud-"))
    );
}

#[tokio::test]
async fn video_edit_cloud_transport_version_source_and_authorization_fail_closed() {
    for (mode, expected) in [
        ("wrong-client", "来源不一致"),
        ("upgrade", "版本门禁"),
        ("unauthorized", "未安装或未授权"),
        ("ambiguous", "明确选择"),
        ("wrong-source", "消费范围不一致"),
        ("missing-provenance", "消费范围缺失"),
    ] {
        let (_dir, ctx, skill) = fixture(mode);
        let error = ShijuCloudVideoToolProvider::discover(ctx, &[skill], None).await.err().unwrap();
        assert!(error.contains(expected), "{mode}: {error}");
    }
}

#[tokio::test]
async fn video_edit_cloud_transport_missing_credentials_never_logs_in_or_falls_back() {
    let (_dir, ctx, skill) = fixture("ok");
    let hub = skill.location.as_ref().unwrap().parent().unwrap();
    std::fs::remove_file(hub.join(".env")).unwrap();
    let error = ShijuCloudVideoToolProvider::discover(ctx, std::slice::from_ref(&skill), None)
        .await
        .err()
        .unwrap();
    assert!(error.contains("缺少凭据"), "{error}");
    assert!(!hub.join("capture.jsonl").exists());
}

#[tokio::test]
async fn video_edit_cloud_transport_native_tool_and_path_denials_prevent_process() {
    let (_dir, ctx, skill) = fixture("ok");
    assert!(
        ShijuCloudVideoToolProvider::discover(
            ctx.clone(),
            std::slice::from_ref(&skill),
            Some(json!([]))
        )
        .await
        .is_err()
    );
    let native = ctx.tool_use_context().as_ref().clone().with_full_access_enabled(false);
    let ctx = ctx.with_tool_use_context(native);
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("SKILL.md"), "fixture").unwrap();
    let outside_skill = Skill {
        name: "shiju".into(),
        location: Some(outside.path().join("SKILL.md")),
        ..Default::default()
    };
    assert!(ShijuCloudVideoToolProvider::discover(ctx, &[outside_skill], None).await.is_err());
    assert!(!skill.location.unwrap().parent().unwrap().join("capture.jsonl").exists());
}

#[tokio::test]
async fn video_edit_cloud_transport_cancel_before_discovery_starts_no_process() {
    let (_dir, ctx, skill) = fixture("ok");
    let token = tokio_util::sync::CancellationToken::new();
    token.cancel();
    let native = ctx.tool_use_context().as_ref().clone().with_abort_token(token);
    let ctx = ctx.with_tool_use_context(native);
    let error = ShijuCloudVideoToolProvider::discover(ctx, std::slice::from_ref(&skill), None)
        .await
        .err()
        .unwrap();
    assert!(error.ends_with("cancelled"), "{error}");
    assert!(!skill.location.unwrap().parent().unwrap().join("capture.jsonl").exists());
}

#[tokio::test]
async fn video_edit_cloud_transport_errors_and_unknown_states_are_not_success() {
    let (_dir, ctx, skill) = fixture("ok");
    let hub = skill.location.as_ref().unwrap().parent().unwrap();
    let provider = ShijuCloudVideoToolProvider::discover(ctx, std::slice::from_ref(&skill), None)
        .await
        .unwrap();
    for mode in ["exit-1", "exit-2", "exit-3", "exit-4", "unknown-state"] {
        std::fs::write(hub.join("mode"), mode).unwrap();
        let error = provider
            .call(request("generation_status", json!({"taskId":"fixture-job"})))
            .await
            .unwrap_err();
        assert!(!error.contains("fixture-private-credential"));
        assert!(!error.contains("Signature="));
    }
    assert!(provider.call(request("export_project", json!({}))).await.is_err());
    assert!(
        provider
            .call(request("generation_status", json!({"taskId":"job","saveToLibrary":{}})))
            .await
            .is_err()
    );
    let mut overridden = request("get_my_permissions", json!({}));
    overridden.credential_id = Some("other-agent".into());
    assert!(provider.call(overridden).await.is_err());
}

#[test]
fn video_edit_cloud_transport_duplicate_visible_instances_are_ambiguous() {
    let (_dir, _ctx, skill) = fixture("ok");
    assert!(visible_shiju_root(&[skill.clone(), skill]).is_err());
}

#[tokio::test]
async fn video_edit_cloud_transport_no_project_uses_native_workspace() {
    let (_dir, mut ctx, skill) = fixture("ok");
    ctx.root = None;
    let native = ctx.tool_use_context().as_ref().clone();
    let ctx = ctx.with_tool_use_context(native);
    assert!(ShijuCloudVideoToolProvider::discover(ctx, &[skill], None).await.is_ok());
}

#[tokio::test]
async fn video_edit_cloud_transport_inflight_cancel_stops_process_before_private_cleanup() {
    let (dir, ctx, skill) = fixture("ok");
    let token = tokio_util::sync::CancellationToken::new();
    let native = ctx.tool_use_context().as_ref().clone().with_abort_token(token.clone());
    let provider = ShijuCloudVideoToolProvider::discover(
        ctx.with_tool_use_context(native),
        std::slice::from_ref(&skill),
        None,
    )
    .await
    .unwrap();
    let hub = skill.location.as_ref().unwrap().parent().unwrap().to_owned();
    std::fs::write(hub.join("mode"), "slow").unwrap();
    let ready = hub.join("captured-args.json");
    let watcher = std::thread::spawn(move || {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !ready.exists() && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let observed = ready.exists();
        token.cancel();
        observed
    });
    let error = provider
        .call(request("generation_status", json!({"taskId":"fixture-job"})))
        .await
        .unwrap_err();
    assert!(watcher.join().unwrap(), "fixture must reach the external process before cancellation");
    assert_eq!(error, "cancelled");
    assert!(
        !std::fs::read_dir(dir.path()).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".video-cloud-"))
    );
}

#[tokio::test]
async fn video_edit_cloud_transport_native_approval_denial_prevents_process() {
    use crate::app::agent::approval::ApprovalManager;
    use crate::app::agent::config::AutonomyConfig;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Deny(Arc<AtomicUsize>);
    #[async_trait::async_trait]
    impl crate::hooks::HookHandler for Deny {
        fn name(&self) -> &str {
            "cloud-approval-denial"
        }
        async fn permission_request(&self, _: &str, _: &Value) -> anyhow::Result<Option<bool>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Some(false))
        }
    }
    let (_dir, ctx, skill) = fixture("ok");
    let observed = Arc::new(AtomicUsize::new(0));
    let mut runner = crate::hooks::HookRunner::new();
    runner.register(Box::new(Deny(observed.clone())));
    let config = AutonomyConfig {
        level: crate::app::agent::security::AutonomyLevel::Supervised,
        always_ask: vec!["bash".into()],
        auto_approve: vec![],
        ..Default::default()
    };
    let native = ctx
        .tool_use_context()
        .as_ref()
        .clone()
        .with_approval(Arc::new(ApprovalManager::from_config(&config)))
        .with_hook_runner(Arc::new(runner));
    let error = ShijuCloudVideoToolProvider::discover(
        ctx.with_tool_use_context(native),
        std::slice::from_ref(&skill),
        None,
    )
    .await
    .err()
    .unwrap();
    assert!(error.contains("审批拒绝"), "{error}");
    assert_eq!(observed.load(Ordering::SeqCst), 1);
    assert!(!skill.location.unwrap().parent().unwrap().join("capture.jsonl").exists());
}

#[tokio::test]
async fn video_edit_cloud_transport_native_hooks_never_receive_private_payloads() {
    use std::sync::{Arc, Mutex};
    struct Audit(Arc<Mutex<Vec<String>>>);
    #[async_trait::async_trait]
    impl crate::hooks::HookHandler for Audit {
        fn name(&self) -> &str {
            "cloud-audit"
        }
        async fn before_tool_call(
            &self,
            name: String,
            args: Value,
        ) -> crate::hooks::HookResult<(String, Value)> {
            self.0.lock().unwrap().push(args.to_string());
            crate::hooks::HookResult::Continue((name, args))
        }
        async fn on_after_tool_call(
            &self,
            _: &str,
            result: &crate::tools::ToolResult,
            _: std::time::Duration,
        ) {
            self.0.lock().unwrap().push(format!("{:?}", result));
        }
    }
    let (_dir, ctx, skill) = fixture("ok");
    let events = Arc::new(Mutex::new(vec![]));
    let mut runner = crate::hooks::HookRunner::new();
    runner.register(Box::new(Audit(events.clone())));
    let native = ctx.tool_use_context().as_ref().clone().with_hook_runner(Arc::new(runner));
    let provider =
        ShijuCloudVideoToolProvider::discover(ctx.with_tool_use_context(native), &[skill], None)
            .await
            .unwrap();
    provider
        .call(request("generation_status", json!({"taskId":"fixture-sensitive-input"})))
        .await
        .unwrap();
    let events = events.lock().unwrap();
    assert!(events.len() > 10);
    for event in events.iter() {
        for private in [
            "fixture-sensitive-input",
            "fixture-private-credential",
            "Signature=secret",
            "truncated output",
        ] {
            assert!(!event.contains(private), "private payload in native audit");
        }
    }
}
