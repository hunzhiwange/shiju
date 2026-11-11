use super::{AggregationApiError as Error, test_support};
use serde_json::json;
use vw_api_types::agent_apps::aggregation::*;
use vw_api_types::app_providers::{omp::OmpProfile, pi::PiProfile};

fn scope() -> AggregationQuery {
    AggregationQuery { target_host_id: "remote-host".into(), project_id: "project-a".into() }
}
fn pi(change: PiDirectoryChange) -> PiDirectoryWrite {
    PiDirectoryWrite {
        target_host_id: scope().target_host_id,
        project_id: scope().project_id,
        expected_revision: "pi-revision".into(),
        change,
    }
}
fn omp(change: OmpDirectoryChange) -> OmpDirectoryWrite {
    OmpDirectoryWrite {
        target_host_id: scope().target_host_id,
        project_id: scope().project_id,
        expected_revision: "omp-revision".into(),
        change,
    }
}
fn state() -> serde_json::Value {
    json!({"revision":"next", "members":["member"], "defaultProviderId":"builtin", "defaultModelId":"model"})
}

#[tokio::test]
async fn aggregation_t55_aggregation_native_api_preserves_contract() {
    for app in ["pi", "omp"] {
        let body = json!({"error":{"code":"revision_conflict","message":"revision_conflict"}});
        let server = test_support::server(vec![(409, body.clone())]);
        let result = if app == "pi" {
            server
                .client()
                .pi_directory_entry("member", &pi(PiDirectoryChange::Remove {}))
                .await
                .map(|_| ())
        } else {
            server
                .client()
                .omp_directory_entry("member", &omp(OmpDirectoryChange::Remove {}))
                .await
                .map(|_| ())
        };
        assert_eq!(result, Err(Error::Http { status: 409, body }));
        let request = server.take_request();
        assert_eq!(request.method, "PUT");
        assert_eq!(request.path, format!("/v1/app-providers/{app}/member/directory-entry"));
        assert_eq!(request.body["change"], json!({"kind":"remove"}));
        server.join();
    }
}

#[tokio::test]
async fn aggregation_t55_separate_profiles_and_scoped_state_roundtrip() {
    let mut omp_state = state();
    omp_state["profile"] = json!("host-selected");
    let server = test_support::server(vec![
        (200, state()),
        (200, omp_state.clone()),
        (200, state()),
        (200, state()),
        (200, omp_state.clone()),
        (200, omp_state),
    ]);
    let client = server.client();
    let pi_state = client.pi_directory_state(&scope()).await.unwrap();
    assert_eq!(pi_state.default_provider_id.as_deref(), Some("builtin"));
    let state = client.omp_directory_state(&scope()).await.unwrap();
    assert_eq!(state.profile.as_deref(), Some("host-selected"));
    let declaration = json!({"baseUrl":"https://example.invalid/v1","models":[{"id":"m"}]});
    for change in [
        PiDirectoryChange::Insert {
            profile: PiProfile { name: "pi-name".into(), declaration: declaration.clone() },
        },
        PiDirectoryChange::Replace {
            profile: PiProfile { name: "pi-name".into(), declaration: declaration.clone() },
        },
    ] {
        assert_eq!(
            client.pi_directory_entry("member", &pi(change)).await.unwrap().revision,
            "next"
        );
    }
    for change in [
        OmpDirectoryChange::Insert {
            profile: OmpProfile { name: "omp-name".into(), declaration: declaration.clone() },
        },
        OmpDirectoryChange::Replace {
            profile: OmpProfile { name: "omp-name".into(), declaration: declaration.clone() },
        },
    ] {
        assert_eq!(
            client.omp_directory_entry("member", &omp(change)).await.unwrap().revision,
            "next"
        );
    }
    for app in ["pi", "omp"] {
        let request = server.take_request();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, format!("/v1/app-providers/{app}/config/state"));
        assert_eq!(request.body, serde_json::to_value(scope()).unwrap());
    }
    for (app, name) in [("pi", "pi-name"), ("omp", "omp-name")] {
        for kind in ["insert", "replace"] {
            let request = server.take_request();
            assert_eq!(request.path, format!("/v1/app-providers/{app}/member/directory-entry"));
            assert_eq!(request.body["expectedRevision"], format!("{app}-revision"));
            assert_eq!(
                request.body["change"],
                json!({"kind":kind,"profile":{"name":name,"declaration":declaration}})
            );
            assert_eq!(request.body["projectId"], "project-a");
            assert_eq!(request.body["targetHostId"], "remote-host");
            assert!(request.body.get("profile").is_none());
            assert!(request.header("x-platform-id").is_some());
            assert!(request.header("x-company-id").is_some());
        }
    }
    server.join();
}

