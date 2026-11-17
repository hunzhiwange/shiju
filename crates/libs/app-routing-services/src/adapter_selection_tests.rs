use crate::app_routing::adapter_selection::{
    Adapter, ProviderType, adapter, auth_headers, gemini_auth_headers,
};
use vw_agent_apps::auth::{
    credential_file::{AccountCredential, CredentialFileStore, GeminiCredential},
    gemini_refresh::{GeminiRefresh, GeminiScope},
};
use {
    vw_api_transport::TimestampMs,
    vw_api_app_identity::AppId,
    vw_api_transport::id::TenantScope,
};

#[test]
fn cc_route_031_adapter_selection_expected_behavior() {
    use ProviderType::*;
    for (kind, expected, name) in [
        (Claude, Adapter::Claude, "x-api-key"),
        (ClaudeAuth, Adapter::Claude, "authorization"),
        (Codex, Adapter::Codex, "authorization"),
        (Gemini, Adapter::Gemini, "x-goog-api-key"),
        (GeminiCli, Adapter::Gemini, "authorization"),
        (OpenRouter, Adapter::Claude, "authorization"),
        (GitHubCopilot, Adapter::Claude, "authorization"),
        (CodexOAuth, Adapter::Claude, "authorization"),
        (XaiOAuth, Adapter::Claude, "authorization"),
    ] {
        assert_eq!(adapter(kind), expected);
        let headers = auth_headers(kind, "fixture-key").unwrap();
        assert!(headers[name].is_sensitive());
        assert_eq!(
            headers[name],
            if name == "authorization" { "Bearer fixture-key" } else { "fixture-key" }
        );
    }
}
#[test]
fn cc_route_031_crlf_returns_authentication_error() {
    for token in ["", "fixture\r\nx-injected: value", "fixture\n", "fixture\0", "测试"] {
        assert_eq!(
            auth_headers(ProviderType::GeminiCli, token).unwrap_err(),
            axum::http::StatusCode::UNAUTHORIZED
        );
    }
}
#[tokio::test]
async fn cc_route_031_gemini_reads_owner_credential_each_time() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::default();
    let scope = GeminiScope { tenant: &tenant, principal: "fixture-owner", host: "fixture-host" };
    let owner = store.for_owner(scope.principal).unwrap();
    let refresh = GeminiRefresh::new().unwrap();
    let database_root = tempfile::tempdir().unwrap();
    let database =
        agent_database::GlobalDatabase::new(database_root.path().join("fixture.sqlite"))
            .unwrap();
    let accounts =
        agent_app_services::auth_accounts::AuthAccountStore::new(database, root.path()).unwrap();
    for token in ["fixture-one", "fixture-two"] {
        let credential = AccountCredential::GeminiOauth(GeminiCredential {
            access_token: token.into(),
            refresh_token: "fixture-refresh".into(),
            client_id: "fixture-client".into(),
            client_secret: "fixture-secret".into(),
            expires_at: TimestampMs(i64::MAX),
            requires_reauth: false,
        });
        if token == "fixture-one" {
            let metadata = agent_app_services::auth_accounts::AuthAccountMetadata::new(
                "fixture-identity",
                TimestampMs(1),
            )
            .unwrap();
            accounts.import_gemini_grant(scope, "fixture-account", &metadata, &credential).unwrap();
        } else {
            owner
                .lease(&tenant, scope.host, AppId::Gemini, "fixture-account", false)
                .unwrap()
                .save(&credential)
                .unwrap();
        }
        let headers =
            gemini_auth_headers(&accounts, &refresh, scope, "fixture-account").await.unwrap();
        assert_eq!(headers["authorization"], format!("Bearer {token}"));
    }
    assert!(
        gemini_auth_headers(
            &accounts,
            &refresh,
            GeminiScope { principal: "other-owner", ..scope },
            "fixture-account"
        )
        .await
        .is_err()
    );
    assert!(
        gemini_auth_headers(
            &accounts,
            &refresh,
            GeminiScope { host: "other-host", ..scope },
            "fixture-account"
        )
        .await
        .is_err()
    );
}
#[test]
fn cc_route_031_targets_and_url_rejection() {
    use crate::app_routing::adapter_selection::target_url;
    for (kind, host) in [
        (ProviderType::Claude, "api.anthropic.com"),
        (ProviderType::ClaudeAuth, "api.anthropic.com"),
        (ProviderType::Codex, "api.openai.com"),
        (ProviderType::Gemini, "generativelanguage.googleapis.com"),
        (ProviderType::GeminiCli, "generativelanguage.googleapis.com"),
        (ProviderType::OpenRouter, "openrouter.ai"),
        (ProviderType::GitHubCopilot, "api.githubcopilot.com"),
        (ProviderType::CodexOAuth, "chatgpt.com"),
        (ProviderType::XaiOAuth, "api.x.ai"),
    ] {
        assert_eq!(target_url(kind, None, "/v1/messages").unwrap().host_str(), Some(host));
    }
    for endpoint in ["//attacker.example", "/../token", "/%2e%2e/token", "/v1/messages?key=fixture"]
    {
        assert!(target_url(ProviderType::Claude, None, endpoint).is_err());
    }
}
#[test]
fn cc_route_031_target_deduplicates_protocol_prefix() {
    let url = crate::app_routing::adapter_selection::target_url(
        ProviderType::Codex,
        Some("https://fixture.invalid/v1"),
        "/v1/chat/completions",
    )
    .unwrap();
    assert_eq!(url.path(), "/v1/chat/completions");
}

