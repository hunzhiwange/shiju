use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use config_gateway::GatewaySkey;
use gateway_pairing::pairing::hash_skey;
use gateway_resumable_stream::{STREAM_REQUEST_ID_HEADER, global_stream_store, request_signature};
use serde_json::{Value, json};
use session_engine::agent_binding::EmployeeTarget;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use vw_api_chat::chat::GatewayChatStreamRequest;
use vw_api_transport::{TenantScope, USER_ID_HEADER, UserId};

const EMPLOYEE_ID: &str = "11111111-2222-4333-8444-555555555555";

#[test]
fn employee_acp_gateway_target_has_a_closed_schema() {
    let valid = json!({"employee_id": EMPLOYEE_ID, "kind": "private"});
    let target: EmployeeTarget = serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(target.employee_id(), EMPLOYEE_ID);
    assert_eq!(serde_json::to_value(target).unwrap(), valid);
    assert!(
        serde_json::from_value::<EmployeeTarget>(
            json!({"employee_id": "employee_1", "kind": "shared"})
        )
        .is_ok()
    );

    for invalid in [
        json!({"employee_id": EMPLOYEE_ID, "kind": "team"}),
        json!({"employee_id": EMPLOYEE_ID, "kind": "private", "user_id": "42"}),
        json!({"employee_id": EMPLOYEE_ID, "kind": "private", "credential": "synthetic"}),
        json!({"employee_id": "", "kind": "shared"}),
        json!({"employee_id": "x".repeat(129), "kind": "shared"}),
        json!({"employee_id": "../assistant", "kind": "shared"}),
        json!({"employee_id": EMPLOYEE_ID}),
        Value::Null,
    ] {
        assert!(serde_json::from_value::<EmployeeTarget>(invalid).is_err());
    }
}

