use super::lifecycle_api_tests::{
    HOST, MANAGER, MEMBER, READER, assert_error, client, spawn_gateway, write_attached_app,
    write_policy,
};
use reqwest::{Client, Method, Response};
use serde_json::{Value, json};
use std::{
    fs,
    net::{Ipv4Addr, TcpListener},
    time::Duration,
};
use vw_api_types::{
    agent_apps::AppId,
    app_routing::{AppRoutingConfig, RoutingState},
};

async fn request(
    port: u16,
    path: &str,
    method: Method,
    token: Option<&str>,
    body: Option<Value>,
) -> Response {
    let mut request = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap()
        .request(method, format!("http://127.0.0.1:{port}/v1/app-routing/hosts/{HOST}/{path}"))
        .header("x-platform-id", "p")
        .header("x-company-id", "owner");
    if let Some(token) = token {
        request = request.header("x-agent-apps-token", token);
    }
    if let Some(body) = body {
        request = request.json(&body);
    }
    request.send().await.unwrap()
}

#[tokio::test]
async fn cc_route_012_config_api_expected_behavior() {
    let home = tempfile::tempdir().unwrap();
    let policy = home.path().join("access.json");
    write_policy(&policy, home.path(), 0);
    let (gateway, port) = spawn_gateway(home.path(), &policy).await;
    let listener = json!({
        "enabled": false, "listenAddress": "127.0.0.1", "listenPort": 0, "loggingEnabled": false
    });
    let unauthorized = request(port, "config", Method::PUT, None, Some(listener.clone())).await;
    assert_eq!(unauthorized.status(), 401, "config writes require an authenticated member");

    let mut value: Value = serde_json::from_slice(&fs::read(&policy).unwrap()).unwrap();
    for i in 0..2 {
        value["principals"][i]["grants"][0]["routing_apps"] = json!([
            {"project_id": "project", "app_id": "codex"},
            {"project_id": "other", "app_id": "codex"},
            {"project_id": "project", "app_id": "claude"},
            {"project_id": "project", "app_id": "claude-desktop"}
        ]);
    }
    fs::write(&policy, value.to_string()).unwrap();

    let owner = client(port, "owner");
    let other_tenant = client(port, "other");
    let initial = owner.app_routing_config(HOST, READER).await.unwrap();
    assert_eq!(serde_json::to_value(initial).unwrap(), listener);
    assert_error(owner.app_routing_config(HOST, MEMBER).await.unwrap_err(), 403, "forbidden");
    assert_error(
        owner.app_routing_config("other-mac", READER).await.unwrap_err(),
        403,
        "forbidden",
    );
    assert_error(
        other_tenant.app_routing_config(HOST, READER).await.unwrap_err(),
        403,
        "forbidden",
    );
    assert_error(
        owner.app_routing_set_config(HOST, READER, &initial).await.unwrap_err(),
        403,
        "forbidden",
    );

    let running = owner.app_routing_start(HOST, MANAGER).await.unwrap();
    let original_address = (running.bound_address.unwrap(), running.bound_port.unwrap());
    for (field, invalid) in [
        ("unknown", json!(1)),
        ("listenPort", json!(-1)),
        ("listenPort", json!(65536)),
        ("listenPort", json!(1023)),
        ("listenAddress", json!("0.0.0.0")),
    ] {
        let mut body = listener.clone();
        body[field] = invalid;
        let response = request(port, "config", Method::PUT, Some(MANAGER), Some(body)).await;
        assert_eq!(response.status(), 400, "{field}");
        assert_eq!(response.json::<Value>().await.unwrap()["error"]["code"], "invalid_input");
    }
    let array = json!([false, "127.0.0.1", 0, true]);
    let response = request(port, "config", Method::PUT, Some(MANAGER), Some(array)).await;
    assert_eq!(response.status(), 400, "configuration bodies must be objects");
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(owner.app_routing_config(HOST, READER).await.unwrap(), initial);
    assert_eq!(owner.app_routing_status(HOST, READER).await.unwrap(), running);
    let occupied = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let next_port = occupied.local_addr().unwrap().port();
    let changed = vw_api_types::app_routing::RoutingConfig {
        listen_port: next_port,
        logging_enabled: true,
        ..initial
    };
    assert_error(
        owner.app_routing_set_config(HOST, MANAGER, &changed).await.unwrap_err(),
        409,
        "config_not_applied",
    );
    assert_eq!(owner.app_routing_config(HOST, READER).await.unwrap(), initial);
    assert_eq!(owner.app_routing_status(HOST, READER).await.unwrap(), running);

    let project = "project".into();
    let other = "other".into();
    let defaults =
        owner.app_routing_app_config(HOST, &project, AppId::Codex, READER).await.unwrap();
    assert_eq!(
        defaults,
        AppRoutingConfig {
            first_byte_timeout_secs: 60,
            idle_timeout_secs: 120,
            non_streaming_timeout_secs: 600,
        }
    );
    let changed_timeouts = AppRoutingConfig {
        first_byte_timeout_secs: 120,
        idle_timeout_secs: 0,
        non_streaming_timeout_secs: 1200,
    };
    assert_error(
        owner
            .app_routing_app_config(HOST, &"foreign".into(), AppId::Codex, MANAGER)
            .await
            .unwrap_err(),
        403,
        "forbidden",
    );
    assert_error(
        owner.app_routing_app_config(HOST, &other, AppId::Claude, MANAGER).await.unwrap_err(),
        403,
        "forbidden",
    );
    assert_error(
        owner
            .app_routing_app_config(HOST, &project, AppId::ClaudeDesktop, MANAGER)
            .await
            .unwrap_err(),
        422,
        "unsupported_app",
    );
    assert_error(
        owner
            .app_routing_set_app_config(HOST, &project, AppId::Codex, READER, &changed_timeouts)
            .await
            .unwrap_err(),
        403,
        "forbidden",
    );
    assert_eq!(
        owner
            .app_routing_set_app_config(HOST, &project, AppId::Codex, MANAGER, &changed_timeouts)
            .await
            .unwrap(),
        changed_timeouts
    );

    let app_config = "projects/project/apps/codex/config";
    for (field, invalid) in [
        ("enabled", json!(true)),
        ("firstByteTimeoutSecs", json!(-1)),
        ("firstByteTimeoutSecs", json!(0)),
        ("firstByteTimeoutSecs", json!(121)),
        ("idleTimeoutSecs", json!(-1)),
        ("idleTimeoutSecs", json!(601)),
        ("nonStreamingTimeoutSecs", json!(59)),
        ("nonStreamingTimeoutSecs", json!(1201)),
    ] {
        let mut body = serde_json::to_value(defaults).unwrap();
        body[field] = invalid;
        let response = request(port, app_config, Method::PUT, Some(MANAGER), Some(body)).await;
        assert_eq!(response.status(), 400, "{field}");
        assert_eq!(response.json::<Value>().await.unwrap()["error"]["code"], "invalid_input");
    }
    assert_eq!(
        owner.app_routing_app_config(HOST, &project, AppId::Codex, READER).await.unwrap(),
        changed_timeouts
    );
    assert_eq!(
        owner.app_routing_app_config(HOST, &other, AppId::Codex, READER).await.unwrap(),
        defaults
    );
    assert_eq!(
        owner.app_routing_app_config(HOST, &project, AppId::Claude, READER).await.unwrap(),
        defaults
    );
    assert_eq!(owner.app_routing_status(HOST, READER).await.unwrap(), running);

    drop(occupied);
    let (state_path, original_state) = write_attached_app(home.path());
    assert_error(
        owner.app_routing_set_config(HOST, MANAGER, &changed).await.unwrap_err(),
        409,
        "config_not_applied",
    );
    assert_eq!(fs::read_to_string(&state_path).unwrap(), original_state);
    assert_eq!(owner.app_routing_config(HOST, READER).await.unwrap(), initial);
    assert_eq!(owner.app_routing_status(HOST, READER).await.unwrap(), running);
    fs::remove_file(state_path).unwrap();
    assert_eq!(owner.app_routing_set_config(HOST, MANAGER, &changed).await.unwrap(), changed);
    assert_eq!(owner.app_routing_status(HOST, READER).await.unwrap().bound_port, Some(next_port));
    let listener_response = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap()
        .get(format!("http://127.0.0.1:{next_port}/v1/models"))
        .send()
        .await
        .unwrap();
    assert_eq!(listener_response.status(), 401, "the new address keeps data-plane authentication");
    assert!(tokio::net::TcpStream::connect(original_address).await.is_err());
    drop(gateway);

    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
    let owner = client(port, "owner");
    assert_eq!(owner.app_routing_status(HOST, READER).await.unwrap().state, RoutingState::Stopped);
    assert_eq!(owner.app_routing_config(HOST, READER).await.unwrap(), changed);
    assert_eq!(
        owner.app_routing_app_config(HOST, &project, AppId::Codex, READER).await.unwrap(),
        changed_timeouts
    );
}
