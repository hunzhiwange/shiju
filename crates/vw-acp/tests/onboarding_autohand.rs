use vw_acp::built_in_agent_specs;

#[test]
fn acp_onboarding_autohand_registry_contract() {
    let specs = built_in_agent_specs();
    let spec = specs.get("autohand").expect("Autohand must be registered");
    assert_eq!(spec.display_name, "Autohand Code");
    assert_eq!(spec.command, "npx");
    assert_eq!(spec.args, ["-y", "autohand-cli@0.9.11", "--acp"]);
    assert_eq!(spec.env["AUTOHAND_SKIP_AHTRACES_INSTALL"], "1");
    assert_eq!(spec.env["AUTOHAND_SKIP_COMPUTER_CONTROL_INSTALL"], "1");
}

async fn run_peer(deny: bool) {
    use vw_acp::{AcpAgentConfig, AcpClient, PromptRequest, SessionStrategy};
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("peer.py");
    let capture = directory.path().join("frames.jsonl");
    std::fs::write(&script, r#"
import json,sys
for line in sys.stdin:
 r=json.loads(line)
 with open(sys.argv[1],'a') as f:f.write(line)
 if 'method' not in r:continue
 m=r['method']
 if m=='initialize':result={'protocolVersion':1,'agentCapabilities':{},'authMethods':[]}
 elif m=='session/new':result={'sessionId':'autohand-fixture'}
 elif m=='session/prompt':
  if r['params']['prompt'][0]['text']=='cancel':
   cancel=json.loads(sys.stdin.readline())
   with open(sys.argv[1],'a') as f:f.write(json.dumps(cancel)+'\n')
   assert cancel['method']=='session/cancel',cancel
   print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':{'stopReason':'cancelled'}}),flush=True)
   continue
  if sys.argv[2]=='deny':
   print(json.dumps({'jsonrpc':'2.0','id':900,'method':'fs/write_text_file','params':{'sessionId':'autohand-fixture','path':sys.argv[1]+'.denied','content':'denied'}}),flush=True)
   response=json.loads(sys.stdin.readline())
   with open(sys.argv[1],'a') as f:f.write(json.dumps(response)+'\n')
   assert 'error' in response,response
  result={'stopReason':'end_turn'}
 else:result={}
 if 'id' in r:print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':result}),flush=True)
"#).unwrap();
    let client = AcpClient::new(
        "autohand",
        AcpAgentConfig {
            command: "python3".into(),
            args: vec![
                "-u".into(),
                script.display().to_string(),
                capture.display().to_string(),
                if deny { "deny" } else { "allow" }.into(),
            ],
            env: Default::default(),
        },
    )
    .with_permission_mode(vw_acp::PermissionMode::DenyAll);
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        let session = client.create_session(directory.path()).await.unwrap();
        for _ in 0..2 {
            let response = client
                .run_prompt(
                    PromptRequest::new(directory.path().to_path_buf(), "fixture")
                        .with_session_strategy(SessionStrategy::ResumeOrLoad(
                            session.session_id.clone(),
                        )),
                    &mut |_| {},
                )
                .await
                .unwrap();
            assert_eq!(response.session_id, session.session_id);
        }
        if !deny {
            let request = PromptRequest::new(directory.path().to_path_buf(), "cancel")
                .with_session_strategy(SessionStrategy::ResumeOrLoad(session.session_id.clone()));
            let mut callback = |_| {};
            let prompt = client.run_prompt(request, &mut callback);
            let cancel = async {
                while !client.has_active_prompt() {
                    tokio::task::yield_now().await;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                assert!(client.cancel(&session.session_id).await.unwrap());
            };
            let (outcome, ()) = tokio::join!(prompt, cancel);
            outcome.unwrap();
            client
                .run_prompt(
                    PromptRequest::new(directory.path().to_path_buf(), "after cancel")
                        .with_session_strategy(SessionStrategy::ResumeOrLoad(
                            session.session_id.clone(),
                        )),
                    &mut |_| {},
                )
                .await
                .unwrap();
        }
    })
    .await
    .unwrap();
    client.close().await.unwrap();
    let frames: Vec<serde_json::Value> = std::fs::read_to_string(&capture)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(frames.iter().filter(|r| r["method"] == "initialize").count(), 1);
    assert_eq!(frames.iter().filter(|r| r["method"] == "session/new").count(), 1);
    assert_eq!(
        frames.iter().filter(|r| r["method"] == "session/prompt").count(),
        if deny { 2 } else { 4 }
    );
    if !deny {
        assert!(frames.iter().any(|r| r["method"] == "session/cancel"));
    }
    if deny {
        assert_eq!(frames.iter().filter(|r| r["id"] == 900 && r.get("error").is_some()).count(), 2);
        assert!(!capture.with_extension("jsonl.denied").exists());
    }
}

#[tokio::test]
async fn acp_onboarding_autohand_session_lifecycle() {
    run_peer(false).await;
}

#[tokio::test]
async fn acp_onboarding_autohand_permission_denial() {
    run_peer(true).await;
}