#[tokio::test]
async fn cc_route_031_listener_uses_current_managed_credential_on_wire() {
    use crate::app_routing::{
        RoutingListener, SharedTransport, adapter_selection::ManagedCredential,
        dispatch::JsonTarget,
    };
    use tokio::io::AsyncWriteExt;
    use rustls_pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
    use std::{os::unix::fs::PermissionsExt, sync::Arc, time::Duration};
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let database_root = tempfile::tempdir().unwrap();
    let database =
        agent_database::GlobalDatabase::new(database_root.path().join("fixture.sqlite"))
            .unwrap();
    let accounts = Arc::new(
        agent_app_services::auth_accounts::AuthAccountStore::new(database, root.path()).unwrap(),
    );
    let tenant = TenantScope::default();
    let scope = GeminiScope { tenant: &tenant, principal: "fixture-owner", host: "fixture-host" };
    let credential = |token: &str| {
        AccountCredential::GeminiOauth(GeminiCredential {
            access_token: token.into(),
            refresh_token: "fixture-refresh".into(),
            client_id: "fixture-client".into(),
            client_secret: "fixture-secret".into(),
            expires_at: TimestampMs(i64::MAX),
            requires_reauth: false,
        })
    };
    let metadata = agent_app_services::auth_accounts::AuthAccountMetadata::new(
        "fixture-identity",
        TimestampMs(1),
    )
    .unwrap();
    accounts
        .import_gemini_grant(scope, "fixture-account", &metadata, &credential("fixture-one"))
        .unwrap();
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base: reqwest::Url = "https://generativelanguage.googleapis.com".parse().unwrap();
    let endpoint = "/v1beta/models/fixture:generateContent";
    let target = base.join(endpoint).unwrap();
    let client = model_transport::upstream::upstream_http_client_builder()
        .no_proxy()
        .tls_certs_only([reqwest::Certificate::from_pem(GEMINI_FIXTURE_CERT).unwrap()])
        .resolve("generativelanguage.googleapis.com", upstream.local_addr().unwrap())
        .build()
        .unwrap();
    let tls = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions().unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from_pem_slice(GEMINI_FIXTURE_CERT).unwrap()],
            PrivateKeyDer::from_pem_slice(GEMINI_FIXTURE_KEY).unwrap(),
        ).unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls));
    let (sent, mut received) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        for _ in 0..2 {
            let (tcp, _) = upstream.accept().await.unwrap();
            let mut tls = acceptor.accept(tcp).await.unwrap();
            let head = crate::outbound_custom_tests::read_head(&mut tls).await;
            sent.send(head).unwrap();
            tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 16\r\nConnection: close\r\n\r\n{\"id\":\"fixture\"}").await.unwrap();
        }
    });
    let listener = crate::test_listener();
    let address = listener.start(0).unwrap();
    let binding = super::VerifiedRouteBinding::new(
        vw_api_transport::ProjectScope::new(tenant.clone(), "fixture-project".into()).unwrap(),
        "fixture-host".into(),
        AppId::Gemini,
        "fixture-account".into(),
        1,
    );
    let active = listener
        .auth
        .issue_with_target(
            binding,
            Some(JsonTarget {
                url: target,
                authorization: "Bearer obsolete-fixture".parse().unwrap(),
                transport: SharedTransport::with_fixture_client(client),
                request_options: Default::default(),
                managed: Some(Arc::new(ManagedCredential {
                    owner: accounts.clone(),
                    account_adapter: crate::app_routing::adapter_selection::AccountAdapter::new(
                        ProviderType::GeminiCli,
                    )
                    .unwrap(),
                    tenant: tenant.clone(),
                    principal: scope.principal.into(),
                    host: scope.host.into(),
                    account: "fixture-account".into(),
                    endpoint: endpoint.into(),
                    base: Some(base),
                })),
            }),
        )
        .unwrap();
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    for token in ["fixture-one", "fixture-two"] {
        if token == "fixture-two" {
            let store =
                CredentialFileStore::new(root.path()).unwrap().for_owner(scope.principal).unwrap();
            store
                .lease(&tenant, scope.host, AppId::Gemini, "fixture-account", false)
                .unwrap()
                .save(&credential(token))
                .unwrap();
        }
        let response = client
            .post(format!("http://{address}{endpoint}"))
            .bearer_auth(&active.token)
            .json(&serde_json::json!({"contents":[{"role":"user","parts":[{"text":"fixture"}]}]}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let headers =
            tokio::time::timeout(Duration::from_secs(5), received.recv()).await.unwrap().unwrap();
        let header = |name: &str| headers.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name).then(|| value.trim())
        });
        assert_eq!(header("authorization"), Some(format!("Bearer {token}").as_str()));
        assert_eq!(header("x-goog-api-client"), Some("GeminiCLI/1.0"));
        assert!(!headers.contains("obsolete-fixture"));
    }
    listener.stop().await.unwrap();
    server.await.unwrap();
}

