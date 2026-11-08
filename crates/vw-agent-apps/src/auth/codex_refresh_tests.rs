use super::{
    codex_refresh::CodexRefresh,
    copilot_tests::oauth_server,
    credential_file::{AccountCredential, CredentialFileStore},
    flow::AuthAccountScope,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Request, Url};
use serde_json::{Value, json};
use std::{os::unix::fs::PermissionsExt, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::oneshot,
    task::JoinHandle,
    time::timeout,
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthService},
    id::TenantScope,
};

pub(super) fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "oidc:owner",
        host: "host",
        service: AuthService::CodexOauth,
    }
}

fn jwt(claims: Value) -> String {
    format!(
        "{}.{}.fixture-signature",
        URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256"}"#),
        URL_SAFE_NO_PAD.encode(claims.to_string()),
    )
}

pub(super) fn seed(
    store: &CredentialFileStore,
    scope: AuthAccountScope<'_>,
    account_id: &str,
) -> Value {
    let credential = json!({
        "auth_mode": "chatgpt",
        "tokens": {
            "account_id": format!("workspace-{account_id}"),
            "access_token": jwt(json!({"exp": 1})),
            "id_token": jwt(json!({"sub": format!("user-{account_id}")})),
            "refresh_token": format!("refresh-{account_id}"),
        }
    });
    store
        .for_owner(scope.principal)
        .unwrap()
        .save(
            scope.tenant,
            scope.host,
            AppId::Codex,
            account_id,
            &AccountCredential::CodexAuth(credential.clone()),
        )
        .unwrap();
    credential
}

pub(super) fn saved(store: &CredentialFileStore, scope: AuthAccountScope<'_>, id: &str) -> Value {
    let AccountCredential::CodexAuth(value) = store
        .for_owner(scope.principal)
        .unwrap()
        .lease(scope.tenant, scope.host, AppId::Codex, id, false)
        .unwrap()
        .read()
        .unwrap()
    else {
        panic!("wrong credential kind")
    };
    value
}

pub(super) fn route(request: &mut Request, server: &Url) {
    assert_eq!(request.url().as_str(), "https://auth.openai.com/oauth/token");
    let mut url = server.clone();
    url.set_path("/oauth/token");
    *request.url_mut() = url;
}

fn assert_form(request: &str, id: &str) {
    let (headers, body) = request.split_once("\r\n\r\n").unwrap();
    assert!(headers.starts_with("POST /oauth/token HTTP/1.1\r\n"));
    assert!(headers.contains("\r\ncontent-type: application/x-www-form-urlencoded\r\n"));
    assert!(headers.contains("\r\nuser-agent: cc-switch-codex-oauth\r\n"));
    assert_eq!(
        body,
        format!(
            "grant_type=refresh_token&refresh_token=refresh-{id}&client_id=app_EMoamEEZ73f0CkXaXp7hrann&scope=openid+profile+email"
        )
    );
}

pub(super) async fn gated_refresh(
    reply: Value,
) -> (Url, oneshot::Receiver<()>, oneshot::Sender<()>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let (received, seen) = oneshot::channel();
    let (release, wait) = oneshot::channel();
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut headers = String::new();
        while !headers.ends_with("\r\n\r\n") {
            assert_ne!(stream.read_line(&mut headers).await.unwrap(), 0);
        }
        let size: usize = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length: "))
            .unwrap()
            .parse()
            .unwrap();
        let mut body = vec![0; size];
        stream.read_exact(&mut body).await.unwrap();
        assert_form(&format!("{headers}{}", String::from_utf8(body).unwrap()), "alpha");
        received.send(()).unwrap();
        wait.await.unwrap();
        let reply = reply.to_string();
        stream.get_mut().write_all(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
            reply.len(),
        ).as_bytes()).await.unwrap();
        // Keep listening until the waiters finish so any second request is observable.
        let _ = listener.accept().await.unwrap();
        panic!("a second alpha refresh reached the HTTP server");
    });
    (server, seen, release, task)
}

