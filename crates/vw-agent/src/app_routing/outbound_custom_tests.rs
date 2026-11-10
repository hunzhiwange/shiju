use super::{
    RoutingListener, SharedTransport, dispatch::JsonTarget, route_auth::VerifiedRouteBinding,
};
use serde_json::{Value, json};
use std::{net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
    time::timeout,
};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

const GUARD: Duration = Duration::from_secs(5);
const PROXY_AUTH: &str = "Basic Zml4dHVyZTp0ZXN0";

pub(super) fn issue(
    listener: &RoutingListener,
    project: &ProjectScope,
    host: &str,
    url: &str,
    transport: SharedTransport,
) -> String {
    listener
        .auth
        .issue_test_token(
            VerifiedRouteBinding::new(
                project.clone(),
                host.into(),
                AppId::Codex,
                "account".into(),
                1,
            ),
            JsonTarget {
                url: url.parse().unwrap(),
                authorization: "Bearer provider-fixture".parse().unwrap(),
                transport,
                request_options: Default::default(),
            },
        )
        .unwrap()
}

pub(super) fn project() -> ProjectScope {
    ProjectScope::new(TenantScope::new("platform", "company").unwrap(), "outbound-fixture".into())
        .unwrap()
}

pub(super) async fn fixture() -> TcpListener {
    TcpListener::bind("127.0.0.1:0").await.unwrap()
}

pub(super) async fn read_head(socket: &mut (impl AsyncRead + Unpin)) -> String {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        assert!(head.len() < 4096);
        head.push(socket.read_u8().await.unwrap());
    }
    String::from_utf8(head).unwrap()
}

fn header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.lines()
        .filter_map(|line| line.split_once(": "))
        .find_map(|(key, value)| key.eq_ignore_ascii_case(name).then_some(value))
}

pub(super) async fn complete_upstream(
    socket: &mut (impl AsyncRead + AsyncWrite + Unpin),
    head: &str,
    origin: &str,
) {
    assert!(head.starts_with("POST /fixture HTTP/1.1\r\n"));
    assert_eq!(header(head, "Authorization"), Some("Bearer provider-fixture"));
    assert_eq!(header(head, "Proxy-Authorization"), None);
    let size = header(head, "Content-Length").unwrap().parse().unwrap();
    assert!(size < 1024);
    let mut body = vec![0; size];
    socket.read_exact(&mut body).await.unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), json!({"model": "fixture"}));
    let body = json!({"origin": origin}).to_string();
    socket.write_all(format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ).as_bytes()).await.unwrap();
    socket.flush().await.unwrap();
}

pub(super) fn direct_fixture(listener: TcpListener, requests: usize) -> JoinHandle<()> {
    tokio::spawn(async move {
        for _ in 0..requests {
            let (mut socket, _) = listener.accept().await.unwrap();
            let head = read_head(&mut socket).await;
            complete_upstream(&mut socket, &head, "direct").await;
        }
    })
}

