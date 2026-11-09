use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    time::Duration,
};

use iced::widget::text_editor::{Action, Edit};
use serde_json::json;
use vw_gateway_client::{
    AppsUsageTemplate, GatewayClient, GatewayEndpoint,
    vw_api_types::{agent_apps::AppId, apps_usage::UsageSnapshotQuery},
};

use super::{Draft, Editor, Event, Failure, Kind, run_trial};

const TOKEN: &str = "fixture-token";
const PRESET: &str = "({ request: { url: '{{baseUrl}}/balance' } })";

fn selection() -> UsageSnapshotQuery {
    UsageSnapshotQuery {
        project_id: "project-a".into(),
        target_host_id: "local".into(),
        app_id: AppId::Codex,
        provider_id: "provider-a".into(),
        account_id: None,
        kind: None,
    }
}

#[tokio::test]
async fn usage_usage_script_returns_expected_result() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let server = std::thread::spawn(move || {
        let response = [
            json!([{"template": "custom", "source": PRESET}, {"template": "newapi", "source": "x"}]),
            json!([{"planName": "Fixture plan", "remaining": 0, "unit": "USD"}]),
        ];
        for (index, response) in response.into_iter().enumerate() {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            let mut headers = String::new();
            loop {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).unwrap() > 0);
                if line == "\r\n" {
                    break;
                }
                headers.push_str(&line.to_ascii_lowercase());
            }
            assert!(headers.contains("x-agent-apps-token: fixture-token\r\n"));
            if index == 0 {
                assert!(request.starts_with("GET /v1/apps/usage/templates?"));
                assert!(request.contains("providerId=provider-a"));
            } else {
                assert!(request.starts_with("POST /v1/apps/usage/script-test "));
                let len = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();
                let mut body = vec![0; len];
                reader.read_exact(&mut body).unwrap();
                let trial: serde_json::Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(trial["selection"]["providerId"], "provider-a");
                assert_eq!(trial["template"], "custom");
                assert_eq!(trial["timeoutSeconds"], 10);
                assert_eq!(trial["credentials"], json!({}));
                assert!(trial["source"].as_str().unwrap().contains("// fixture edit"));
            }
            let body = response.to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
    });
    let client = GatewayClient::new(endpoint).unwrap();
    let selection = selection();
    let templates = client.apps_usage_templates(&selection, TOKEN).await.unwrap();
    let mut draft = Draft::new(templates);
    assert_eq!(draft.source.text().trim_end(), PRESET);
    draft
        .source
        .perform(Action::Edit(Edit::Paste(std::sync::Arc::new("// fixture edit\n".into()))));
    let trial = draft.request(&selection).unwrap();
    let result = run_trial(client, trial, TOKEN.into()).await.unwrap();
    assert_eq!(result[0].plan_name.as_deref(), Some("Fixture plan"));
    assert_eq!(f64::from(result[0].remaining.unwrap()), 0.0);
    server.join().unwrap();

    use crate::app::components::apps_usage_details_tests::{Scene, renderer};
    use iced::{Length, widget::Space};
    let editor = Editor::Ready(Box::new(super::Ready { draft, pending: None, packages: result }));
    let base = Space::new().width(Length::Fill).height(Length::Fill).into();
    let scene = Scene::new(super::modal_view(&editor, &selection, base), &mut renderer().await);
    assert!(scene.bounds("Fixture plan · 剩余 0 USD").is_some());
}

#[test]
fn usage_usage_script_rejects_invalid_input() {
    let mut draft = Draft::new(vec![
        AppsUsageTemplate { template: "custom".into(), source: Some("   ".into()) },
        AppsUsageTemplate { template: "general".into(), source: Some(PRESET.into()) },
    ]);
    let selection = selection();
    assert!(draft.request(&selection).is_err());
    draft.select(Kind::General);
    draft.key_ref = "raw-key-fixture".into();
    assert!(draft.request(&selection).is_err());
    draft.key_ref = "a0a0a0a0-0000-4000-8000-000000000001".into();
    draft.timeout = "31".into();
    assert!(draft.request(&selection).is_err());
    draft.timeout = "2".into();
    let trial = draft.request(&selection).unwrap();
    assert!(trial.source.is_none());
    assert_eq!(trial.credentials.api_key_ref.as_deref(), Some(draft.key_ref.as_str()));
}

#[tokio::test]
async fn usage_usage_script_cancellation_disconnects_the_sdk_request() {
    use iced::futures::StreamExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let mut editor = Editor::Loading(1);
    let mut sequence = 1;
    let selection = selection();
    let _ = editor
        .update(
            Event::Loaded(
                1,
                Some(vec![AppsUsageTemplate {
                    template: "custom".into(),
                    source: Some(PRESET.into()),
                }]),
            ),
            &selection,
            endpoint.clone(),
            TOKEN,
            &mut sequence,
        )
        .unwrap();
    let task =
        editor.update(Event::Run, &selection, endpoint.clone(), TOKEN, &mut sequence).unwrap();
    let mut actions = iced_runtime::task::into_stream(task).unwrap();

    tokio::time::timeout(Duration::from_secs(5), async {
        let read_request = async {
            let (mut stream, _) = listener.accept().await.unwrap();
            let prefix = b"POST /v1/apps/usage/script-test ";
            let mut request = [0; 32];
            stream.read_exact(&mut request[..prefix.len()]).await.unwrap();
            assert_eq!(&request[..prefix.len()], prefix);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\n\r\n[")
                .await
                .unwrap();
            stream
        };
        let mut stream = tokio::select! {
            result = actions.next() => panic!("trial completed before cancellation: {result:?}"),
            stream = read_request => stream,
        };
        editor.cancel();
        assert!(actions.next().await.is_none());
        drop(actions);
        if let Err(error) = stream.read_to_end(&mut Vec::new()).await {
            assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset);
        }
    })
    .await
    .expect("cancelling must disconnect the SDK request");

    // An event already queued when the modal cancels must have no effect.
    assert!(
        iced_runtime::task::into_stream(
            editor
                .update(
                    Event::Finished(sequence, Err(Failure::Timeout)),
                    &selection,
                    endpoint,
                    TOKEN,
                    &mut sequence,
                )
                .unwrap()
        )
        .is_none()
    );
}

#[test]
fn usage_usage_script_hides_gateway_error_details() {
    use vw_gateway_client::{
        AppsUsageWriteError,
        vw_api_types::agent_apps_auth::{AuthError, AuthErrorCode},
    };

    let failure = Failure::from_gateway(AppsUsageWriteError::Http {
        status: 403,
        error: Some(AuthError {
            code: AuthErrorCode::Forbidden,
            message: "raw-secret-fixture".into(),
            correlation_id: "untrusted-id-fixture".into(),
        }),
    });
    assert_eq!(failure, Failure::Denied);
    assert_eq!(failure.label(), "脚本试运行权限不足，或目标地址未获授权");
}
