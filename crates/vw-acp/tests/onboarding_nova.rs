use vw_acp::built_in_agent_specs;

#[test]
fn acp_onboarding_nova_registry_contract() {
    let specs = built_in_agent_specs();
    let spec = specs.get("nova").expect("Nova must be registered");
    assert_eq!(spec.display_name, "Nova");
    if spec.command == "npx" {
        assert_eq!(spec.args, ["-y", "@compass-ai/nova@1.1.65", "acp"]);
    } else {
        assert_eq!(std::path::Path::new(&spec.command).file_stem().unwrap(), "nova");
        assert_eq!(spec.args, ["acp"]);
    }
}

async fn run_peer(deny: bool) {
    use vw_acp::{AcpAgentConfig, AcpClient, PromptRequest, SessionStrategy};
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("peer.py");
    let capture = directory.path().join("frames.jsonl");
    std::fs::write(&script, r#"
import json,sys,os
assert os.path.realpath(os.getcwd())==os.path.realpath(os.path.dirname(sys.argv[1]))
for line in sys.stdin:
 r=json.loads(line)
 with open(sys.argv[1],'a') as f:f.write(line)
 if 'method' not in r:continue
 m=r['method']
 if m=='initialize':
  assert r['params']['clientCapabilities']['_meta']['compass.owner-approval.v1'] is True
  result={'protocolVersion':1,'agentCapabilities':{'_meta':{'compass.owner-approval.v1':True}},'_meta':{'ownerApprovalRequired':True},'authMethods':[]}
 elif m=='session/new':
  assert r['params']['_meta']['compass.owner-approval.v1'] is True
  result={'sessionId':'nova-fixture','_meta':{'ownerApprovalRequired':True}}
 elif m=='session/prompt':
  if r['params']['prompt'][0]['text']=='cancel':
   cancel=json.loads(sys.stdin.readline())
   with open(sys.argv[1],'a') as f:f.write(json.dumps(cancel)+'\n')
   assert cancel['method']=='session/cancel',cancel
   print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':{'stopReason':'cancelled'}}),flush=True)
   continue
  if sys.argv[2]=='deny':
   print(json.dumps({'jsonrpc':'2.0','id':900,'method':'permission/request','params':{'sessionId':'nova-fixture','toolCall':{'toolCallId':'edit','title':'edit file','kind':'edit','status':'pending'},'options':[{'optionId':'reject','kind':'reject_once','name':'Reject'}]}}),flush=True)
   response=json.loads(sys.stdin.readline())
   with open(sys.argv[1],'a') as f:f.write(json.dumps(response)+'\n')
   assert response['result']['outcome']['optionId']=='reject',response
  result={'stopReason':'end_turn'}
 else:result={}
 if 'id' in r:print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':result}),flush=True)
"#).unwrap();
    let client = AcpClient::new(
        "nova",
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
        assert_eq!(
            frames
                .iter()
                .filter(|r| r["id"] == 900 && r["result"]["outcome"]["optionId"] == "reject")
                .count(),
            2
        );
        assert!(!capture.with_extension("jsonl.denied").exists());
    }
}

#[tokio::test]
async fn acp_onboarding_nova_session_lifecycle() {
    run_peer(false).await;
}

#[tokio::test]
async fn acp_onboarding_nova_permission_denial() {
    run_peer(true).await;
}

#[tokio::test]
async fn acp_onboarding_nova_missing_executable_fails() {
    let directory = tempfile::tempdir().unwrap();
    let client = vw_acp::AcpClient::new(
        "nova",
        vw_acp::AcpAgentConfig {
            command: directory.path().join("missing-nova").display().to_string(),
            args: vec![],
            env: Default::default(),
        },
    );
    assert!(client.create_session(directory.path()).await.is_err());
    client.close().await.unwrap();
}

// The official CLI and an already authorized, isolated HOME are supplied by the operator.
#[tokio::test]
#[ignore = "requires the fixed official Nova npm package and isolated local consent"]
async fn acp_onboarding_nova_real_cli_permissions() {
    use vw_acp::{AcpAgentConfig, AcpClient, PermissionMode, PromptRequest, SessionStrategy};
    let home = std::env::var("SHIJU_NOVA_TEST_HOME").unwrap();
    let entry = std::env::var("SHIJU_NOVA_TEST_ENTRY").unwrap();
    for (mode, expected) in [
        (PermissionMode::DenyAll, "before"),
        (PermissionMode::ApproveReads, "before"),
        (PermissionMode::ApproveAll, "after"),
    ] {
        let work = tempfile::tempdir().unwrap();
        std::fs::write(work.path().join("fixture.txt"), "before").unwrap();
        let client = AcpClient::new(
            "nova",
            AcpAgentConfig {
                command: "python3".into(),
                args: vec![
                    "-u".into(),
                    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/nova_real_peer.py").into(),
                ],
                env: std::collections::HashMap::from([
                    ("HOME".into(), home.clone()),
                    ("SHIJU_NOVA_TEST_ENTRY".into(), entry.clone()),
                ]),
            },
        )
        .with_permission_mode(mode);
        tokio::time::timeout(std::time::Duration::from_secs(60), async {
            let session = client.create_session(work.path()).await.unwrap();
            assert!(
                client
                    .set_session_model(&session.session_id, work.path(), "unknown-fixture-model")
                    .await
                    .is_err()
            );
            client
                .set_session_model(&session.session_id, work.path(), "anthropic/claude-haiku-5-5")
                .await
                .unwrap();
            for turn in 0..2 {
                let result = client
                    .run_prompt(
                        PromptRequest::new(work.path().to_path_buf(), "controlled fixture")
                            .with_session_strategy(SessionStrategy::ResumeOrLoad(
                                session.session_id.clone(),
                            )),
                        &mut |_| {},
                    )
                    .await
                    .unwrap();
                assert_eq!(result.session_id, session.session_id);
                if turn == 0 {
                    assert_eq!(
                        std::fs::read_to_string(work.path().join("fixture.txt")).unwrap(),
                        expected
                    );
                    if expected != "after" {
                        assert!(client.permission_stats().denied > 0);
                    }
                }
            }
            let mut callback = |_| {};
            let request = PromptRequest::new(work.path().to_path_buf(), "slow fixture")
                .with_session_strategy(SessionStrategy::ResumeOrLoad(session.session_id.clone()));
            let prompt = client.run_prompt(request, &mut callback);
            let cancel = async {
                while !client.has_active_prompt() {
                    tokio::task::yield_now().await;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                assert!(client.cancel(&session.session_id).await.unwrap());
            };
            let (result, ()) = tokio::join!(prompt, cancel);
            result.unwrap();
            client
                .run_prompt(
                    PromptRequest::new(work.path().to_path_buf(), "continue fixture")
                        .with_session_strategy(SessionStrategy::ResumeOrLoad(
                            session.session_id.clone(),
                        )),
                    &mut |_| {},
                )
                .await
                .unwrap();
            client.close().await.unwrap();
            let restored = client
                .run_prompt(
                    PromptRequest::new(work.path().to_path_buf(), "restore fixture")
                        .with_session_strategy(SessionStrategy::Load(session.session_id.clone())),
                    &mut |_| {},
                )
                .await
                .unwrap();
            assert_eq!(restored.session_id, session.session_id);
            assert_eq!(std::fs::read_to_string(work.path().join("fixture.txt")).unwrap(), expected);
        })
        .await
        .unwrap();
        client.close().await.unwrap();
    }
}

#[tokio::test]
async fn acp_onboarding_nova_unverified_approval_capability_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("unverified.py");
    std::fs::write(&script, r#"
import json,sys
r=json.loads(sys.stdin.readline())
print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':{'protocolVersion':1,'agentCapabilities':{},'authMethods':[]}}),flush=True)
for line in sys.stdin: pass
"#).unwrap();
    let client = vw_acp::AcpClient::new(
        "nova",
        vw_acp::AcpAgentConfig {
            command: "python3".into(),
            args: vec!["-u".into(), script.display().to_string()],
            env: Default::default(),
        },
    );
    let error = client.create_session(directory.path()).await.unwrap_err();
    assert!(error.to_string().contains("mandatory owner approval"));
    client.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires the fixed official Nova npm package and isolated local consent"]
async fn acp_onboarding_nova_real_cli_provider_errors() {
    for status in ["401", "404"] {
        let work = tempfile::tempdir().unwrap();
        let client = vw_acp::AcpClient::new(
            "nova",
            vw_acp::AcpAgentConfig {
                command: "python3".into(),
                args: vec![
                    "-u".into(),
                    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/nova_real_peer.py").into(),
                ],
                env: std::collections::HashMap::from([
                    ("HOME".into(), std::env::var("SHIJU_NOVA_TEST_HOME").unwrap()),
                    (
                        "SHIJU_NOVA_TEST_ENTRY".into(),
                        std::env::var("SHIJU_NOVA_TEST_ENTRY").unwrap(),
                    ),
                    ("SHIJU_NOVA_TEST_HTTP_STATUS".into(), status.into()),
                    (
                        "SHIJU_NOVA_TEST_CONTACT".into(),
                        work.path().join("contact").display().to_string(),
                    ),
                ]),
            },
        );
        let session = client.create_session(work.path()).await.unwrap();
        client
            .set_session_model(&session.session_id, work.path(), "anthropic/claude-haiku-5-5")
            .await
            .unwrap();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            client.run_prompt(
                vw_acp::PromptRequest::new(work.path().to_path_buf(), "controlled failure")
                    .with_session_strategy(vw_acp::SessionStrategy::ResumeOrLoad(
                        session.session_id,
                    )),
                &mut |_| {},
            ),
        )
        .await
        .unwrap();
        client.close().await.unwrap();
        let models = std::fs::read_to_string(work.path().join("contact"))
            .expect("provider boundary must actually run");
        assert!(!models.is_empty());
        assert!(
            models.lines().all(|model| model == "claude-haiku-5-5"),
            "selected model must not fall back: {models}"
        );
        assert!(result.is_err(), "failed native turn must not become host success");
    }
}

#[tokio::test]
async fn acp_onboarding_nova_unsuccessful_turns_are_errors() {
    for status in ["failed", "queued"] {
        let work = tempfile::tempdir().unwrap();
        let script = work.path().join("unsuccessful.py");
        std::fs::write(&script, r#"
import json,sys
for line in sys.stdin:
 r=json.loads(line)
 if r.get('method')=='initialize': result={'protocolVersion':1,'agentCapabilities':{'_meta':{'compass.owner-approval.v1':True}},'_meta':{'ownerApprovalRequired':True},'authMethods':[]}
 elif r.get('method')=='session/new': result={'sessionId':'unsuccessful','_meta':{'ownerApprovalRequired':True}}
 elif r.get('method')=='session/prompt': result={'stopReason':'end_turn','_meta':{'compass':{'turnOutcome':{'status':sys.argv[1]}}}}
 else: result={}
 if 'id' in r: print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':result}),flush=True)
"#).unwrap();
        let client = vw_acp::AcpClient::new(
            "nova",
            vw_acp::AcpAgentConfig {
                command: "python3".into(),
                args: vec!["-u".into(), script.display().to_string(), status.into()],
                env: Default::default(),
            },
        );
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            client.run_prompt(
                vw_acp::PromptRequest::new(work.path().to_path_buf(), "fixture"),
                &mut |_| {},
            ),
        )
        .await
        .unwrap();
        client.close().await.unwrap();
        assert!(result.is_err(), "{status} must not become success");
    }
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "requires the official Nova package and isolated consent"]
async fn acp_onboarding_nova_real_cli_command_denial() {
    use vw_acp::{AcpAgentConfig, AcpClient, PermissionMode, PromptRequest, SessionStrategy};
    for mode in [PermissionMode::DenyAll, PermissionMode::ApproveReads] {
        let work = tempfile::tempdir().unwrap();
        let marker = work.path().join("command-marker");
        let command =
            format!("printf marker > '{}'", marker.display().to_string().replace('\'', "'\\''"));
        let client = AcpClient::new(
            "nova",
            AcpAgentConfig {
                command: "python3".into(),
                args: vec![
                    "-u".into(),
                    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/nova_real_peer.py").into(),
                ],
                env: std::collections::HashMap::from([
                    ("HOME".into(), std::env::var("SHIJU_NOVA_TEST_HOME").unwrap()),
                    (
                        "SHIJU_NOVA_TEST_ENTRY".into(),
                        std::env::var("SHIJU_NOVA_TEST_ENTRY").unwrap(),
                    ),
                    ("SHIJU_NOVA_TEST_TOOL".into(), "command".into()),
                    ("SHIJU_NOVA_TEST_COMMAND".into(), command),
                ]),
            },
        )
        .with_permission_mode(mode);
        tokio::time::timeout(std::time::Duration::from_secs(60), async {
            let session = client.create_session(work.path()).await.unwrap();
            client
                .set_session_model(&session.session_id, work.path(), "anthropic/claude-haiku-5-5")
                .await
                .unwrap();
            let result = client
                .run_prompt(
                    PromptRequest::new(work.path().to_path_buf(), "controlled fixture")
                        .with_session_strategy(SessionStrategy::ResumeOrLoad(
                            session.session_id.clone(),
                        )),
                    &mut |_| {},
                )
                .await;
            if let Err(error) = result {
                assert!(error.to_string().contains("Nova did not complete"), "{error}");
            }
            assert!(client.permission_stats().denied > 0);
            assert!(!marker.exists(), "denied command must have no effect");
            client
                .run_prompt(
                    PromptRequest::new(work.path().to_path_buf(), "continue fixture")
                        .with_session_strategy(SessionStrategy::ResumeOrLoad(session.session_id)),
                    &mut |_| {},
                )
                .await
                .unwrap();
        })
        .await
        .unwrap();
        client.close().await.unwrap();
    }
}