pub(super) async fn request(address: SocketAddr, token: &str) -> (u16, String) {
    request_with_body(address, token, br#"{"model":"fixture"}"#).await
}

async fn request_with_body(address: SocketAddr, token: &str, body: &[u8]) -> (u16, String) {
    let mut socket = TcpStream::connect(address).await.unwrap();
    let head = format!(
        "POST /v1/chat/completions HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nConnection: close\r\nAcCePt: application/json\r\nACCEPT-Language: zh-CN\r\naccePT: text/plain\r\nContent-Length: {}\r\n\r\n",
        body.len(),
    );
    socket.write_all(head.as_bytes()).await.unwrap();
    socket.write_all(body).await.unwrap();
    let response = read_head(&mut socket).await;
    let status = response.split_whitespace().nth(1).unwrap().parse().unwrap();
    let mut body = String::new();
    socket.take(4096).read_to_string(&mut body).await.unwrap();
    (status, body)
}

pub(super) async fn assert_origin(address: SocketAddr, token: &str, origin: &str) {
    let (status, body) = request(address, token).await;
    assert_eq!(status, 200);
    assert!(body.contains(&json!({"origin": origin}).to_string()), "unexpected origin: {body}");
}

async fn accept_connect(socket: &mut (impl AsyncRead + AsyncWrite + Unpin), authority: &str) {
    let connect = read_head(socket).await;
    assert!(connect.starts_with(&format!("CONNECT {authority} HTTP/1.1\r\n")));
    assert_eq!(header(&connect, "Proxy-Authorization"), Some(PROXY_AUTH));
    assert_eq!(header(&connect, "Authorization"), None);
    socket.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await.unwrap();
    socket.flush().await.unwrap();
}

fn assert_wire_order(head: &str) {
    let fields: Vec<_> = head
        .lines()
        .filter(|line| {
            let name = line.split(':').next().unwrap();
            name.eq_ignore_ascii_case("accept") || name.eq_ignore_ascii_case("accept-language")
        })
        .collect();
    assert_eq!(
        fields,
        ["AcCePt: application/json", "ACCEPT-Language: zh-CN", "accePT: text/plain"]
    );
}

#[tokio::test]
async fn cc_route_025_outbound_custom_expected_behavior() {
    timeout(GUARD, async {
        let upstream = fixture().await;
        let url = format!("http://{}/fixture", upstream.local_addr().unwrap());
        let proxy_target = "http://proxy-target.invalid/fixture";
        let direct = direct_fixture(upstream, 2);
        let proxy = fixture().await;
        let proxy_url = format!("http://fixture:test@{}", proxy.local_addr().unwrap());
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let project = project();
        let host_a = issue(&listener, &project, "mac-a", &url, SharedTransport::new().unwrap());
        let host_b = issue(&listener, &project, "mac-b", &url, SharedTransport::new().unwrap());
        assert_origin(address, &host_a, "direct").await;

        let tunnel = tokio::spawn(async move {
            let (mut socket, _) = proxy.accept().await.unwrap();
            accept_connect(&mut socket, "proxy-target.invalid:80").await;
            let head = read_head(&mut socket).await;
            assert_wire_order(&head);
            complete_upstream(&mut socket, &head, "proxy").await;
        });
        let updated_a = issue(
            &listener,
            &project,
            "mac-a",
            proxy_target,
            SharedTransport::with_custom_proxy(&proxy_url).unwrap(),
        );
        assert_origin(address, &updated_a, "proxy").await;
        assert_origin(address, &host_b, "direct").await;
        // Revocation is checked before the request body is read.
        assert_eq!(request_with_body(address, &host_a, b"").await.0, 401);
        direct.await.unwrap();
        tunnel.await.unwrap();
        listener.stop().await.unwrap();
    })
    .await
    .expect("custom HTTP proxy fixture timed out");
}

async fn accept_socks(socket: &mut TcpStream, port: u16, remote_dns: bool) {
    assert_eq!(socket.read_u8().await.unwrap(), 5);
    let mut methods = vec![0; socket.read_u8().await.unwrap().into()];
    socket.read_exact(&mut methods).await.unwrap();
    assert!(methods.contains(&2));
    socket.write_all(&[5, 2]).await.unwrap();
    assert_eq!(socket.read_u8().await.unwrap(), 1);
    for expected in [b"fixture".as_slice(), b"test"] {
        let mut field = vec![0; socket.read_u8().await.unwrap().into()];
        socket.read_exact(&mut field).await.unwrap();
        assert_eq!(field, expected);
    }
    socket.write_all(&[1, 0]).await.unwrap();
    let mut prefix = [0; 3];
    socket.read_exact(&mut prefix).await.unwrap();
    assert_eq!(prefix, [5, 1, 0]);
    let address_type = socket.read_u8().await.unwrap();
    if remote_dns {
        assert_eq!(address_type, 3);
        let mut name = vec![0; socket.read_u8().await.unwrap().into()];
        socket.read_exact(&mut name).await.unwrap();
        assert_eq!(name, b"proxy-target.invalid");
    } else {
        let address = match address_type {
            1 => std::net::IpAddr::from(socket.read_u32().await.unwrap().to_be_bytes()),
            4 => std::net::IpAddr::from(socket.read_u128().await.unwrap().to_be_bytes()),
            other => panic!("expected an IP from local DNS, got SOCKS address type {other}"),
        };
        assert_eq!(address, "192.0.2.1".parse::<std::net::IpAddr>().unwrap());
    }
    assert_eq!(socket.read_u16().await.unwrap(), port);
    socket.write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 0]).await.unwrap();
}

