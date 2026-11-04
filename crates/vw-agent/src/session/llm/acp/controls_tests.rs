//! Real ACP JSON-RPC child process; record codec round trips use temporary storage.
use super::*;
use serde_json::json;

#[cfg(unix)]
#[tokio::test]
async fn acp_session_options_follow_capabilities_protocol_and_persistence() {
    let temp = tempfile::tempdir().unwrap();
    let script = temp.path().join("agent.py");
    std::fs::write(&script, r#"
import json, sys
options = [{'id':'effort','name':'思考强度','category':'thought_level','type':'select','currentValue':'low','options':[{'value':'low','name':'低'},{'value':'high','name':'高'},{'value':'reject','name':'拒绝'}]}]
for line in sys.stdin:
    req=json.loads(line)
    if 'id' not in req: continue
    method=req['method']; p=req.get('params',{})
    error=None
    if method=='initialize': result={'protocolVersion':1,'agentCapabilities':{},'authMethods':[]}
    elif method=='session/new': result={'sessionId':'fixture-session','modes':{'currentModeId':'ask','availableModes':[{'id':'ask','name':'询问'},{'id':'code','name':'编码'}]},'models':{'currentModelId':'small','availableModels':[{'modelId':'small','name':'小模型'},{'modelId':'large','name':'大模型'}]},'configOptions':options}
    elif method=='session/set_mode':
        print(json.dumps({'jsonrpc':'2.0','id':900,'method':'fs/write_text_file','params':{'sessionId':'fixture-session','path':sys.argv[1]+'/must-not-exist','content':'denied'}}),flush=True)
        permission=json.loads(sys.stdin.readline())
        if 'error' not in permission: error={'code':-32603,'message':'mode changed local permissions'}
        result={}
    elif method=='session/set_model': result={}
    elif method=='session/set_config_option':
        if p['value']=='reject': error={'code':-32602,'message':'rejected'}
        else: options[0]['currentValue']=p['value']; result={'configOptions':options}
    else: error={'code':-32601,'message':'unexpected request '+method}
    out={'jsonrpc':'2.0','id':req['id']}
    if error: out['error']=error
    else: out['result']=result
    print(json.dumps(out),flush=True)
"#).unwrap();
    let client = vw_acp::AcpClient::new(
        "fixture",
        vw_acp::AcpAgentConfig {
            command: "python3".into(),
            args: vec![
                script.to_string_lossy().into_owned(),
                temp.path().to_string_lossy().into_owned(),
            ],
            env: Default::default(),
        },
    )
    .with_permission_mode(vw_acp::PermissionMode::DenyAll);
    assert!(client.session_capabilities("fixture-session").is_none());
    let info = client.create_session(temp.path()).await.unwrap();
    let options = native_advertised(client.session_capabilities(&info.session_id).unwrap());
    assert_eq!(options.options.len(), 3);
    assert_eq!(options.options[2].choices[1].name, "高");
    let mut record = vw_acp::parse_session_record(&json!({
        "schema": vw_acp::SESSION_RECORD_SCHEMA, "vwacp_record_id":"fixture-session",
        "acp_session_id":"fixture-session", "agent_command":"python3", "cwd":temp.path(),
        "created_at":"2026-11-03T00:00:00Z", "last_used_at":"2026-11-03T00:00:00Z", "last_seq":0, "messages":[], "updated_at":"2026-11-03T00:00:00Z",
    })).unwrap();
    for (kind, value) in [
        (AcpOptionKind::Mode, "code"),
        (AcpOptionKind::Model, "large"),
        (AcpOptionKind::Config { id: "effort".into() }, "high"),
    ] {
        apply_selection(&client, &mut record, AcpSessionSelection { kind, value: value.into() })
            .await
            .unwrap();
    }
    assert!(has_saved_config_category(&record, SessionConfigOptionCategory::ThoughtLevel));
    assert!(!has_saved_config_category(&record, SessionConfigOptionCategory::Model));
    assert!(!temp.path().join("must-not-exist").exists());
    let saved = vw_acp::serialize_session_record_for_disk(&record);
    let file = temp.path().join("session.json");
    std::fs::write(&file, serde_json::to_vec(&saved).unwrap()).unwrap();
    let loaded = vw_acp::parse_session_record(
        &serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(vw_acp::get_desired_mode_id(loaded.vwacp.as_ref()).as_deref(), Some("code"));
    assert_eq!(vw_acp::get_desired_model_id(loaded.vwacp.as_ref()).as_deref(), Some("large"));
    assert_eq!(
        vw_acp::get_desired_config_options(loaded.vwacp.as_ref()).get("effort").map(String::as_str),
        Some("high")
    );
    for value in ["reject", "unadvertised"] {
        assert!(
            apply_selection(
                &client,
                &mut record,
                AcpSessionSelection {
                    kind: AcpOptionKind::Config { id: "effort".into() },
                    value: value.into()
                }
            )
            .await
            .is_err()
        );
        assert_eq!(vw_acp::serialize_session_record_for_disk(&record), saved);
        assert_eq!(
            native_advertised(client.session_capabilities(&info.session_id).unwrap()).options[2]
                .current,
            "high"
        );
    }
    client.close().await.unwrap();
    assert!(client.session_capabilities(&info.session_id).is_none());
}

#[test]
fn acp_session_options_follow_capabilities_config_categories_replace_legacy_selectors() {
    let info = vw_acp::SessionCapabilities {
        session_id: "session".into(),
        modes: serde_json::from_value(json!({"currentModeId":"ask","availableModes":[{"id":"ask","name":"Ask"}]})).ok(),
        models: None,
        config_options: Some(serde_json::from_value(json!([{
            "id":"workflow","name":"工作模式","category":"mode","type":"select","currentValue":"plan",
            "options":[{"group":"work","name":"工作","options":[{"value":"plan","name":"计划"},{"value":"code","name":"编码"}]}]
        }])).unwrap()),
    };
    let options = native_advertised(info);
    assert_eq!(options.options.len(), 1);
    assert_eq!(options.options[0].kind, AcpOptionKind::Config { id: "workflow".into() });
    assert_eq!(options.options[0].choices.len(), 2);
    assert_eq!(options.options[0].current, "plan");
}

#[tokio::test]
async fn acp_session_options_follow_capabilities_isolate_tenant_connections() {
    let temp = tempfile::tempdir().unwrap();
    let config = crate::config::schema::AcpAgentConfig {
        command: "unused-fixture-command".into(),
        args: vec![],
        env: Default::default(),
    };
    let options = super::super::ParsedAcpOptions::default();
    let first = crate::session::ui_store::with_tenant_scope(
        vw_api_types::id::TenantScope::new("platform", "company-a").unwrap(),
        async {
            super::super::session::get_cached_acp_client("fixture", &config, temp.path(), &options)
        },
    )
    .await;
    let second = crate::session::ui_store::with_tenant_scope(
        vw_api_types::id::TenantScope::new("platform", "company-b").unwrap(),
        async {
            super::super::session::get_cached_acp_client("fixture", &config, temp.path(), &options)
        },
    )
    .await;
    assert!(!Arc::ptr_eq(&first.client, &second.client));
    assert_ne!(first.client.tenant, second.client.tenant);
    let mut cache = ACP_CLIENT_CACHE.lock();
    cache.remove(&first.key);
    cache.remove(&second.key);
}

#[test]
fn acp_model_source_session_controls_managed_models_are_not_native_choices() {
    let info = vw_acp::SessionCapabilities {
        session_id: "managed".into(), modes: None,
        models: serde_json::from_value(json!({"currentModelId":"native","availableModels":[{"modelId":"native","name":"Native"}]})).ok(),
        config_options: None,
    };
    let options = advertised(info);
    assert!(
        !options.options.iter().any(|option| option.kind == AcpOptionKind::Model),
        "managed Provider selection must not be replaced by a native CLI model"
    );
}

#[test]
fn acp_model_source_session_controls_model_config_is_hidden_but_mode_survives() {
    let info = vw_acp::SessionCapabilities {
        session_id: "managed".into(), models: None, modes: None,
        config_options: Some(serde_json::from_value(json!([
            {"id":"llm","name":"Model","category":"model","type":"select","currentValue":"native","options":[{"value":"native","name":"Native"}]},
            {"id":"mode","name":"Mode","category":"mode","type":"select","currentValue":"ask","options":[{"value":"ask","name":"Ask"}]}
        ])).unwrap()),
    };
    assert_eq!(native_advertised(info.clone()).options.len(), 2);
    let managed = advertised(info);
    assert_eq!(managed.options.len(), 1);
    assert_eq!(managed.options[0].kind, AcpOptionKind::Config { id: "mode".into() });
}

#[tokio::test]
async fn acp_model_source_session_controls_busy_guard_is_session_scoped() {
    let session = format!("controls-{}", uuid::Uuid::new_v4());
    let guard = super::super::lifecycle::try_lock(&session, "/workspace").unwrap();
    assert!(
        super::super::lifecycle::try_lock(&session, "/workspace").unwrap_err().contains("busy")
    );
    assert!(super::super::lifecycle::try_lock(&session, "/other").is_ok());
    let other = crate::session::ui_store::with_tenant_scope(
        vw_api_types::id::TenantScope::new("platform", "other-company").unwrap(),
        async { super::super::lifecycle::try_lock(&session, "/workspace") },
    )
    .await;
    assert!(other.is_ok());
    drop(guard);
    assert!(super::super::lifecycle::try_lock(&session, "/workspace").is_ok());
}

#[test]
fn acp_model_source_session_controls_disconnected_keeps_provider_identity() {
    use vw_api_types::acp_model_source::{AcpModelSource, AcpSessionModelSelection};
    let selection = super::super::lifecycle::Selection {
        agent: "unknown-custom-driver".into(),
        model: AcpSessionModelSelection {
            source: AcpModelSource::SessionModel,
            provider_id: Some("provider".into()),
            model: Some("nested/model".into()),
        },
        generation: 2,
        record_id: "recoverable-record".into(),
    };
    let options = project_options(None, Some(&selection));
    assert!(!options.connected);
    assert_eq!(options.selected_model, Some(selection.model));
    assert!(matches!(
        options.model_source_capabilities[0],
        vw_api_types::acp_model_source::AcpModelSourceCapability::Unsupported { .. }
    ));
}