#[tokio::test]
async fn auth6_codex_refresh_commits_expected_scoped_result() {
    let auth = CodexRefresh::new().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    seed(&store, scope, "alpha");
    let beta_before = seed(&store, scope, "beta");
    let foreign = AuthAccountScope { principal: "oidc:other", ..scope };
    let foreign_before = seed(&store, foreign, "alpha");
    let alpha_id = jwt(json!({"sub":"user-alpha", "chatgpt_account_id":"workspace-alpha"}));
    let (alpha, seen, release, alpha_server) = gated_refresh(json!({
        "access_token":"access-alpha-next", "refresh_token":"refresh-alpha-next",
        "id_token":alpha_id, "expires_in":3600
    }))
    .await;
    let (beta, beta_server) = oauth_server(
        "200 OK",
        "",
        json!({
            "access_token":"access-beta-next", "expires_in":3600
        }),
    )
    .await;

    let first = async {
        let result =
            auth.access_token_with(&store, scope, "alpha", |request| route(request, &alpha)).await;
        assert_eq!(result.as_ref().err(), None, "a valid refresh should commit");
        result.unwrap()
    };
    let others = async {
        seen.await.unwrap();
        let second =
            auth.access_token_with(&store, scope, "alpha", |request| route(request, &alpha));
        let independent = async {
            let result = auth
                .access_token_with(&store, scope, "beta", |request| route(request, &beta))
                .await;
            assert_eq!(result.as_ref().err(), None, "beta should finish while alpha is in flight");
            assert_eq!(result.unwrap(), "access-beta-next");
            release.send(()).unwrap();
        };
        let (second, ()) = tokio::join!(second, independent);
        second
    };
    let (first, second) = timeout(Duration::from_secs(5), async { tokio::join!(first, others) })
        .await
        .expect("account refreshes should complete independently");
    assert_eq!(first, "access-alpha-next");
    assert_eq!(second.unwrap(), "access-alpha-next");
    assert_form(&beta_server.await.unwrap(), "beta");
    alpha_server.abort();
    assert!(alpha_server.await.unwrap_err().is_cancelled());
    let alpha_saved = saved(&store, scope, "alpha");
    assert_eq!(
        alpha_saved["tokens"],
        json!({
            "access_token":"access-alpha-next", "refresh_token":"refresh-alpha-next",
            "id_token":alpha_id, "account_id":"workspace-alpha"
        })
    );
    assert!(alpha_saved["last_refresh"].as_str().is_some());
    let beta_saved = saved(&store, scope, "beta");
    assert_eq!(beta_saved["tokens"]["access_token"], "access-beta-next");
    assert_eq!(beta_saved["tokens"]["id_token"], beta_before["tokens"]["id_token"]);
    assert_eq!(beta_saved["tokens"]["refresh_token"], beta_before["tokens"]["refresh_token"]);
    assert_eq!(saved(&store, foreign, "alpha"), foreign_before);
}

#[tokio::test]
async fn auth6_codex_refresh_rejects_invalid_or_stale_input() {
    let auth = CodexRefresh::new().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    let before = seed(&store, scope, "alpha");

    let invalid =
        auth.access_token_with(&store, scope, "../alpha", |_| panic!("invalid ID sent HTTP")).await;
    assert_eq!(invalid.unwrap_err(), AuthErrorCode::InvalidInput);
    let foreign = AuthAccountScope { host: "other-host", ..scope };
    let missing =
        auth.access_token_with(&store, foreign, "alpha", |_| panic!("wrong host sent HTTP")).await;
    assert_eq!(missing.unwrap_err(), AuthErrorCode::StorageUnavailable);

    for (status, body, expected) in [
        (
            "503 Service Unavailable",
            json!({"error":"fixture-private-detail"}),
            AuthErrorCode::UpstreamFailure,
        ),
        (
            "200 OK",
            json!({
                "access_token":"access-wrong-user",
                "id_token":jwt(json!({"sub":"user-other","chatgpt_account_id":"workspace-alpha"})),
                "refresh_token":"refresh-wrong-user",
            }),
            AuthErrorCode::IdentityMismatch,
        ),
        (
            "200 OK",
            json!({"access_token":"already-expired","refresh_token":"unused","expires_in":0}),
            AuthErrorCode::UpstreamFailure,
        ),
        (
            "400 Bad Request",
            json!({"error":{"code":"refresh_token_reused","message":"fixture-private-detail"}}),
            AuthErrorCode::ReauthRequired,
        ),
    ] {
        let (server, seen) = oauth_server(status, "", body).await;
        let result =
            auth.access_token_with(&store, scope, "alpha", |request| route(request, &server)).await;
        assert_eq!(result.unwrap_err(), expected);
        assert_form(&seen.await.unwrap(), "alpha");
        assert_eq!(saved(&store, scope, "alpha"), before);
    }
    let rejected = auth
        .access_token_with(&store, scope, "alpha", |_| panic!("rejected generation retried"))
        .await;
    assert_eq!(rejected.unwrap_err(), AuthErrorCode::ReauthRequired);

    let mut replacement = before;
    let access = jwt(json!({"exp":4102444800_i64}));
    replacement["tokens"]["access_token"] = access.clone().into();
    replacement["tokens"]["refresh_token"] = "replacement-refresh".into();
    store
        .for_owner(scope.principal)
        .unwrap()
        .save(
            scope.tenant,
            scope.host,
            AppId::Codex,
            "alpha",
            &AccountCredential::CodexAuth(replacement),
        )
        .unwrap();
    let resolved =
        auth.access_token_with(&store, scope, "alpha", |_| panic!("replacement sent HTTP")).await;
    assert_eq!(resolved, Ok(access));
}