#[tokio::test]
async fn employee_acp_gateway_sessions_replay_and_stop_are_user_scoped() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let tenant = TenantScope::new("employee-acp-fixture", "42").unwrap();
    let owner = tenant.clone().with_user_id(UserId::new("101").unwrap());
    let owner_key = "employee-acp-test-owner";
    let other_key = "employee-acp-test-other";
    let skeys = [(owner_key, "101"), (other_key, "102")].map(|(key, user)| GatewaySkey {
        enabled: true,
        skey: None,
        skey_hash: hash_skey(key),
        masked_skey: String::new(),
        name: String::new(),
        expires_at: None,
        user_id: Some(user.into()),
    });
    crate::config::replace_gateway_skeys_for_tenant(&tenant, &skeys).await.unwrap();

    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().canonicalize().unwrap().to_string_lossy().into_owned();
    let (project, _) = crate::project::from_directory(&owner, &directory).await.unwrap();
    let app = super::tests::test_app_for_tenant(tenant);
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("directory", &directory)
        .finish();
    let session_request = |method, path: &str, key: &str, user: &str, body: Body| {
        Request::builder()
            .method(method)
            .uri(format!("{path}?{query}"))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .header(USER_ID_HEADER, user)
            .body(body)
            .unwrap()
    };
    let response = app
        .clone()
        .oneshot(session_request(
            Method::POST,
            "/session",
            owner_key,
            "101",
            Body::from(r#"{"title":"Owner session"}"#),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let session = response_json(response).await;
    let session_id = session["id"].as_str().unwrap();
    let path = format!("/session/{session_id}");
    for (method, body) in
        [(Method::GET, Body::empty()), (Method::PATCH, Body::from(r#"{"title":"Other user"}"#))]
    {
        let response = app
            .clone()
            .oneshot(session_request(method, &path, other_key, "102", body))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    let response = app
        .clone()
        .oneshot(session_request(Method::GET, &path, owner_key, "101", Body::empty()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["title"], "Owner session");

    assert_employee_binding(&owner, &project.id, &directory, session_id).await;

    let request_id = uuid::Uuid::new_v4().to_string();
    let chat = json!({
        "session_id": session_id,
        "agent": "acp-employee-acp",
        "messages": [{"role": "user", "content": "fixture"}],
    });
    let body: GatewayChatStreamRequest = serde_json::from_value(chat.clone()).unwrap();
    let signature =
        request_signature(&serde_json::to_vec(&(&project.id, &directory, body)).unwrap());
    let mut stream = global_stream_store()
        .open_chat(owner, request_id.clone(), signature, None, Some(session_id.into()))
        .unwrap();
    let producer = stream.producer.take().unwrap();
    let cancelled = CancellationToken::new();
    let stop_token = cancelled.clone();
    producer.install_cancellation(move || stop_token.cancel());

    let stop = |key: &str, user: &str| {
        Request::post(format!("/chat/stream/{request_id}/stop"))
            .header("authorization", format!("Bearer {key}"))
            .header(USER_ID_HEADER, user)
            .body(Body::empty())
            .unwrap()
    };
    let spoofed = app.clone().oneshot(stop(other_key, "101")).await.unwrap();
    assert_eq!(spoofed.status(), StatusCode::UNAUTHORIZED);
    let other_stop = app.clone().oneshot(stop(other_key, "102")).await.unwrap();
    assert_eq!(other_stop.status(), StatusCode::OK);
    assert_eq!(response_json(other_stop).await, json!({"cancelled": false}));
    assert!(!cancelled.is_cancelled());

    let owner_stop = app.clone().oneshot(stop(owner_key, "101")).await.unwrap();
    assert_eq!(owner_stop.status(), StatusCode::OK);
    assert_eq!(response_json(owner_stop).await, json!({"cancelled": true}));
    assert!(cancelled.is_cancelled());
    assert!(producer.publish(r#"{"type":"done"}"#, true));

    let replay = |key: &str, user: &str| {
        Request::post(format!("/chat/stream?{query}"))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .header(USER_ID_HEADER, user)
            .header(STREAM_REQUEST_ID_HEADER, &request_id)
            .header("last-event-id", "0")
            .body(Body::from(chat.to_string()))
            .unwrap()
    };
    let other_replay = app.clone().oneshot(replay(other_key, "102")).await.unwrap();
    assert_eq!(other_replay.status(), StatusCode::GONE);
    let owner_replay = app.oneshot(replay(owner_key, "101")).await.unwrap();
    assert_eq!(owner_replay.status(), StatusCode::OK);
    let body = to_bytes(owner_replay.into_body(), 8192).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains(r#"{"type":"done"}"#));
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), 8192).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

async fn assert_employee_binding(
    tenant: &TenantScope,
    project: &str,
    directory: &str,
    session: &str,
) {
    use crate::session::agent_binding::AdmissionHost;
    use session_engine::{
        admission::{EmployeeAdmission, prepare_with_employee},
        agent_binding::AgentBinding,
    };

    let definition = config_agent::AgentDefinitionConfig {
        runtime: config_agent_profile::AgentRuntime::Acp { backend: "employee-acp".into() },
        ..Default::default()
    };
    let legacy = AgentBinding::from_definition("acp-employee-acp".into(), &definition);
    let mut encoded = serde_json::to_value(legacy).unwrap();
    assert!(encoded.get("employee_target").is_none());
    let legacy: AgentBinding = serde_json::from_value(encoded.clone()).unwrap();
    assert!(legacy.employee_target().is_none());

    encoded["employee_target"] = json!({"employee_id": EMPLOYEE_ID, "kind": "private"});
    let binding: AgentBinding = serde_json::from_value(encoded).unwrap();
    let home = crate::global::paths().home.clone();
    session_engine::binding_store::save_scoped(home, tenant.clone(), project, session, &binding)
        .await
        .unwrap();

    let session = session.to_owned();
    let workspace = std::path::PathBuf::from(directory);
    super::shared::with_tenant_instance(tenant.clone(), directory.to_owned(), move || {
        Box::pin(async move {
            let changed: EmployeeTarget = serde_json::from_value(json!({
                "employee_id": EMPLOYEE_ID, "kind": "shared"
            }))
            .unwrap();
            let denied = prepare_with_employee::<AdmissionHost>(
                &session,
                None,
                None,
                None,
                &workspace,
                EmployeeAdmission::Verified { target: Some(&changed) },
            )
            .await
            .err()
            .expect("a different target must be rejected");
            assert_eq!(denied.to_string(), "changing the employee target requires a new session");
            let (restored, _) = prepare_with_employee::<AdmissionHost>(
                &session,
                None,
                None,
                None,
                &workspace,
                EmployeeAdmission::Verified { target: None },
            )
            .await
            .unwrap();
            assert_eq!(restored.employee_target(), binding.employee_target());
            Ok(())
        })
    })
    .await
    .unwrap();
}

#[test]
fn employee_acp_gateway_recovery_anchor_isolates_users_in_one_tenant() {
    use agent_database::GlobalDatabase;
    use app_settings_store::settings_store::GatewaySettingsStore;
    use llm_runtime::acp::lifecycle::{Selection, Store, try_lock};
    use vw_api_acp_model::acp_model_source::{AcpModelSource, AcpSessionModelSelection};

    let root = tempfile::tempdir().unwrap();
    let database = GlobalDatabase::for_home(root.path()).unwrap();
    let store = GatewaySettingsStore::new(database).unwrap();
    let tenant = TenantScope::default();
    let owner = tenant.clone().with_user_id(UserId::new("101").unwrap());
    let other = tenant.clone().with_user_id(UserId::new("102").unwrap());
    let session = "session-employee-shared-id";
    let workspace = "/workspace";
    let owner_store = Store::open(store.clone(), owner.clone(), session, workspace);
    let other_store = Store::open(store.clone(), other.clone(), session, workspace);
    let legacy_store = Store::open(store, tenant, session, workspace);
    let selection = Selection {
        agent: "employee-acp".into(),
        model: AcpSessionModelSelection {
            source: AcpModelSource::CliLocal,
            provider_id: None,
            model: None,
        },
        generation: 0,
        record_id: "synthetic-acp-session".into(),
    };
    owner_store.commit(&selection).unwrap();
    assert_eq!(owner_store.read().unwrap().unwrap().record_id, selection.record_id);
    assert!(other_store.read().unwrap().is_none());
    assert!(legacy_store.read().unwrap().is_none());

    let _owner_guard = try_lock(&owner, session, workspace).unwrap();
    assert!(try_lock(&owner, session, workspace).is_err());
    assert!(try_lock(&other, session, workspace).is_ok());
}
