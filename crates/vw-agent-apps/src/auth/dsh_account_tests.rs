use super::{
    copilot_tests::oauth_server,
    dsh_account::DshAccount,
    flow::{AuthAccountScope, AuthFlows},
};
use crate::providers::live::commit::{ExecutionHostId, ProviderTransactions};
use reqwest::{Client, Request, Url};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{net::TcpListener, time::timeout};
use uuid::Uuid;
use vw_api_types::{
    agent_apps_auth::{
        AuthCredentialState, AuthErrorCode, AuthFlow, AuthLoginMethod, AuthService,
        AuthStartRequest,
    },
    id::TenantScope,
};

const ISSUER: &str = "https://platform.deepseek.com";
const GRANT: &str = "deepseek-account-platform/default";
const DEVICE: &str = "deepseek-account-platform/device";
const DEVICE_ID: &str = "4826593b-e124-4af4-8c75-eb740b7318b7";
const OLD_TOKEN: &str = "fixture-previous-grant";
const NEW_TOKEN: &str = "fixture-next-grant";

struct Fixture {
    _directory: tempfile::TempDir,
    home: PathBuf,
    credentials: PathBuf,
    tenant: TenantScope,
    transactions: ProviderTransactions,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let home = directory.path().join("home");
        let native = home.join(".dsh");
        fs::create_dir_all(&native).unwrap();
        let credentials = native.join(".credentials.yaml");
        let transaction_root = directory.path().join("private");
        fs::DirBuilder::new().mode(0o700).create(&transaction_root).unwrap();
        let transactions = ProviderTransactions::new(
            &transaction_root,
            ExecutionHostId::new("test-host").unwrap(),
        )
        .unwrap();
        Self {
            _directory: directory,
            home,
            credentials,
            tenant: TenantScope::new("platform", "company").unwrap(),
            transactions,
        }
    }

    fn scope(&self) -> AuthAccountScope<'_> {
        AuthAccountScope {
            tenant: &self.tenant,
            principal: "oidc:owner",
            host: "test-host",
            service: AuthService::DshAccount,
        }
    }

    fn account(&self) -> DshAccount<'_> {
        DshAccount::new(&self.home, &self.transactions, self.scope()).unwrap()
    }

    fn write(&self, document: &Value) {
        write_private(&self.credentials, serde_yaml::to_string(document).unwrap().as_bytes());
    }

    fn read(&self) -> Value {
        serde_yaml::from_slice(&fs::read(&self.credentials).unwrap()).unwrap()
    }
}

fn write_private(path: &Path, bytes: &[u8]) {
    use std::io::Write;
    fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
}

fn grant(token: &str, issuer: &str) -> Value {
    json!({"kind": "grant", "payload": {"version": 1, "token": token, "issuer": issuer}})
}

fn document() -> Value {
    json!({
        "version": 1,
        "refs": {"OTHER": "fixture-api-key"},
        "records": {
            (GRANT): grant(OLD_TOKEN, ISSUER),
            (DEVICE): {"kind": "grant", "payload": {"id": DEVICE_ID}, "unknown": true},
            "unrelated/default": {"kind": "grant", "payload": {"unknown": ["retained"]}},
        },
        "extension": {"retained": true},
    })
}

fn envelope(data: Value) -> Value {
    json!({"code": 0, "data": {"biz_code": 0, "biz_data": data}})
}

fn start_request() -> AuthStartRequest {
    AuthStartRequest {
        method: AuthLoginMethod::Browser,
        github_domain: None,
        region: None,
        target_account_id: None,
    }
}

async fn init_server() -> (Url, tokio::task::JoinHandle<String>) {
    oauth_server(
        "200 OK",
        "",
        envelope(json!({
            "authorize_url": "https://platform.deepseek.com/dsh/authorize",
            "authorize_id": "fixture-attempt",
            "expires_in": 90,
        })),
    )
    .await
}