#[tokio::test]
async fn cc_route_025_socks5_auth_and_dns() {
    timeout(GUARD, async {
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let project = project();
        // The local form sends an IP. The remote form sends a name that the
        // fixture accepts without contacting DNS or an upstream service.
        let port = 8080;
        for (scheme, remote_dns) in [("socks5", false), ("socks5h", true)] {
            let proxy = fixture().await;
            let proxy_url = format!("{scheme}://fixture:test@{}", proxy.local_addr().unwrap());
            let host = if remote_dns { "proxy-target.invalid" } else { "192.0.2.1" };
            let url = format!("http://{host}:{port}/fixture");
            let token = issue(
                &listener,
                &project,
                "mac",
                &url,
                SharedTransport::with_custom_proxy(&proxy_url).unwrap(),
            );
            let socks = tokio::spawn(async move {
                let (mut socket, _) = proxy.accept().await.unwrap();
                accept_socks(&mut socket, port, remote_dns).await;
                let head = read_head(&mut socket).await;
                complete_upstream(&mut socket, &head, scheme).await;
            });
            assert_origin(address, &token, scheme).await;
            socks.await.unwrap();
        }
        listener.stop().await.unwrap();
    })
    .await
    .expect("custom SOCKS fixture timed out");
}

#[test]
fn cc_route_025_rejects_invalid_proxy_urls() {
    for url in [
        "ftp://fixture:secret@127.0.0.1:1234",
        "socks4://127.0.0.1:1080",
        "127.0.0.1:1080",
        "http://",
        "https://127.0.0.1/path",
        "https://127.0.0.1/?query",
        "https://127.0.0.1/#fragment",
        "https://127.0.0.1:0",
    ] {
        let error = SharedTransport::with_custom_proxy(url).err().expect("invalid proxy accepted");
        assert!(!format!("{error:?}").contains("secret"));
    }
}

#[tokio::test]
async fn cc_route_025_proxy_auth_failure_never_falls_back() {
    timeout(GUARD, async {
        let upstream = fixture().await;
        let url = format!("http://0.0.0.0:{}/fixture", upstream.local_addr().unwrap().port());
        let proxy = fixture().await;
        let proxy_url = format!("http://fixture:test@{}", proxy.local_addr().unwrap());
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let token = issue(
            &listener,
            &project(),
            "mac",
            &url,
            SharedTransport::with_custom_proxy(&proxy_url).unwrap(),
        );
        let auth_failure = tokio::spawn(async move {
            let (mut socket, _) = proxy.accept().await.unwrap();
            let head = read_head(&mut socket).await;
            assert_eq!(header(&head, "Proxy-Authorization"), Some(PROXY_AUTH));
            socket
                .write_all(
                    b"HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\n\r\n",
                )
                .await
                .unwrap();
        });
        let (status, body) = request(address, &token).await;
        assert_eq!(status, 502);
        assert!(!body.contains(PROXY_AUTH) && !body.contains("fixture"));
        assert!(timeout(Duration::from_millis(25), upstream.accept()).await.is_err());
        auth_failure.await.unwrap();
        listener.stop().await.unwrap();
    })
    .await
    .expect("proxy authentication failure fixture timed out");
}

fn tls_fixture() -> (tokio_rustls::TlsAcceptor, tokio_rustls::TlsConnector) {
    use rustls_pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
    use std::sync::Arc;

    let certificate = CertificateDer::from_pem_slice(PROXY_CERT).unwrap();
    let key = PrivateKeyDer::from_pem_slice(PROXY_KEY).unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let server = rustls::ServerConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![certificate], key)
        .unwrap();

    let mut roots = rustls::RootCertStore::empty();
    roots.add(CertificateDer::from_pem_slice(PROXY_CA).unwrap()).unwrap();
    let client = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
    (Arc::new(server).into(), Arc::new(client).into())
}

#[tokio::test]
async fn cc_route_025_https_proxy_connects_with_verified_tls() {
    timeout(GUARD, async {
        let proxy = fixture().await;
        let proxy_url = format!("https://fi%78ture:te%73t@{}", proxy.local_addr().unwrap());
        let (_, intercept) = super::outbound_custom::parse(&proxy_url).unwrap();
        let (acceptor, connector) = tls_fixture();
        let server = tokio::spawn(async move {
            let (tcp, _) = proxy.accept().await.unwrap();
            let mut socket = acceptor.accept(tcp).await.unwrap();
            accept_connect(&mut socket, "[::1]:80").await;
            assert_eq!(socket.read_u8().await.unwrap(), b'Q');
            socket.write_all(b"R").await.unwrap();
            socket.flush().await.unwrap();
        });
        let target = "http://[::1]/fixture".parse().unwrap();
        let mut tunnel =
            super::outbound_custom::connect_tunnel(&intercept, &target, &connector).await.unwrap();
        tunnel.write_all(b"Q").await.unwrap();
        tunnel.flush().await.unwrap();
        assert_eq!(tunnel.read_u8().await.unwrap(), b'R');
        server.await.unwrap();
    })
    .await
    .expect("trusted HTTPS proxy fixture timed out");
}

