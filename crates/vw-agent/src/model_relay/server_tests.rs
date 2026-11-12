use super::*;
use axum::{body::Body, http::StatusCode, response::Response};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;

struct ExternalProvider {
    calls: AtomicUsize,
    mode: AtomicUsize,
    cancellation: parking_lot::Mutex<Option<CancellationToken>>,
}
#[async_trait::async_trait]
impl RelayBackend for ExternalProvider {
    async fn execute(
        &self,
        identity: &RelayIdentity,
        request: RelayRequest,
        cancel: CancellationToken,
    ) -> Result<Response, StatusCode> {
        assert_eq!(crate::session::ui_store::current_tenant_scope(), identity.tenant);
        assert_eq!(crate::project::instance::project().unwrap().id, identity.project);
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.cancellation.lock() = Some(cancel);
        assert_eq!(request.path, "/v1/chat/completions");
        assert!(
            serde_json::from_slice::<serde_json::Value>(&request.body)
                .unwrap()
                .get("messages")
                .is_some()
        );
        match self.mode.load(Ordering::SeqCst) {
            1 => Ok(Response::builder()
                .status(307)
                .header("location", "http://127.0.0.1:1/secret")
                .body(Body::empty())
                .unwrap()),
            2 => {
                let chunks = futures_util::stream::once(async {
                    Ok::<_, std::io::Error>(axum::body::Bytes::from_static(b"first"))
                });
                use futures_util::StreamExt;
                Ok(Response::new(Body::from_stream(chunks.chain(futures_util::stream::pending()))))
            }
            _ => Ok(Response::new(Body::from("{\"result\":\"ok\"}"))),
        }
    }
}
struct EnvGuard(Option<std::ffi::OsString>);
impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match self.0.take() {
                Some(v) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", v),
                None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
            }
        }
    }
}