#[tokio::test]
async fn aggregation_t55_invalid_identity_is_rejected_before_transport() {
    let server = test_support::server(vec![]);
    for id in ["", " ", "../escape", "a/b", "中文", &"x".repeat(129)] {
        assert!(matches!(
            server.client().pi_directory_entry(id, &pi(PiDirectoryChange::Remove {})).await,
            Err(Error::Transport(_))
        ));
        assert!(matches!(
            server.client().omp_directory_entry(id, &omp(OmpDirectoryChange::Remove {})).await,
            Err(Error::Transport(_))
        ));
    }
    server.join();
}

#[tokio::test]
async fn aggregation_t55_permission_and_target_failures_remain_errors() {
    for (status, code) in [
        (401, "unauthenticated"),
        (403, "forbidden"),
        (404, "not_found"),
        (422, "unsupported_target"),
        (502, "target_failed"),
        (503, "target_unavailable"),
    ] {
        let body = json!({"error":{"code":code,"message":code}});
        let server = test_support::server(vec![(status, body.clone()), (status, body.clone())]);
        assert_eq!(
            server.client().pi_directory_state(&scope()).await.unwrap_err(),
            Error::Http { status, body: body.clone() }
        );
        assert_eq!(
            server.client().omp_directory_state(&scope()).await.unwrap_err(),
            Error::Http { status, body }
        );
        server.join();
    }
    let server = test_support::server(vec![(200, json!({})), (200, json!({}))]);
    assert!(matches!(server.client().pi_directory_state(&scope()).await, Err(Error::Decode(_))));
    assert!(matches!(server.client().omp_directory_state(&scope()).await, Err(Error::Decode(_))));
    server.join();
}

#[tokio::test]
async fn aggregation_t55_bearer_context_and_redirect_do_not_replay_writes() {
    use crate::{GatewayAuth, GatewayClient, GatewayEndpoint};
    let server = test_support::server(vec![
        (307, json!({})),
        (200, state()),
        (
            200,
            json!({"revision":"r","profile":null,"members":[],"defaultProviderId":null,"defaultModelId":null}),
        ),
    ]);
    let endpoint = server
        .client()
        .endpoint()
        .clone()
        .with_auth(GatewayAuth { skey: Some("fixture-scoped-bearer".into()) });
    let client = GatewayClient::new(endpoint).unwrap();
    assert!(matches!(
        client.pi_directory_entry("member", &pi(PiDirectoryChange::Remove {})).await,
        Err(Error::Http { status: 307, .. })
    ));
    assert_eq!(client.pi_directory_state(&scope()).await.unwrap().revision, "next");
    assert!(client.omp_directory_state(&scope()).await.unwrap().profile.is_none());
    for _ in 0..3 {
        let request = server.take_request();
        assert_eq!(request.header("authorization"), Some("Bearer fixture-scoped-bearer"));
        assert!(request.header("x-agent-apps-token").is_none());
        assert!(request.body.get("token").is_none());
    }
    server.join();
    // A closed local listener produces a transport error, never a successful empty state.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    assert!(matches!(client.omp_directory_state(&scope()).await, Err(Error::Transport(_))));
}

#[tokio::test]
async fn aggregation_t55_redirect_location_is_not_followed() {
    use crate::{GatewayClient, GatewayEndpoint};
    use std::io::{BufRead, Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
        let mut reader = std::io::BufReader::new(&mut stream);
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["change"]["kind"],
            "remove"
        );
        write!(stream,"HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:{port}/replayed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        drop(stream);
        listener.set_nonblocking(true).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(300);
        while std::time::Instant::now() < deadline {
            match listener.accept() {
                Ok(_) => panic!("directory write replayed"),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5))
                }
                Err(e) => panic!("{e}"),
            }
        }
    });
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    assert!(matches!(
        client.omp_directory_entry("member", &omp(OmpDirectoryChange::Remove {})).await,
        Err(Error::Http { status: 307, .. })
    ));
    handle.join().unwrap();
}