#[tokio::test]
async fn cc_route_025_https_proxy_rejects_an_untrusted_certificate() {
    timeout(GUARD, async {
        let proxy = fixture().await;
        let proxy_url = format!("https://fixture:test@{}", proxy.local_addr().unwrap());
        let upstream = fixture().await;
        let target = format!("http://0.0.0.0:{}/fixture", upstream.local_addr().unwrap().port());
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let token = issue(
            &listener,
            &project(),
            "mac",
            &target,
            SharedTransport::with_custom_proxy(&proxy_url).unwrap(),
        );
        let (acceptor, _) = tls_fixture();
        let server = tokio::spawn(async move {
            let (tcp, _) = proxy.accept().await.unwrap();
            assert!(acceptor.accept(tcp).await.is_err());
        });
        assert_eq!(request(address, &token).await.0, 502);
        server.await.unwrap();
        assert!(timeout(Duration::from_millis(25), upstream.accept()).await.is_err());
        listener.stop().await.unwrap();
    })
    .await
    .expect("untrusted HTTPS proxy fixture timed out");
}

// Loopback-only TLS material. The private key is generated for this fixture.
const PROXY_CA: &[u8] = br#"-----BEGIN CERTIFICATE-----
MIIBgDCCASagAwIBAgIUZ4RQcJ7xL+MjSgafZyvVx4KQzbEwCgYIKoZIzj0EAwIw
HjEcMBoGA1UEAwwTU2hpSnUgVGVzdCBQcm94eSBDQTAeFw0yMDAxMDEwMDAwMDBa
Fw00NTAxMDEwMDAwMDBaMB4xHDAaBgNVBAMME1NoaUp1IFRlc3QgUHJveHkgQ0Ew
WTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAAQwFlcu2gVWR/oZz4lWt4U3T85KEN+y
s3BkEbKF48D5C6I0f5G/A8izm8/64ITQ+Xujx9tg7SQihUnvWqN1MVqMo0IwQDAd
BgNVHQ4EFgQUrTvMUSslNMWuUDfknlg6+58WazgwDwYDVR0TAQH/BAUwAwEB/zAO
BgNVHQ8BAf8EBAMCAgQwCgYIKoZIzj0EAwIDSAAwRQIgacvUbnVxDXDEpFTWjYdm
n5VtukhG77104dKVO2xdbwUCIQDFSSienCcoi9ghd8JvNpo1NXpTXiw2/bBrQC0l
R6Ip7Q==
-----END CERTIFICATE-----
"#;
const PROXY_CERT: &[u8] = br#"-----BEGIN CERTIFICATE-----
MIIBcjCCARigAwIBAgIBATAKBggqhkjOPQQDAjAeMRwwGgYDVQQDDBNTaGlKdSBU
ZXN0IFByb3h5IENBMB4XDTIwMDEwMTAwMDAwMFoXDTQ1MDEwMTAwMDAwMFowFDES
MBAGA1UEAwwJbG9jYWxob3N0MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE5yPv
TWMu+Z3lKFvNIEvjTJ02e+3VRVNf2tLoSALdeDZPH9RuhvzvqCyrAuHVGowlMrt6
ROhjCCiurmy3ivE0GaNRME8wDAYDVR0TAQH/BAIwADAOBgNVHQ8BAf8EBAMCB4Aw
EwYDVR0lBAwwCgYIKwYBBQUHAwEwGgYDVR0RBBMwEYIJbG9jYWxob3N0hwR/AAAB
MAoGCCqGSM49BAMCA0gAMEUCICjy5cqOnYdFUI8yXU3IY01Y9uemtjIGyHJQ4cXw
NRaoAiEA4T3WgOzeahGbyBYAaQ6Ue3GIqqe1Cpr/DeYhRj74MhI=
-----END CERTIFICATE-----
"#;
const PROXY_KEY: &[u8] = br#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgU04FltSpN8mTy0ts
9weYVuj527KPBgDZYM6tObtsUgqhRANCAATnI+9NYy75neUoW80gS+NMnTZ77dVF
U1/a0uhIAt14Nk8f1G6G/O+oLKsC4dUajCUyu3pE6GMIKK6ubLeK8TQZ
-----END PRIVATE KEY-----
"#;
