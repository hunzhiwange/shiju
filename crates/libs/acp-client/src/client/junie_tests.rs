use super::*;

#[tokio::test]
async fn junie_confirms_safe_mode_before_the_managed_model() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("peer.py");
    std::fs::write(
        &script,
        r#"
import json, sys
expected = iter([('brave_mode', 'off'), ('model', 'v1:6:custom:custom:shiju')])
for line in sys.stdin:
    request = json.loads(line)
    if 'method' not in request:
        continue
    method = request['method']
    if method == 'initialize':
        result = {'protocolVersion': 1, 'agentCapabilities': {}, 'authMethods': []}
    elif method == 'session/new':
        result = {'sessionId': 'junie-session'}
    elif method == 'session/set_config_option':
        config, value = next(expected)
        assert (request['params']['configId'], request['params']['value']) == (config, value)
        result = {'configOptions': [{
            'id': config, 'name': config, 'type': 'select', 'currentValue': value,
            'options': [{'value': value, 'name': value}],
        }]}
    else:
        raise AssertionError(method)
    print(json.dumps({'jsonrpc': '2.0', 'id': request['id'], 'result': result}), flush=True)
"#,
    )
    .unwrap();
    let client = AcpClient::new(
        "junie",
        AcpAgentConfig {
            command: "python3".into(),
            args: vec![script.to_string_lossy().into_owned()],
            env: [("SHIJU_JUNIE_CONFIG".into(), "fixture".into())].into(),
        },
    );
    tokio::time::timeout(Duration::from_secs(10), async {
        let session = client.create_session(directory.path()).await.unwrap();
        client
            .set_session_model(session.session_id, directory.path(), "v1:6:custom:custom:shiju")
            .await
            .unwrap();
    })
    .await
    .unwrap();
    client.close().await.unwrap();
}