fn route(
    path: &'static str,
    mut target: Url,
) -> impl Fn(&mut Request) + Clone + Send + Sync + 'static {
    target.set_path(path);
    move |request| {
        assert_eq!(request.url().as_str(), format!("{ISSUER}{path}"));
        *request.url_mut() = target.clone();
    }
}

fn wire_body(request: &str) -> Value {
    serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
}

async fn callback(init: tokio::task::JoinHandle<String>) {
    let begin = wire_body(&init.await.unwrap());
    let mut url = Url::parse(begin["redirect_uri"].as_str().unwrap()).unwrap();
    url.query_pairs_mut()
        .append_pair("state", begin["state"].as_str().unwrap())
        .append_pair("code", "fixture-code");
    let response = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn auth6_dsh_account_commits_expected_scoped_result() {
    let fixture = Fixture::new();
    let account = fixture.account();
    let absent =
        account.status_with("zh-CN", 28800, |_| panic!("missing grant made a request")).await;
    assert_eq!(absent, Ok(None), "an absent native grant is signed out");
    assert!(!fixture.credentials.exists());

    let mut expected = document();
    fixture.write(&expected);
    for profile in ["web", "desktop"] {
        let directory = fixture.home.join(".dsh/profiles").join(profile);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("cordis.patch.yml"), format!("# {profile} profile\n")).unwrap();
    }

    let flows = AuthFlows::default();
    let (init_url, init) = init_server().await;
    let login = account
        .start_login_with(
            &flows,
            start_request(),
            true,
            "zh-CN",
            28800,
            route("/auth-api/v0/dsh/auth_init", init_url),
        )
        .await
        .unwrap();
    callback(init).await;
    expected["refs"]["EXTERNAL"] = json!("fixture-external-ref");
    fixture.write(&expected);

    let (exchange_url, exchange) =
        oauth_server("200 OK", "", envelope(json!({"token": NEW_TOKEN}))).await;
    let approved = login
        .poll_with(&flows, route("/auth-api/v0/dsh/auth_exchange", exchange_url))
        .await
        .unwrap();
    let AuthFlow::Approved { account: approved, .. } = approved else {
        panic!("the shared grant must be committed before approval: {approved:?}");
    };
    assert_eq!(wire_body(&exchange.await.unwrap())["device_id"], DEVICE_ID);
    expected["records"][GRANT] = grant(NEW_TOKEN, ISSUER);
    assert_eq!(fixture.read(), expected);
    assert_eq!(fs::metadata(&fixture.credentials).unwrap().permissions().mode() & 0o777, 0o600);

    let before = fs::read(&fixture.credentials).unwrap();
    let (status_url, status) = oauth_server(
        "200 OK",
        "",
        envelope(json!({"id": "user-1", "id_profile": {"name": NEW_TOKEN}})),
    )
    .await;
    let read = account
        .status_with("zh-CN", 28800, route("/auth-api/v0/users/current", status_url))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(read.account_id, approved.account_id);
    assert_eq!(read.credential_state, AuthCredentialState::Ready);
    assert!(!serde_json::to_string(&read).unwrap().contains(NEW_TOKEN));
    let wire = status.await.unwrap();
    assert!(wire.starts_with("GET /auth-api/v0/users/current HTTP/1.1\r\n"));
    assert!(wire.contains(&format!("x-dsh-auth-token: {NEW_TOKEN}\r\n")));
    assert!(!wire.contains("authorization:"));
    assert!(wire.contains("x-client-platform: web\r\n"));
    assert!(wire.contains("x-client-locale: zh_CN\r\n"));
    assert!(wire.contains("x-client-timezone-offset: 28800\r\n"));
    assert_eq!(fs::read(&fixture.credentials).unwrap(), before);

    account.sign_out().unwrap();
    expected["records"].as_object_mut().unwrap().remove(GRANT);
    assert_eq!(fixture.read(), expected);
    let signed_out = fs::read(&fixture.credentials).unwrap();
    account.sign_out().unwrap();
    assert_eq!(fs::read(&fixture.credentials).unwrap(), signed_out);
    assert_eq!(
        account.status_with("zh-CN", 28800, |_| panic!("signed-out request")).await,
        Ok(None)
    );
    for profile in ["web", "desktop"] {
        let path = fixture.home.join(".dsh/profiles").join(profile).join("cordis.patch.yml");
        assert_eq!(fs::read_to_string(path).unwrap(), format!("# {profile} profile\n"));
    }

    let first = Fixture::new();
    let account = first.account();
    let flows = AuthFlows::default();
    let (init_url, init) = init_server().await;
    let login = account
        .start_login_with(
            &flows,
            start_request(),
            true,
            "en",
            0,
            route("/auth-api/v0/dsh/auth_init", init_url),
        )
        .await
        .unwrap();
    assert!(!first.credentials.exists());
    callback(init).await;
    let (exchange_url, exchange) =
        oauth_server("200 OK", "", envelope(json!({"token": NEW_TOKEN}))).await;
    assert!(matches!(
        login
            .poll_with(&flows, route("/auth-api/v0/dsh/auth_exchange", exchange_url))
            .await
            .unwrap(),
        AuthFlow::Approved { .. }
    ));
    let stored = first.read();
    let device = stored["records"][DEVICE]["payload"]["id"].as_str().unwrap();
    assert_eq!(Uuid::parse_str(device).unwrap().get_version_num(), 4);
    assert_eq!(wire_body(&exchange.await.unwrap())["device_id"], device);
    assert_eq!(stored["records"][GRANT], grant(NEW_TOKEN, ISSUER));
    assert!(!first.home.join(".dsh/profiles").exists());
}