#[tokio::test]
async fn acp_model_source_relay_binding_http_isolation_limits_and_lifecycle() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();
    let _env = EnvGuard(std::env::var_os("VIBEWINDOW_CONFIG_DIR"));
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", dir.path().join("config"));
    }
    let tenant = vw_api_types::id::TenantScope::new("relay", "one").unwrap();
    let root = dir.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let (project, _) = crate::project::from_directory(&tenant, &root).await.unwrap();
    crate::config::update_project_for_tenant(&tenant, &root, serde_json::json!({"providers": {"relay-fixture": {"api": "http://127.0.0.1:1/v1", "models": {"model": {"status": "active"}}}}})).await.unwrap();
    let context = crate::project::instance::Context {
        directory: root.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project: project.clone(),
    };
    let id = RelayIdentity {
        tenant,
        project: project.id,
        session: "one".into(),
        provider: "relay-fixture".into(),
        model: "model".into(),
        generation: 1,
    };
    let binding = RelayBinding::new(id.clone(), Duration::from_secs(60)).unwrap();
    let backend = Arc::new(ExternalProvider {
        calls: AtomicUsize::new(0),
        mode: AtomicUsize::new(0),
        cancellation: parking_lot::Mutex::new(None),
    });
    let server =
        RelayServer::start(binding.clone(), backend.clone(), context.clone()).await.unwrap();
    assert!(server.address().ip().is_loopback());
    let url = format!("http://{}/v1/chat/completions", server.address());
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let body = serde_json::json!({"model": "model", "messages": []});
    assert_eq!(client.post(&url).json(&body).send().await.unwrap().status(), 401);
    let foreign = RelayBinding::new(id.clone(), Duration::from_secs(60)).unwrap();
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(foreign.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "{\"result\":\"ok\"}"
    );
    for key in ["base_url", "api_key", "headers", "tenant", "generation"] {
        let mut bad = body.clone();
        bad[key] = "attacker".into();
        assert_eq!(
            client
                .post(&url)
                .bearer_auth(binding.bearer_token())
                .json(&bad)
                .send()
                .await
                .unwrap()
                .status(),
            400
        );
    }
    assert_eq!(
        client
            .post(format!("{url}?url=http://evil"))
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&serde_json::json!({"model":"other"}))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        client
            .post(format!("http://{}/admin", server.address()))
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .body(vec![b'x'; 4 * 1024 * 1024 + 1])
            .send()
            .await
            .unwrap()
            .status(),
        413
    );
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
    backend.mode.store(1, Ordering::SeqCst);
    let redirect =
        client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
    assert_eq!(redirect.status(), 502);
    assert!(!redirect.headers().contains_key("location"));
    backend.mode.store(2, Ordering::SeqCst);
    let mut response =
        client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
    assert_eq!(response.chunk().await.unwrap().unwrap(), "first");
    let disconnected = backend.cancellation.lock().clone().unwrap();
    drop(response);
    tokio::time::timeout(Duration::from_secs(3), disconnected.cancelled()).await.unwrap();
    let mut held = Vec::new();
    for _ in 0..4 {
        let mut response =
            client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
        response.chunk().await.unwrap().unwrap();
        held.push(response);
    }
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        429
    );
    drop(held);
    tokio::time::sleep(Duration::from_millis(30)).await;
    backend.mode.store(0, Ordering::SeqCst);
    // An actual child process crosses the HTTP boundary; the capability travels over stdin.
    use tokio::io::AsyncWriteExt;
    let mut child = tokio::process::Command::new("python3").arg("-c").arg("import sys,json,urllib.request; v=json.load(sys.stdin); r=urllib.request.urlopen(urllib.request.Request(v['url'], data=b'{\"model\":\"model\",\"messages\":[]}', headers={'Authorization':'Bearer '+v['token'],'Content-Type':'application/json'})); print(r.status); r.read()")
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            serde_json::to_string(&serde_json::json!({"url":url,"token":binding.bearer_token()}))
                .unwrap()
                .as_bytes(),
        )
        .await
        .unwrap();
    let output = child.wait_with_output().await.unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "200");
    // Model removal is checked against the real tenant/project catalog, not a mock flag.
    crate::config::update_project_for_tenant(&id.tenant, &root, serde_json::json!({"providers":{"relay-fixture":{"models":{"model":{"status":"disabled"}}}}})).await.unwrap();
    let count = backend.calls.load(Ordering::SeqCst);
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(backend.calls.load(Ordering::SeqCst), count);
    // Restore the model before checking the per-listener request rate.
    crate::config::update_project_for_tenant(
        &id.tenant,
        &root,
        serde_json::json!({"providers":{"relay-fixture":{"models":{"model":{"status":"active"}}}}}),
    )
    .await
    .unwrap();
    backend.mode.store(0, Ordering::SeqCst);
    let limited = RelayBinding::new(id.clone(), Duration::from_secs(60)).unwrap();
    let rate_server = RelayServer::start(limited.clone(), backend.clone(), context).await.unwrap();
    for _ in 0..60 {
        let response = client
            .post(format!("http://{}/v1/chat/completions", rate_server.address()))
            .bearer_auth(limited.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        response.bytes().await.unwrap();
    }
    assert_eq!(
        client
            .post(format!("http://{}/v1/chat/completions", rate_server.address()))
            .bearer_auth(limited.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        429
    );
    rate_server.shutdown().await.unwrap();
    binding.revoke();
    tokio::time::timeout(Duration::from_secs(3), server.shutdown()).await.unwrap().unwrap();
    assert!(client.post(&url).json(&body).send().await.is_err());
}

#[tokio::test]
async fn acp_model_source_relay_expiry_cancels_an_established_http_stream() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();
    let _env = EnvGuard(std::env::var_os("VIBEWINDOW_CONFIG_DIR"));
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", dir.path().join("config"));
    }
    let tenant = vw_api_types::id::TenantScope::new("relay-expiry", "one").unwrap();
    let root = dir.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let (project, _) = crate::project::from_directory(&tenant, &root).await.unwrap();
    crate::config::update_project_for_tenant(&tenant, &root, serde_json::json!({"providers": {"relay-fixture": {"api": "http://127.0.0.1:1/v1", "models": {"model": {"status": "active"}}}}})).await.unwrap();
    let context = crate::project::instance::Context {
        directory: root.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project: project.clone(),
    };
    let id = RelayIdentity {
        tenant,
        project: project.id,
        session: "one".into(),
        provider: "relay-fixture".into(),
        model: "model".into(),
        generation: 1,
    };
    let backend = Arc::new(ExternalProvider {
        calls: AtomicUsize::new(0),
        mode: AtomicUsize::new(2),
        cancellation: parking_lot::Mutex::new(None),
    });
    // Let catalog setup finish before advancing time; startup latency is not a
    // lease-expiry assertion. This test owns its runtime so the clock stays local.
    let binding = RelayBinding::new(id.clone(), Duration::from_secs(60)).unwrap();
    let server = RelayServer::start(binding.clone(), backend.clone(), context).await.unwrap();
    let mut response = reqwest::Client::new()
        .post(format!("http://{}/v1/chat/completions", server.address()))
        .bearer_auth(binding.bearer_token())
        .json(&serde_json::json!({"model":"model","messages":[],"stream":true}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.chunk().await.unwrap().unwrap().as_ref(), b"first");
    let cancel = backend.cancellation.lock().clone().unwrap();
    assert!(!cancel.is_cancelled());
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(61)).await;
    tokio::time::timeout(Duration::from_secs(3), binding.cancelled.cancelled()).await.unwrap();
    assert!(!binding.accepts(binding.bearer_token(), &id));
    tokio::time::timeout(Duration::from_secs(3), cancel.cancelled()).await.unwrap();
    assert!(response.chunk().await.unwrap().is_none());
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn acp_model_source_relay_binding_upstream_redirect_is_not_followed() {
    use axum::{Router, routing::post};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let target_hits = hits.clone();
    let router = Router::new()
        .route(
            "/redirect",
            post(|| async { (StatusCode::TEMPORARY_REDIRECT, [("location", "/target")], "") }),
        )
        .route(
            "/target",
            post(move || {
                let hits = target_hits.clone();
                async move {
                    hits.fetch_add(1, Ordering::SeqCst);
                    "unexpected"
                }
            }),
        );
    let stop = CancellationToken::new();
    let stopping = stop.clone();
    let task = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(stopping.cancelled_owned())
            .await
            .unwrap();
    });
    let response = upstream_http_client()
        .unwrap()
        .post(format!("http://{address}/redirect"))
        .bearer_auth("fixture-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 307);
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    drop(response);
    stop.cancel();
    task.await.unwrap();
}
