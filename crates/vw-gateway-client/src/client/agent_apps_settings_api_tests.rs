use super::{AgentAppsSettingsError, test_support};
use serde_json::json;
use vw_api_types::agent_apps::*;

fn launch_request() -> LaunchRequest {
    serde_json::from_value(json!({"appId":"codex","installationId":"remote-instance","cwd":"Z:\\remote\\不存在","args":["literal;$(touch marker)","https://example.invalid"],"skipPermissions":false})).unwrap()
}
fn preferences() -> serde_json::Value {
    json!({"user":{"appOrder":["codex"],"visibleApps":["codex"]},"host":{"preferredTerminal":"remote-terminal","wslShells":[]},"restartRequired":true})
}
#[tokio::test]
async fn cc_apps_settings_launch_sdk_roundtrip_paths_stay_remote() {
    let server = test_support::server(vec![
        (200, preferences()),
        (200, preferences()),
        (200, json!({"installationId":"remote-instance"})),
    ]);
    let client = super::GatewayClient::new(
        server
            .client()
            .endpoint()
            .clone()
            .with_auth(crate::GatewayAuth { skey: Some("settings-token".into()) }),
    )
    .unwrap();
    let result = client.agent_apps_preferences("host /?中文").await.unwrap();
    assert!(result.restart_required);
    let patch = AppPreferencesPatch::Host(result.host.clone());
    assert_eq!(client.agent_apps_preferences_update("host /?中文", &patch).await.unwrap(), result);
    let launch = launch_request();
    assert_eq!(
        client.agent_apps_launch("host /?中文", &launch, None, None).await.unwrap().installation_id,
        launch.installation_id
    );
    for (method, resource, body) in [
        ("GET", "preferences", serde_json::Value::Null),
        ("PATCH", "preferences", serde_json::to_value(patch).unwrap()),
        ("POST", "launch", serde_json::to_value(launch).unwrap()),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(
            request.path,
            format!("/v1/agent-apps/{resource}?targetHostId=host+%2F%3F%E4%B8%AD%E6%96%87")
        );
        assert_eq!(request.body, body);
        assert_eq!(request.header("authorization"), Some("Bearer settings-token"));
        assert!(request.header("x-platform-id").is_some());
        assert!(request.header("x-company-id").is_some());
    }
    server.join();
}
#[tokio::test]
async fn cc_apps_settings_launch_sdk_roundtrip_errors_preserve_selection() {
    for status in [403, 409, 422, 501, 503, 307] {
        let body = json!({"code":"gateway_rejection","details":{"supported":false}});
        let server = test_support::server(vec![(status, body.clone())]);
        assert_eq!(
            server
                .client()
                .agent_apps_launch(
                    "remote",
                    &launch_request(),
                    Some("provider /?"),
                    Some("account-id")
                )
                .await
                .unwrap_err(),
            AgentAppsSettingsError::Http { status, body }
        );
        let request = server.take_request();
        assert_eq!(request.body["providerId"], "provider /?");
        assert_eq!(request.body["accountId"], "account-id");
        assert!(request.body.get("env").is_none());
        assert!(request.body.get("token").is_none());
        server.join();
    }
}
#[tokio::test]
async fn cc_apps_settings_launch_sdk_roundtrip_decode_and_empty_errors() {
    let server = test_support::server(vec![(200, json!({"unexpected":true}))]);
    assert!(matches!(
        server.client().agent_apps_preferences("h").await,
        Err(AgentAppsSettingsError::Decode(_))
    ));
    server.join();
    let server = test_support::server_raw(vec![(501, String::new())]);
    assert_eq!(
        server.client().agent_apps_launch("h", &launch_request(), None, None).await.unwrap_err(),
        AgentAppsSettingsError::Http { status: 501, body: json!("") }
    );
    server.join();
}

#[tokio::test]
async fn cc_apps_settings_launch_sdk_roundtrip_dropped_reply_and_redirect_never_repeat_write() {
    use std::io::{BufRead, Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};
    for redirect in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut count = 0;
            let mut first_seen = None;
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
                        let mut reader = std::io::BufReader::new(&mut stream);
                        let mut length = 0;
                        loop {
                            let mut line = String::new();
                            reader.read_line(&mut line).unwrap();
                            if line == "\r\n" {
                                break;
                            }
                            if let Some(value) =
                                line.to_ascii_lowercase().strip_prefix("content-length:")
                            {
                                length = value.trim().parse::<usize>().unwrap();
                            }
                        }
                        let mut body = vec![0; length];
                        reader.read_exact(&mut body).unwrap();
                        assert_eq!(
                            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["installationId"],
                            "remote-instance"
                        );
                        count += 1;
                        first_seen.get_or_insert(Instant::now());
                        if redirect {
                            write!(stream,"HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:{port}/replayed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                        }
                        // Closing without a reply models an accepted write whose acknowledgement was lost.
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if first_seen
                            .is_some_and(|at: Instant| at.elapsed() > Duration::from_millis(300))
                        {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            }
            count
        });
        let client =
            super::GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
        let result = client.agent_apps_launch("h", &launch_request(), None, None).await;
        if redirect {
            assert_eq!(
                result.unwrap_err(),
                AgentAppsSettingsError::Http { status: 307, body: json!("") }
            );
        } else {
            assert!(matches!(result, Err(AgentAppsSettingsError::Transport(_))));
        }
        assert_eq!(handle.join().unwrap(), 1, "write must reach the server exactly once");
    }
}