#[tokio::test]
async fn auth6_dsh_account_rejects_invalid_or_stale_input() {
    let fixture = Fixture::new();
    let wrong_host = AuthAccountScope { host: "other-host", ..fixture.scope() };
    assert_eq!(
        DshAccount::new(&fixture.home, &fixture.transactions, wrong_host).err(),
        Some(AuthErrorCode::Forbidden)
    );
    let account = fixture.account();
    let mut expected = document();
    expected["records"][GRANT] = grant(OLD_TOKEN, "https://untrusted.example");
    fixture.write(&expected);
    let before = fs::read(&fixture.credentials).unwrap();
    assert_eq!(
        account.status_with("zh-CN", 28800, |_| panic!("unsupported issuer was contacted")).await,
        Err(AuthErrorCode::ReauthRequired),
        "an unsupported issuer requires reauthorization"
    );
    assert_eq!(fs::read(&fixture.credentials).unwrap(), before);
    account.sign_out().unwrap();
    assert!(fixture.read()["records"].get(GRANT).is_none());

    fixture.write(&document());
    let before = fs::read(&fixture.credentials).unwrap();
    let (expired_url, expired) = oauth_server("200 OK", "", json!({"code": 40003})).await;
    let read = account
        .status_with("zh-CN", 28800, route("/auth-api/v0/users/current", expired_url))
        .await
        .unwrap();
    assert_eq!(read.unwrap().credential_state, AuthCredentialState::ReauthRequired);
    assert!(expired.await.unwrap().contains("x-dsh-auth-token: fixture-previous-grant"));
    assert_eq!(fs::read(&fixture.credentials).unwrap(), before);

    let sink = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let location = format!("Location: http://{}/capture\r\n", sink.local_addr().unwrap());
    let (redirect_url, redirected) =
        oauth_server("307 Temporary Redirect", &location, json!({})).await;
    assert_eq!(
        account.status_with("en", 0, route("/auth-api/v0/users/current", redirect_url)).await,
        Err(AuthErrorCode::UpstreamFailure)
    );
    redirected.await.unwrap();
    assert!(timeout(Duration::from_millis(100), sink.accept()).await.is_err());
    assert_eq!(fs::read(&fixture.credentials).unwrap(), before);

    let flows = AuthFlows::default();
    let (init_url, init) = init_server().await;
    let login = account
        .start_login_with(
            &flows,
            start_request(),
            true,
            "en",
            0,
            route("/auth-api/v0/dsh/auth_init", init_url),
        )
        .await
        .unwrap();
    callback(init).await;
    let (exchange_url, exchange) =
        oauth_server("200 OK", "", envelope(json!({"token": "invalid token"}))).await;
    let failed = login
        .poll_with(&flows, route("/auth-api/v0/dsh/auth_exchange", exchange_url))
        .await
        .unwrap();
    exchange.await.unwrap();
    assert!(
        matches!(failed, AuthFlow::Failed { error, .. } if error.code == AuthErrorCode::UpstreamFailure)
    );
    assert_eq!(fs::read(&fixture.credentials).unwrap(), before);

    let (init_url, init) = init_server().await;
    let login = account
        .start_login_with(
            &flows,
            start_request(),
            true,
            "en",
            0,
            route("/auth-api/v0/dsh/auth_init", init_url),
        )
        .await
        .unwrap();
    callback(init).await;
    expected["records"][GRANT] = grant("fixture-newer-native-grant", ISSUER);
    fixture.write(&expected);
    let newer = fs::read(&fixture.credentials).unwrap();
    let (exchange_url, exchange) =
        oauth_server("200 OK", "", envelope(json!({"token": NEW_TOKEN}))).await;
    let failed = login
        .poll_with(&flows, route("/auth-api/v0/dsh/auth_exchange", exchange_url))
        .await
        .unwrap();
    exchange.await.unwrap();
    assert!(
        matches!(failed, AuthFlow::Failed { error, .. } if error.code == AuthErrorCode::Conflict)
    );
    assert_eq!(fs::read(&fixture.credentials).unwrap(), newer);

    let lock = fixture.credentials.with_file_name(".credentials.yaml.lock");
    write_private(&lock, b"fixture-native-lock");
    assert_eq!(account.sign_out(), Err(AuthErrorCode::Conflict));
    assert_eq!(fs::read(&fixture.credentials).unwrap(), newer);
    assert_eq!(fs::read(&lock).unwrap(), b"fixture-native-lock");
    fs::remove_file(lock).unwrap();

    write_private(&fixture.credentials, b"version: 1\nrecords: [invalid]\n");
    let invalid = fs::read(&fixture.credentials).unwrap();
    assert_eq!(account.sign_out(), Err(AuthErrorCode::InvalidInput));
    assert_eq!(fs::read(&fixture.credentials).unwrap(), invalid);

    let external = fixture.home.join("outside.yaml");
    fs::rename(&fixture.credentials, &external).unwrap();
    symlink(&external, &fixture.credentials).unwrap();
    assert_eq!(account.sign_out(), Err(AuthErrorCode::InvalidInput));
    assert_eq!(fs::read(&external).unwrap(), invalid);
    assert!(fixture.credentials.is_symlink());
}

#[test]
fn auth6_remove_dsh_clears_only_the_authorized_shared_grant() {
    let fixture = Fixture::new();
    let mut expected = document();
    fixture.write(&expected);
    let scope = fixture.scope();
    let account = fixture.account();
    let id = super::dsh_login::account(scope, AuthCredentialState::Ready, None).unwrap().account_id;

    assert_eq!(account.remove_account(scope, "different"), Err(AuthErrorCode::NotFound));
    assert_eq!(
        account.remove_account(AuthAccountScope { principal: "foreign", ..scope }, &id),
        Err(AuthErrorCode::Forbidden)
    );
    assert_eq!(fixture.read(), expected);
    assert_eq!(account.remove_account(scope, &id), Ok(()));
    expected["records"].as_object_mut().unwrap().remove(GRANT);
    assert_eq!(fixture.read(), expected);
    assert_eq!(account.remove_account(scope, &id), Err(AuthErrorCode::NotFound));
}
