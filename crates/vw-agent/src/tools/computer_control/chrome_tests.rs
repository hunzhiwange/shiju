use super::*;
use crate::computer_control::policy::ControlFeature;
use crate::tools::{ToolUseContext, execute_tool_from_registry, scope_tool_use_context};
use serde_json::json;

type ApprovalLog = Arc<parking_lot::Mutex<Vec<(String, Value)>>>;
struct Approvals(ApprovalLog);
#[async_trait::async_trait]
impl crate::hooks::HookHandler for Approvals {
    fn name(&self) -> &str {
        "cc10-fixture-approval"
    }
    async fn permission_request(&self, name: &str, input: &Value) -> anyhow::Result<Option<bool>> {
        self.0.lock().push((name.into(), input.clone()));
        Ok(Some(true))
    }
}
fn approval_context() -> (Arc<ToolUseContext>, ApprovalLog) {
    let seen = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let mut hooks = crate::hooks::HookRunner::new();
    hooks.register(Box::new(Approvals(seen.clone())));
    (Arc::new(ToolUseContext::new("cc10", None).with_hook_runner(Arc::new(hooks))), seen)
}
fn authority() -> (Arc<ControlHost>, Arc<RwLock<ComputerControlPolicy>>, ComputerControlRequest) {
    let mut p = ComputerControlPolicy::new(Default::default());
    p.set_enabled(&local_user(), ControlFeature::Chrome, true).unwrap();
    p.set_grant(
        &local_user(),
        GrantScope::ChromeOrigin("https://example.test".into()),
        Some(GrantDecision::Allow),
    )
    .unwrap();
    let p = Arc::new(RwLock::new(p));
    let host = Arc::new(ControlHost::new(p.clone(), Arc::new(AtomicBool::new(false))));
    let target =
        ComputerControlTarget::ChromeTab { tab_id: 7, origin: "https://example.test".into() };
    let ticket = host.issue(target.clone(), Duration::from_secs(60)).unwrap();
    let r = ComputerControlRequest {
        request_id: ControlRequestId("cc10".into()),
        session_id: ticket.session_id,
        permission_version: ticket.permission_version,
        target,
        action: ComputerControlAction::Chrome {
            command: ChromeCommand::Download {
                selector: "#download".into(),
                url: "https://example.test/file".into(),
            },
        },
    };
    (host, p, r)
}
#[tokio::test]
async fn cc_control_10_transfer_approval_is_fresh_and_redacts_file_bytes() {
    let (host, _, mut r) = authority();
    let (ctx, seen) = approval_context();
    r.action = ComputerControlAction::Chrome {
        command: ChromeCommand::Upload {
            selector: "#file".into(),
            file_name: "fixture.txt".into(),
            content_base64: "eA==".into(),
        },
    };
    for _ in 0..2 {
        scope_tool_use_context(ctx.clone(), approve_transfer(&host, &r)).await.unwrap();
        host.consume_transfer(&r).unwrap();
    }
    let entries = seen.lock();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|(name, args)| name == "computer_control.chrome_upload"
        && !args.to_string().contains("eA==")
        && args["transfer"]["bytes"] == 1));
}
#[tokio::test]
async fn cc_control_10_full_access_and_turn_bypass_cannot_grant_transfers() {
    let (host, _, r) = authority();
    let ctx = Arc::new(
        ToolUseContext::new("cc10", None)
            .with_full_access_enabled(true)
            .with_bypass_non_cli_approval_for_turn(true)
            .with_channel("slack"),
    );
    assert!(scope_tool_use_context(ctx, approve_transfer(&host, &r)).await.is_err());
    assert_eq!(host.consume_transfer(&r), Err(ControlErrorCode::PermissionDenied));
}
#[tokio::test]
async fn cc_control_10_site_approval_cannot_override_explicit_deny() {
    let (_, p, _) = authority();
    let (ctx, seen) = approval_context();
    p.write()
        .set_grant(
            &local_user(),
            GrantScope::ChromeOrigin("https://denied.test".into()),
            Some(GrantDecision::Deny),
        )
        .unwrap();
    assert!(authorize_site(&p, &ctx, "https://denied.test").await.is_err());
    assert!(seen.lock().is_empty());
    authorize_site(&p, &ctx, "https://new.test").await.unwrap();
    assert_eq!(seen.lock()[0].0, "computer_control.chrome_site");
}

