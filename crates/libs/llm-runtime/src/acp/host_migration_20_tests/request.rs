#[cfg(unix)]
#[tokio::test]
async fn agent_acp_defaults_reject_unsupported_mode() {
    let temp = tempfile::tempdir().unwrap();
    let script = temp.path().join("peer.py");
    let capture = temp.path().join("requests.jsonl");
    std::fs::write(&script, r#"
import json, sys
for line in sys.stdin:
    r=json.loads(line)
    if 'method' not in r or 'id' not in r: continue
    with open(sys.argv[1],'a') as f: f.write(line)
    result = {'protocolVersion':1,'agentCapabilities':{},'authMethods':[]} if r['method']=='initialize' else {'sessionId':'defaults'} if r['method']=='session/new' else {}
    print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':result}),flush=True)
"#).unwrap();
    let client = acp_client::AcpClient::new(
        "defaults",
        acp_types::AcpAgentConfig {
            command: "python3".into(),
            args: vec![script.display().to_string(), capture.display().to_string()],
            env: Default::default(),
        },
    );
    let session = client.create_session(temp.path()).await.unwrap();
    let error =
        super::super::request::apply_default_mode(&client, &session.session_id, temp.path(), "unadvertised".into())
            .await
            .unwrap_err();
    assert!(error.to_string().contains("ACP session mode is not advertised: unadvertised"));
    client.close().await.unwrap();
    let requests = std::fs::read_to_string(capture).unwrap();
    assert!(!requests.contains("session/set_mode"));
    assert!(!requests.contains("session/prompt"));
}
