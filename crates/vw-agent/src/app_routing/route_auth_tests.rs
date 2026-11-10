use super::{API_KEY, RouteAuth, VerifiedRouteBinding, serve};
use crate::{
    app_routing::{RoutingListener, SharedTransport},
    model_relay::{RelayBinding, RelayIdentity},
};
use axum::{
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

const DEADLINE: Duration = Duration::from_secs(10);
const LOOPBACK: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

fn binding() -> VerifiedRouteBinding {
    VerifiedRouteBinding {
        project: ProjectScope::new(
            TenantScope::new("platform", "company").unwrap(),
            "project".into(),
        )
        .unwrap(),
        host: "host".into(),
        app: AppId::Claude,
        account: "account".into(),
        account_generation: 3,
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().timeout(DEADLINE).build().unwrap()
}

fn bearer(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
    headers
}

#[tokio::test]
async fn cc_route_008_route_auth_expected_behavior() {
    let listener = RoutingListener::default();
    let address = listener.start_on("127.0.0.1", 0).unwrap();
    let client = client();
    let response = client
        .post(format!("http://{address}/v1/messages"))
        .bearer_auth("fixture-management-skey")
        .body("unparsed request body")
        .send()
        .await
        .expect("the production listener must reject invalid credentials at the HTTP boundary");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let scope = binding();
    let session = RelayBinding::new(
        RelayIdentity {
            tenant: scope.project.tenant_scope().clone(),
            project: scope.project.project_id().as_ref().to_owned(),
            session: "session".into(),
            provider: "provider".into(),
            model: "model".into(),
            generation: 1,
        },
        Duration::from_secs(60),
    )
    .unwrap();
    let issued = listener.auth.issue(scope).unwrap();
    let url = format!("http://{address}/v1/messages");
    for (headers, status) in [
        (HeaderMap::new(), StatusCode::UNAUTHORIZED),
        (bearer(session.bearer_token()), StatusCode::UNAUTHORIZED),
        (bearer(&issued.token), StatusCode::SERVICE_UNAVAILABLE),
    ] {
        assert_eq!(client.post(&url).headers(headers).send().await.unwrap().status(), status);
    }
    // A rejected Expect request must get a final response without sending its body.
    let mut connection = TcpStream::connect(address).await.unwrap();
    connection.write_all(b"POST /v1/messages HTTP/1.1\r\nHost: localhost\r\nContent-Length: 100\r\nExpect: 100-continue\r\n\r\n").await.unwrap();
    let mut rejected = Vec::new();
    timeout(DEADLINE, connection.read_to_end(&mut rejected)).await.unwrap().unwrap();
    assert!(rejected.starts_with(b"HTTP/1.1 401 Unauthorized\r\n"));
    listener.stop().await.unwrap();
}

#[test]
fn cc_route_008_route_auth_binds_each_dimension_and_rejects_stale_generations() {
    let auth = RouteAuth::default();
    let scope = binding();
    let first = auth.issue(scope.clone()).unwrap();
    let first_header = bearer(&first.token);
    let permit = auth.authorize(&first_header).unwrap();
    assert!(permit.matches(&scope, first.revocation.generation));
    let foreign_tenant = ProjectScope::new(
        TenantScope::new("platform", "another-company").unwrap(),
        scope.project.project_id().clone(),
    )
    .unwrap();
    let foreign_project =
        ProjectScope::new(scope.project.tenant_scope().clone(), "another-project".into()).unwrap();
    let wrong_scopes = [
        VerifiedRouteBinding { project: foreign_tenant, ..scope.clone() },
        VerifiedRouteBinding { project: foreign_project, ..scope.clone() },
        VerifiedRouteBinding { host: "another-host".into(), ..scope.clone() },
        VerifiedRouteBinding { app: AppId::ClaudeDesktop, ..scope.clone() },
        VerifiedRouteBinding { account: "another-account".into(), ..scope.clone() },
        VerifiedRouteBinding { account_generation: 4, ..scope.clone() },
    ];
    for foreign in &wrong_scopes {
        assert!(!permit.matches(foreign, first.revocation.generation));
    }
    let other = auth.issue(wrong_scopes[3].clone()).unwrap();
    assert!(
        !auth
            .authorize(&bearer(&other.token))
            .unwrap()
            .matches(&scope, first.revocation.generation)
    );
    assert!(permit.matches(&scope, first.revocation.generation));

    let mut duplicate = first_header.clone();
    duplicate.append(header::AUTHORIZATION, first_header[header::AUTHORIZATION].clone());
    assert!(auth.authorize(&duplicate).is_none());
    let mut ambiguous = first_header.clone();
    ambiguous.insert(API_KEY, first.token.parse().unwrap());
    assert!(auth.authorize(&ambiguous).is_none());
    let mut key = HeaderMap::new();
    key.insert(API_KEY, first.token.parse().unwrap());
    assert!(auth.authorize(&key).unwrap().matches(&scope, first.revocation.generation));

    let updated_scope = wrong_scopes[5].clone();
    let next = auth.issue(updated_scope.clone()).unwrap();
    assert!(next.token != first.token);
    assert!(next.revocation.generation > first.revocation.generation);
    assert!(permit.cancelled.is_cancelled());
    assert!(auth.authorize(&first_header).is_none());
    auth.revoke(&first.revocation);
    let current = auth.authorize(&bearer(&next.token)).unwrap();
    assert!(!current.matches(&updated_scope, first.revocation.generation));
    assert!(current.matches(&updated_scope, next.revocation.generation));
    assert!(auth.authorize(&bearer(&other.token)).is_some());
    auth.revoke(&next.revocation);
    auth.revoke(&next.revocation);
    assert!(!current.matches(&updated_scope, next.revocation.generation));
    assert!(auth.authorize(&bearer(&next.token)).is_none());
}

#[tokio::test]
async fn cc_route_008_route_auth_revokes_http_and_sse_in_flight() {
    timeout(DEADLINE, async {
        let upstream = TcpListener::bind(LOOPBACK).await.unwrap();
        let upstream_addr = upstream.local_addr().unwrap();
        let (sse_ready, sse_received) = oneshot::channel();
        let (http_ready, http_received) = oneshot::channel();
        let upstream_task = tokio::spawn(async move {
            let (sse, _) = upstream.accept().await.unwrap();
            let sse = tokio::spawn(hold_upstream(sse, "/sse", sse_ready));
            let (http, _) = upstream.accept().await.unwrap();
            hold_upstream(http, "/slow", http_ready).await;
            sse.await.unwrap();
        });

        let listener = RoutingListener::default();
        let expected = binding();
        let issued = listener.auth.issue(expected.clone()).unwrap();
        let foreign = listener
            .auth
            .issue(VerifiedRouteBinding { app: AppId::ClaudeDesktop, ..expected.clone() })
            .unwrap();
        let generation = issued.revocation.generation;
        let auth = Arc::clone(&listener.auth);
        let transport = Arc::new(SharedTransport::new().unwrap());
        let address = listener
            .start_at(LOOPBACK, move |connection| {
                let expected = expected.clone();
                let transport = Arc::clone(&transport);
                Some(serve(connection, Arc::clone(&auth), move |request, permit| {
                    let expected = expected.clone();
                    let transport = Arc::clone(&transport);
                    async move {
                        if !permit.matches(&expected, generation) {
                            return StatusCode::UNAUTHORIZED.into_response();
                        }
                        assert!(!request.headers().contains_key(header::AUTHORIZATION));
                        assert!(!request.headers().contains_key(API_KEY));
                        let url: reqwest::Url =
                            format!("http://{upstream_addr}{}", request.uri().path())
                                .parse()
                                .unwrap();
                        let request = reqwest::Request::new(reqwest::Method::GET, url.clone());
                        transport
                            .forward(&url, request, CancellationToken::new())
                            .await
                            .unwrap_or_else(|status| status.into_response())
                    }
                }))
            })
            .unwrap();

        let client = client();
        let url = format!("http://{address}");
        assert_eq!(
            client
                .get(format!("{url}/slow"))
                .bearer_auth(&foreign.token)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let mut sse =
            client.get(format!("{url}/sse")).header(API_KEY, &issued.token).send().await.unwrap();
        sse_received.await.unwrap();
        assert_eq!(sse.chunk().await.unwrap().unwrap(), "data: ready\n\n");
        let pending =
            tokio::spawn(client.get(format!("{url}/slow")).bearer_auth(&issued.token).send());
        http_received.await.unwrap();
        listener.auth.revoke(&issued.revocation);
        assert!(pending.await.unwrap().is_err());
        assert!(sse.chunk().await.is_err());
        upstream_task.await.unwrap();
        assert_eq!(
            client
                .get(format!("{url}/slow"))
                .bearer_auth(&issued.token)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        listener.stop().await.unwrap();
    })
    .await
    .expect("revocation did not close both sides of the requests");
}

async fn hold_upstream(mut socket: TcpStream, path: &str, ready: oneshot::Sender<()>) {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        bytes.push(socket.read_u8().await.unwrap());
        assert!(bytes.len() <= 4096);
    }
    let headers = String::from_utf8(bytes).unwrap();
    assert_eq!(headers.lines().next().unwrap(), format!("GET {path} HTTP/1.1"));
    assert!(!headers.contains("sra_"));
    if path == "/sse" {
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 1024\r\n\r\ndata: ready\n\n").await.unwrap();
    }
    ready.send(()).unwrap();
    assert_eq!(socket.read(&mut [0]).await.unwrap(), 0, "revocation left the upstream open");
}