#[tokio::test]
#[ignore = "requires macOS Chrome; isolated profile, controlled fixture and real Native Messaging"]
async fn cc_control_10_real_chrome_tool_transfer_and_redirect_denial() -> anyhow::Result<()> {
    use std::process::Stdio;
    use tokio::{io::AsyncBufReadExt, process::Command};
    let directory =
        PathBuf::from(std::env::var_os("HOME").unwrap()).join(".shiju/computer-control");
    anyhow::ensure!(
        !directory.exists(),
        "computer-control directory already owned; do not overwrite it"
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()?;
    let script = root.join("scripts/computer_control/chrome_native_host.py");
    anyhow::ensure!(
        Command::new("python3").arg(&script).arg("--install").status().await?.success(),
        "install failed"
    );
    let signal = tempfile::tempdir()?;
    let done = signal.path().join("done");
    let mut driver = Command::new("node")
        .current_dir(&root)
        .arg(root.join("scripts/computer_control/chrome_origin_live.mjs"))
        .env("SHIJU_CC10_DONE", &done)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()?;
    let mut lines = tokio::io::BufReader::new(driver.stdout.take().unwrap()).lines();
    let result: anyhow::Result<()> = async {
        let line=tokio::time::timeout(Duration::from_secs(20),lines.next_line()).await??.context("driver missing identity")?;
        let identity:Value=serde_json::from_str(&line)?;println!("real Chrome identity={identity}");
        let origin=canonical_origin(identity["fixtureUrl"].as_str().unwrap()).map_err(control_error)?;
        let mut p=ComputerControlPolicy::new(Default::default());p.set_enabled(&local_user(),ControlFeature::Chrome,true).unwrap();
        let runtime=NativeControlRuntime::with_factory(Arc::new(RwLock::new(p)),Arc::new(|_,_,_,_,_,_|panic!("Chrome must never enter native backend")));
        let tools:Vec<Box<dyn Tool>>=vec![Box::new(ComputerControlTool::new(runtime.clone(),Some("cc10".into())))];
        let (context,seen)=approval_context();
        let target=json!({"kind":"chrome_tab","tab_id":identity["tabId"],"origin":origin});
        let invoke=|command:Value| json!({"target":target,"action":{"action":"chrome","command":command}});
        let read=execute_tool_from_registry(&tools,"computer_control",invoke(json!({"kind":"snapshot"})),context.clone()).await?;
        anyhow::ensure!(read.result.data.to_string().contains("controlled fixture"),"fixture read failed");
        use base64::Engine;
        let bytes=base64::engine::general_purpose::STANDARD.encode(b"cc10-upload-fixture");
        let upload=execute_tool_from_registry(&tools,"computer_control",invoke(json!({"kind":"upload","selector":"#file","file_name":"fixture.txt","content_base64":bytes})),context.clone()).await?;
        anyhow::ensure!(upload.result.is_success(),"upload failed");
        let downloaded=execute_tool_from_registry(&tools,"computer_control",invoke(json!({"kind":"download","selector":"#download","url":format!("{origin}/download")})),context.clone()).await?;
        let encoded=downloaded.result.data.pointer("/output/content_base64").and_then(Value::as_str).context("download data missing")?;
        anyhow::ensure!(base64::engine::general_purpose::STANDARD.decode(encoded)?==b"cc10-download-fixture","download bytes mismatch");
        let navigation=execute_tool_from_registry(&tools,"computer_control",invoke(json!({"kind":"navigate","url":format!("{origin}/redirect")})),context.clone()).await?;
        anyhow::ensure!(navigation.result.is_success(),"navigation failed");
        tokio::time::sleep(Duration::from_millis(300)).await;
        let denied=execute_tool_from_registry(&tools,"computer_control",invoke(json!({"kind":"snapshot"})),context.clone()).await;
        anyhow::ensure!(denied.is_err(),"old handle survived redirect");println!("real Chrome redirect refused old handle: {}",denied.err().unwrap());
        anyhow::ensure!(seen.lock().iter().filter(|(name,_)|name=="computer_control.chrome_upload").count()==1,"upload approval missing");
        anyhow::ensure!(seen.lock().iter().filter(|(name,_)|name=="computer_control.chrome_download").count()==1,"download approval missing");
        runtime.stop(&(context.tenant_scope(),"cc10".into(),None)).await;
        Ok(())
    }.await;
    std::fs::write(&done, b"done")?;
    let status = tokio::time::timeout(Duration::from_secs(15), driver.wait()).await??;
    while let Some(line) = lines.next_line().await? {
        println!("{line}");
    }
    let removed = Command::new("python3").arg(&script).arg("--uninstall").status().await?;
    // This test created the entire directory after the exclusive nonexistence check.
    std::fs::remove_dir_all(&directory)?;
    result?;
    anyhow::ensure!(status.success() && removed.success(), "driver verification/cleanup failed");
    Ok(())
}

#[tokio::test]
async fn cc_control_10_generic_approval_audit_does_not_record_inline_file_bytes() {
    let runtime = NativeControlRuntime::with_factory(
        Arc::new(RwLock::new(ComputerControlPolicy::new(Default::default()))),
        Arc::new(|_, _, _, _, _, _| panic!("must not reach native backend")),
    );
    let tools: Vec<Box<dyn Tool>> =
        vec![Box::new(ComputerControlTool::new(runtime, Some("cc10".into())))];
    let manager =
        Arc::new(crate::approval::ApprovalManager::from_config(&crate::config::AutonomyConfig {
            level: crate::security::AutonomyLevel::Supervised,
            always_ask: vec!["computer_control".into()],
            ..Default::default()
        }));
    let context = Arc::new(
        ToolUseContext::new("cc10", None).with_approval(manager.clone()).with_channel("slack"),
    );
    let input = json!({"target":{"kind":"chrome_tab","tab_id":7,"origin":"https://example.test"},"action":{"action":"chrome","command":{"kind":"upload","selector":"#file","file_name":"fixture.txt","content_base64":"eA=="}}});
    assert!(execute_tool_from_registry(&tools, "computer_control", input, context).await.is_err());
    let audit = manager.audit_log();
    assert_eq!(audit.len(), 1);
    assert!(!audit[0].arguments_summary.contains("eA=="));
}

#[tokio::test]
async fn cc_control_10_cancelled_connection_never_waits_for_extension() {
    let (_, policy, _) = authority();
    let runtime = NativeControlRuntime::with_factory(
        policy,
        Arc::new(|_, _, _, _, _, _| panic!("Chrome cannot use native backend")),
    );
    let tool = ComputerControlTool::new(runtime, Some("cc10".into()));
    let cancel = CancellationToken::new();
    cancel.cancel();
    let context = Arc::new(ToolUseContext::new("cc10", None).with_abort_token(cancel));
    let input = json!({"target":{"kind":"chrome_tab","tab_id":7,"origin":"https://example.test"},"action":{"action":"chrome","command":{"kind":"snapshot"}}});
    let error = scope_tool_use_context(context, tool.call(input)).await.unwrap_err();
    assert!(error.to_string().contains("Cancelled"), "{error}");
}