// Public test-only certificate and key for the isolated Gemini TLS server.
const GEMINI_FIXTURE_CERT: &[u8] = br#"-----BEGIN CERTIFICATE-----
MIIDRTCCAi2gAwIBAgIULC3DRzyUzc41Q9d8xW99g6M0eawwDQYJKoZIhvcNAQEL
BQAwLDEqMCgGA1UEAwwhZ2VuZXJhdGl2ZWxhbmd1YWdlLmdvb2dsZWFwaXMuY29t
MCAXDTI2MTExNzIwMzA0MloYDzIxMjYxMDI0MjAzMDQyWjAsMSowKAYDVQQDDCFn
ZW5lcmF0aXZlbGFuZ3VhZ2UuZ29vZ2xlYXBpcy5jb20wggEiMA0GCSqGSIb3DQEB
AQUAA4IBDwAwggEKAoIBAQCW9mOsIIcmZZ8C0I0DXUaxd8RUdGawQeVXay6WQKnJ
cqv3rVwXjggNuk8mc8+YJr6hijC7k93R+bKuN0j0lYQzScNXWPNJ1dWP83D6l+NX
MGfzbRVyQSY6SVL3hZ9Fo49ievLhXn8lGHoPZM2w5t30Caxaj5UxOXfnyBXo7g8N
X5CC4QI/N9rl2fEkVNKclquiW8fkplyyIqLUdc+j+uI+A59DoPK9n75/xkKMxhwp
HZo4pYOmozwC4f6Qb4L2lUnRc4j11l2g0a5tMBbeeawMpkIeSXQ5wSUXQY863Mdf
3a1XQlAQ3929lKOd/qjUQrHPKIqDEg8sdi4LYEnfTSSTAgMBAAGjXTBbMB0GA1Ud
DgQWBBQjyytmWJMcPvHZApee2ehBKcwdITAsBgNVHREEJTAjgiFnZW5lcmF0aXZl
bGFuZ3VhZ2UuZ29vZ2xlYXBpcy5jb20wDAYDVR0TAQH/BAIwADANBgkqhkiG9w0B
AQsFAAOCAQEAeFzh9/f6EAMfheSho/cTkGeJGlUQp6LFGIvgkh1cb0zth33cm1pO
eh/PkIiaI/jBRsDBYckmsftRj5aChst7+2h8VjmQmI6kaAKZTO4gOTzhP7auQw0t
bwggoEzs64VYwAjy2MUpj0cQXAQIe8ycT98dK0A4RFQ2Le+LYhm1cIwgxq0LmHA3
9eqErJHx1Pf6x1UJWAtJIJJoApqPg7P4I4/NWbGXD4XSHHQGe7Hy4uvv0mA6yd0x
Ikc36uHwHNcpPHKs7/KsrGN2OxB62/J90GrVvPeAuOmxeu4MHAlrF7mjRlzrpySt
xkUmV4qA3MUxKavxdoZUcbVgtnKKf2bQAQ==
-----END CERTIFICATE-----
"#;
const GEMINI_FIXTURE_KEY: &[u8] = br#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCW9mOsIIcmZZ8C
0I0DXUaxd8RUdGawQeVXay6WQKnJcqv3rVwXjggNuk8mc8+YJr6hijC7k93R+bKu
N0j0lYQzScNXWPNJ1dWP83D6l+NXMGfzbRVyQSY6SVL3hZ9Fo49ievLhXn8lGHoP
ZM2w5t30Caxaj5UxOXfnyBXo7g8NX5CC4QI/N9rl2fEkVNKclquiW8fkplyyIqLU
dc+j+uI+A59DoPK9n75/xkKMxhwpHZo4pYOmozwC4f6Qb4L2lUnRc4j11l2g0a5t
MBbeeawMpkIeSXQ5wSUXQY863Mdf3a1XQlAQ3929lKOd/qjUQrHPKIqDEg8sdi4L
YEnfTSSTAgMBAAECggEAJ+gNNCjLIVLWxIs147BKKkrDSyIULT7WLhRG1fBVEcwu
+oPI5u8kYOCh4MPXmlyPoythM4zkpAeyfMxfjExkDJzQtyvr8FZHJM0RTiVNszQ4
Hoj1FOV1Ag51ELqxflq84icnQn1U9gzw41Xd7AgznznZZn0zTrZm5I4ngJJBx2JW
Nx4IRaqgtKcawdcg84mgcIVYqX89obT8GUKs6vLYp/6w/iERWbiPf5ImxNkkMseF
Sjjs6sNPYa9YszMa9Y/uh4r8b0VnZeUFT5vLVGUiPESKAhRVu7oQgvEXZYm545sR
TIJXt978FDIidADz8QRB/Br7j4InEaY9IQD+XBbUgQKBgQDQgWbUwgJrJU9ulGWL
Tjp5jva9Vhgkl2340A46T4owdas/lOc8ZxTAG+l+aP4FJLqL9+BJPllDAt9BAbm9
CI2sdTCbZ7ZcquNOqu29lPUGmHUkOKeZBhkBVA5No6CkMKwbvtMZqom/2LuS85BF
Nv++9bvEnRBWu9AfOkjQgrKEgQKBgQC5WXoCZ44MGdM2h3vrYJ2c19NJtw19T+K0
OW1Ogqhz+UfcZMY00MNKOPSxLmKf5Tv6Pd75f2diN9G0miDDD4XbzMsrhm2/ym33
5gCqv6itcTINEIRIiLyrId+Xs08W92l4ezBqUJMZ6K/Mfbm6r9NY3NqEgQixpy2B
RD7EemnPEwKBgQDHHeyP3b7lh7n1hi36/PgGwiqL4b4oo5xX8ojD0tXLNXQWwiKZ
om9EjuNK3wVG8C5eIJfGGo6v2fxOXlF8XsC9IyUKI98ZW/C7XaywUaI1bD7iGmCj
WBllFVqbdlmgswKJdJCwgBYQae+nhIYQM1fEi3OHbd7+lVlJmSWUSlwlAQKBgQCq
oyyHcCy/CTuBNSQpo24XSj8z2zmpB1WwFDqEkBdjhbjZt1EeqJ9SArOLNygHCrS4
AiP4UImuS7NdcJP+4JicraLSNFVvv36UPgSZLg+tdcvdaTpPyMrhRjMh5ytF+or4
J3mANyPmM2k9evX1YA/T/KeUW3LUkxN6sYiTvBsTTwKBgB3M0x4GuoqM1pQuLXus
8Ml8PWZbQAgVmMLeGybWbaWsYudtOcAz6xJZCvDjcZE11sC71plBo/Iic5tW+Qrn
uEgnZuwUEt7Wxxuu9X1ZoEOz3zeRBJoM6EIBFfzbVD4Tj3YN0axvNbnC8agXfQWt
WDUko6iXEPPJ+CqA4SLZ/ItG
-----END PRIVATE KEY-----
"#;