fn launch_rotation(
    root: &std::path::Path,
    lease: &super::credential_lease::CredentialLease,
    original: &Value,
    live: &Value,
    exit_code: u8,
) -> Result<(), String> {
    use crate::apps::launch::{account::prepare_account_launch, request::LaunchMaterial};
    use std::fs;

    let program = root.join("codex-fixture");
    fs::write(root.join("next-auth"), live.to_string()).unwrap();
    fs::write(
        &program,
        format!(
            "#!/bin/sh\ncat next-auth > \"$CODEX_HOME/auth.json\" || exit 1\nprintf '%s' \"$CODEX_HOME\" > cli-home\nexit {exit_code}\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
    let launch = prepare_account_launch(
        AppId::Codex,
        AccountCredential::CodexAuth(original.clone()),
        LaunchMaterial { program, cwd: root.into(), args: vec![], skip_permissions: false },
        root,
    )
    .unwrap();
    let result = launch.run_with_lease_using(lease, |command| command.status());
    let cli_home = fs::read_to_string(root.join("cli-home")).unwrap();
    assert!(!std::path::Path::new(&cli_home).exists(), "CLI home must be cleaned");
    result
}

#[tokio::test]
async fn auth6_codex_rotation_commits_expected_scoped_result() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    let other = AuthAccountScope { principal: "oidc:other", ..scope };
    let other_before = seed(&store, other, "alpha");
    let owner = store.for_owner(scope.principal).unwrap();
    let mut original = seed(&store, scope, "alpha");
    original["last_refresh"] = "2026-01-01T00:00:00Z".into();
    owner
        .save(
            scope.tenant,
            scope.host,
            AppId::Codex,
            "alpha",
            &AccountCredential::CodexAuth(original.clone()),
        )
        .unwrap();
    let lease = owner.lease(scope.tenant, scope.host, AppId::Codex, "alpha", false).unwrap();
    let mut live = original.clone();
    let access = jwt(json!({"exp":4102444800_i64,"chatgpt_account_id":"workspace-alpha"}));
    live["tokens"]["access_token"] = access.clone().into();
    live["tokens"]["refresh_token"] = "rotated-alpha".into();
    live["tokens"]["id_token"] =
        jwt(json!({"sub":"user-alpha","chatgpt_account_id":"workspace-alpha"})).into();
    live["last_refresh"] = "2026-01-02T00:00:00Z".into();
    assert_eq!(
        launch_rotation(root.path(), &lease, &original, &live, 7).unwrap_err(),
        "official account process exited unsuccessfully"
    );
    drop(lease);

    assert_eq!(saved(&store, scope, "alpha"), live);
    assert_eq!(saved(&store, other, "alpha"), other_before);
    let actual = CodexRefresh::new()
        .unwrap()
        .access_token_with(&store, scope, "alpha", |_| panic!("rotated access should be usable"))
        .await
        .unwrap();
    assert_eq!(actual, access);
}

#[tokio::test]
async fn auth6_codex_rotation_rejects_invalid_or_stale_input() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    let owner = store.for_owner(scope.principal).unwrap();
    let mut original = seed(&store, scope, "alpha");
    original["last_refresh"] = "2026-01-02T00:00:00Z".into();
    let mut next = original.clone();
    next["last_refresh"] = "2026-01-03T00:00:00Z".into();
    next["tokens"]["refresh_token"] = "rotated-alpha".into();

    let mut wrong_user = next.clone();
    wrong_user["tokens"]["id_token"] =
        jwt(json!({"sub":"different-user","chatgpt_account_id":"workspace-alpha"})).into();
    let mut wrong_workspace = next.clone();
    wrong_workspace["tokens"]["id_token"] =
        jwt(json!({"sub":"user-alpha","chatgpt_account_id":"other-workspace"})).into();
    let mut old = next.clone();
    old["last_refresh"] = "2026-01-01T00:00:00Z".into();
    let mut equal = next.clone();
    equal["last_refresh"] = "2026-01-02T08:00:00+08:00".into();
    let mut undated = next.clone();
    undated["last_refresh"] = Value::Null;
    let mut undated_original = original.clone();
    undated_original["last_refresh"] = Value::Null;
    let mut superseded = original.clone();
    superseded["tokens"]["refresh_token"] = "independently-rotated".into();
    superseded["last_refresh"] = "2026-01-02T12:00:00Z".into();

    for (label, stored, live, expected) in [
        (
            "same workspace, other user",
            &original,
            &wrong_user,
            "rotated official account identity changed",
        ),
        (
            "conflicting workspace claim",
            &original,
            &wrong_workspace,
            "rotated official account identity changed",
        ),
        ("old", &original, &old, "rotated official account generation is stale or ambiguous"),
        ("equal", &original, &equal, "rotated official account generation is stale or ambiguous"),
        (
            "undated",
            &original,
            &undated,
            "rotated official account generation is stale or ambiguous",
        ),
        (
            "undated baseline",
            &undated_original,
            &next,
            "rotated official account generation is stale or ambiguous",
        ),
        ("superseded launch", &superseded, &next, "official credential changed before publication"),
    ] {
        owner
            .save(
                scope.tenant,
                scope.host,
                AppId::Codex,
                "alpha",
                &AccountCredential::CodexAuth(stored.clone()),
            )
            .unwrap();
        let lease = owner.lease(scope.tenant, scope.host, AppId::Codex, "alpha", false).unwrap();
        let launch_auth = if label == "superseded launch" { &original } else { stored };
        let result = launch_rotation(root.path(), &lease, launch_auth, live, 0);
        assert_eq!(result.as_ref().err().map(String::as_str), Some(expected), "{label}");
        drop(lease);
        assert_eq!(&saved(&store, scope, "alpha"), stored, "{label}");
    }

    owner
        .save(
            scope.tenant,
            scope.host,
            AppId::Codex,
            "alpha",
            &AccountCredential::CodexAuth(original),
        )
        .unwrap();
    let (server, seen, release, server_task) = gated_refresh(json!({
        "access_token":"late-access",
        "refresh_token":"late-refresh",
    }))
    .await;
    let adapter = CodexRefresh::new().unwrap();
    let refresh =
        adapter.access_token_with(&store, scope, "alpha", |request| route(request, &server));
    let rotate_during_exchange = async {
        seen.await.unwrap();
        let fixture = root.path().join("outside-rotation");
        std::fs::write(&fixture, next.to_string()).unwrap();
        // Emulate a process that already has access to the file and ignores the advisory lease.
        assert!(
            std::process::Command::new("find")
                .arg(root.path().join(".managed-v1"))
                .args(["-type", "f", "!", "-name", "*.session-lock", "-exec", "/bin/sh", "-c"])
                .arg(r#"cat "$1" > "$2""#)
                .arg("fixture-writer")
                .arg(fixture)
                .args(["{}", ";"])
                .status()
                .unwrap()
                .success()
        );
        release.send(()).unwrap();
    };
    let (result, ()) =
        timeout(Duration::from_secs(5), async { tokio::join!(refresh, rotate_during_exchange) })
            .await
            .expect("the delayed exchange should complete");
    assert_eq!(result.unwrap_err(), AuthErrorCode::Conflict);
    assert_eq!(saved(&store, scope, "alpha"), next);
    server_task.abort();
    assert!(server_task.await.unwrap_err().is_cancelled());
}
