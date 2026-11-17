use super::*;

async fn model_control(wrong_readback: bool) -> (Result<(), AcpError>, Vec<serde_json::Value>) {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("peer.py");
    let frames = directory.path().join("frames.jsonl");
    std::fs::write(&script, r#"
import json,sys
for line in sys.stdin:
 r=json.loads(line)
 with open(sys.argv[1],'a') as f:f.write(line)
 if 'method' not in r:continue
 m=r['method']
 if m=='initialize':result={'protocolVersion':1,'agentCapabilities':{},'authMethods':[]}
 elif m=='session/new':result={'sessionId':'harn-control'}
 elif m=='session/set_config_option':
  value='wrong-model' if sys.argv[2]=='wrong' else r['params']['value']
  result={'configOptions':[{'id':'model','category':'model','name':'Model','type':'select','currentValue':value,'options':[{'value':value,'name':value}]}]}
 elif m=='session/set_model':
  print(json.dumps({'jsonrpc':'2.0','id':r['id'],'error':{'code':-32601,'message':'Method not found'}}),flush=True)
  continue
 else:result={}
 if 'id' in r:print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':result}),flush=True)
"#).unwrap();
    let client = AcpClient::new(
        "harn",
        AcpAgentConfig {
            command: "python3".into(),
            args: vec![
                script.display().to_string(),
                frames.display().to_string(),
                if wrong_readback { "wrong" } else { "correct" }.into(),
            ],
            ..Default::default()
        },
    );
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        let session = client.create_session(directory.path()).await.unwrap();
        client
            .set_session_model(
                session.session_id,
                directory.path(),
                "alibaba-token-plan-cn/deepseek-v4.1-flash-1017",
            )
            .await
    })
    .await
    .unwrap();
    client.close().await.unwrap();
    let frames = std::fs::read_to_string(frames)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    (result, frames)
}

#[tokio::test]
async fn harn_model_selection_uses_confirmed_config_control() {
    let (result, frames) = model_control(false).await;
    result.unwrap();
    assert!(frames.iter().any(|frame| frame["method"] == "session/set_config_option"
        && frame["params"]["configId"] == "model"
        && frame["params"]["value"] == "alibaba-token-plan-cn/deepseek-v4.1-flash-1017"));
    assert!(!frames.iter().any(|frame| frame["method"] == "session/set_model"));
}

#[tokio::test]
async fn harn_model_selection_rejects_wrong_readback_without_fallback() {
    let (result, frames) = model_control(true).await;
    assert!(result.is_err());
    assert!(!frames.iter().any(|frame| frame["method"] == "session/set_model"));
}
