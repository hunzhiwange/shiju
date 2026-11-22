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

    let request_id = uuid::Uuid::new_v4().to_string();
    let chat = json!({
        "session_id": session_id,
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
    let owner = TenantScope::new("employee-acp-fixture", "42")
        .unwrap()
        .with_user_id(UserId::new("101").unwrap());
    assert_employee_binding(&owner, &project.id, &directory, session_id).await;
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

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn employee_acp_gateway_verifies_account_and_waits_for_product_stop() {
    use super::employee_account::{LocalEmployeeGateway, TEST_CLIENT};
    use axum::{Extension, extract::ConnectInfo, routing::get};
    use gateway_shiju_account::SHIJU_CREDENTIAL_HEADER;
    use std::{fs, net::SocketAddr, os::unix::fs::PermissionsExt, time::Duration};

    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().canonicalize().unwrap();
    let executable = directory.join("employee-peer");
    fs::write(&executable, r#"#!/bin/sh
exec python3 -c '
import json, os, pathlib, sys
os.chdir(pathlib.Path(sys.argv.pop(1)).parent)
assert sys.argv[1:6] == ["employee-acp", "--employee-id", "worker-1", "--employee-kind", "private"]
assert os.environ.get("SHIJU_EMPLOYEE_ACP_CREDENTIAL") in ("fixture-login-a", "fixture-login-b")
pending = None
for line in sys.stdin:
    request = json.loads(line)
    method = request.get("method")
    if method == "session/cancel":
        pathlib.Path("cancelled").touch()
        if pending is not None:
            if pathlib.Path("late-event").exists():
                print(json.dumps({"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"employee-gateway-fixture","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"late reply"}}}}), flush=True)
            print(json.dumps({"jsonrpc":"2.0","id":pending,"result":{"stopReason":"cancelled"}}), flush=True)
        continue
    if "id" not in request:
        continue
    result = {}
    if method == "initialize":
        result = {"protocolVersion":1,"agentCapabilities":{"loadSession":True},"authMethods":[]}
    elif method == "session/new":
        result = {"sessionId":"employee-gateway-fixture"}
    elif method == "session/prompt":
        assert request["params"]["prompt"] == [{"type":"text","text":"wait for stop"}]
        pending = request["id"]
        pathlib.Path("prompted").touch()
        continue
    print(json.dumps({"jsonrpc":"2.0","id":request["id"],"result":result}), flush=True)
' "$0" "$@"
"#).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();

    let tenant = TenantScope::new("employee-acp-live-fixture", "42").unwrap();
    let account = tenant.clone().with_user_id(UserId::new("103").unwrap());
    let skey = "employee-live-fixture-local";
    crate::config::replace_gateway_skeys_for_tenant(
        &tenant,
        &[GatewaySkey {
            enabled: true,
            skey: None,
            skey_hash: hash_skey(skey),
            masked_skey: String::new(),
            name: String::new(),
            expires_at: None,
            user_id: Some("103".into()),
        }],
    )
    .await
    .unwrap();
    let mut profile = agent_config::catalog::merged_agent_catalog(&Default::default())
        .remove("acp-employee-acp")
        .unwrap();
    profile.enabled = true;
    crate::config::update_global_for_tenant(
        &tenant,
        json!({"agents":{"acp-employee-acp":profile}}),
    )
    .await
    .unwrap();

    use crate::session::llm::employee_context::TEST_EXECUTABLE;
    struct ResetFixture;
    impl Drop for ResetFixture {
        fn drop(&mut self) {
            TEST_EXECUTABLE.lock().take();
        }
    }
    let _reset = ResetFixture;
    *TEST_EXECUTABLE.lock() = Some((account.clone(), executable));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let upstream = axum::Router::new().route("/iam/api/v1/me", get(|headers: axum::http::HeaderMap| async move {
        let user = match headers.get("x-api-key").and_then(|key| key.to_str().ok()) {
            Some("fixture-login-a" | "fixture-login-b") => "103",
            Some("fixture-foreign") => "999",
            _ => return (StatusCode::UNAUTHORIZED, axum::Json(json!({}))),
        };
        (StatusCode::OK, axum::Json(json!({"code":200,"message":"","data":{
            "user_id":user,"company_id":"42","account_id":user,"account_name":"fixture","name":"fixture",
            "source_type":"manual","external_user_id":"","allow_console_login":true,"status":"enabled",
        }})))
    }));
    let server = tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });
    let client = shiju_client::account::ShijuAccountClient::with_base_urls(&base, &base).unwrap();

    let app = super::tests::test_app_for_tenant(tenant)
        .layer(Extension(LocalEmployeeGateway))
        .layer(Extension(ConnectInfo("127.0.0.1:12345".parse::<SocketAddr>().unwrap())));
    let request_id = uuid::Uuid::new_v4().to_string();
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("directory", directory.to_str().unwrap())
        .finish();
    let request = |path: &str, key: Option<&str>, body: Value| {
        let mut request = Request::post(path)
            .header("authorization", format!("Bearer {skey}"))
            .header(USER_ID_HEADER, "103")
            .header("content-type", "application/json");
        if let Some(key) = key {
            request = request.header(SHIJU_CREDENTIAL_HEADER, key);
        }
        request.body(Body::from(body.to_string())).unwrap()
    };
    let chat_path = format!("/chat/stream?{query}");
    let chat = json!({
        "agent":"acp-employee-acp",
        "messages":[{"role":"user","content":"wait for stop"}],
        "options":{"employee_target":{"employee_id":"worker-1","kind":"private"}},
    });
    TEST_CLIENT
        .scope(client, async {
            for key in [None, Some("fixture-foreign")] {
                let response =
                    app.clone().oneshot(request(&chat_path, key, chat.clone())).await.unwrap();
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            }

            let mut proxied = request(&chat_path, Some("fixture-login-a"), chat.clone());
            proxied.headers_mut().insert("x-forwarded-for", "203.0.113.7".parse().unwrap());
            let response = app.clone().oneshot(proxied).await.unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);

            let mut start = request(&chat_path, Some("fixture-login-a"), chat.clone());
            start.headers_mut().insert(STREAM_REQUEST_ID_HEADER, request_id.parse().unwrap());
            let response = app.clone().oneshot(start).await.unwrap();
            let status = response.status();
            if status != StatusCode::OK {
                panic!(
                    "employee chat admission returned {status}: {}",
                    response_json(response).await
                );
            }
            let session = response.headers()["x-vw-session-id"].to_str().unwrap().to_owned();
            tokio::time::timeout(Duration::from_secs(20), async {
                while !directory.join("prompted").exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("credential reached the controlled peer and it received the prompt");

            let stop_path = format!("/chat/stream/{request_id}/stop");
            let stopped =
                app.clone().oneshot(request(&stop_path, None, Value::Null)).await.unwrap();
            assert_eq!(response_json(stopped).await, json!({"cancelled":true}));
            let body = tokio::time::timeout(
                Duration::from_secs(10),
                to_bytes(response.into_body(), 32_768),
            )
            .await
            .unwrap()
            .unwrap();
            let sse = String::from_utf8_lossy(&body);
            assert!(
                sse.contains(r#""type":"chat.done""#)
                    && sse.contains(r#""finish_reason":"cancelled""#),
                "{sse}"
            );
            assert!(directory.join("cancelled").exists());
            assert!(!sse.contains("fixture-login-a"));
            let (project, _) = crate::project::from_directory(&account, &directory).await.unwrap();
            let binding = session_engine::binding_store::load_scoped(
                crate::global::paths().home.clone(),
                account.clone(),
                &project.id,
                &session,
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(binding.employee_target().unwrap().employee_id(), "worker-1");
            assert!(!serde_json::to_string(&binding).unwrap().contains("fixture-login-a"));
            let record =
                acp_storage::resolve_session_record("employee-gateway-fixture").await.unwrap();
            assert!(record.agent_config.is_none());
            assert!(!serde_json::to_string(&record).unwrap().contains("fixture-login-a"));

            let mut continued = chat.clone();
            continued["session_id"] = json!(session);
            continued.as_object_mut().unwrap().remove("options");
            continued.as_object_mut().unwrap().remove("agent");
            let mut changed = continued.clone();
            changed["options"] =
                json!({"employee_target":{"employee_id":"worker-2","kind":"private"}});
            let denied = app
                .clone()
                .oneshot(request(&chat_path, Some("fixture-login-a"), changed.clone()))
                .await
                .unwrap();
            assert_eq!(denied.status(), StatusCode::BAD_REQUEST);

            fs::remove_file(directory.join("prompted")).unwrap();
            fs::write(directory.join("late-event"), []).unwrap();
            let stale = app
                .clone()
                .oneshot(request(&chat_path, Some("fixture-login-a"), continued.clone()))
                .await
                .unwrap();
            assert_eq!(stale.status(), StatusCode::OK);
            tokio::time::timeout(Duration::from_secs(20), async {
                while !directory.join("prompted").exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("existing employee session reopened");

            // The refreshed account can authorize the same session, but the old stream loses
            // delivery immediately. The requested employee change still fails.
            let rotated = app
                .clone()
                .oneshot(request(&chat_path, Some("fixture-login-b"), changed))
                .await
                .unwrap();
            assert_eq!(rotated.status(), StatusCode::BAD_REQUEST);
            let body =
                tokio::time::timeout(Duration::from_secs(10), to_bytes(stale.into_body(), 32_768))
                    .await
                    .unwrap()
                    .unwrap();
            assert!(!String::from_utf8_lossy(&body).contains("late reply"));

            let old = app
                .clone()
                .oneshot(request(&chat_path, Some("fixture-login-a"), continued.clone()))
                .await
                .unwrap();
            assert_eq!(old.status(), StatusCode::UNAUTHORIZED);
            fs::remove_file(directory.join("prompted")).unwrap();
            let fresh = app
                .clone()
                .oneshot(request(&chat_path, Some("fixture-login-b"), continued.clone()))
                .await
                .unwrap();
            assert_eq!(fresh.status(), StatusCode::OK);
            assert_eq!(fresh.headers()["x-vw-session-id"], session.as_str());
            tokio::time::timeout(Duration::from_secs(20), async {
                while !directory.join("prompted").exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("refreshed credential resumed the original employee session");

            let revoke = app
                .clone()
                .oneshot(request(
                    "/chat/employee-account/revoke",
                    Some("fixture-login-b"),
                    Value::Null,
                ))
                .await
                .unwrap();
            assert_eq!(revoke.status(), StatusCode::NO_CONTENT);
            let body =
                tokio::time::timeout(Duration::from_secs(10), to_bytes(fresh.into_body(), 32_768))
                    .await
                    .unwrap()
                    .unwrap();
            assert!(!String::from_utf8_lossy(&body).contains("late reply"));

            let denied =
                app.oneshot(request(&chat_path, Some("fixture-login-b"), continued)).await.unwrap();
            assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
        })
        .await;
    server.abort();
}
