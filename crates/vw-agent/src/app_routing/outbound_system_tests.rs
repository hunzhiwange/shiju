use super::{
    RoutingListener, SharedTransport,
    outbound_custom_tests::{
        assert_origin, complete_upstream, direct_fixture, fixture, issue, project, read_head,
        request,
    },
    outbound_system::{SystemProxy, is_loopback},
};
use std::{net::SocketAddr, time::Duration};
use system_configuration::{
    core_foundation::{
        array::CFArray,
        base::{CFType, TCFType},
        dictionary::CFDictionary,
        number::CFNumber,
        string::CFString,
    },
    dynamic_store::SCDynamicStoreBuilder,
};
use tokio::{io::AsyncWriteExt, time::timeout};

#[tokio::test]
async fn cc_route_026_outbound_system_expected_behavior() {
    timeout(Duration::from_secs(5), async {
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let upstream = fixture().await;
        let target = format!("http://{}/fixture", upstream.local_addr().unwrap());
        let direct = direct_fixture(upstream, 1);
        let proxy = fixture().await;
        let proxy_url = format!("http://{}", proxy.local_addr().unwrap());
        let token = issue(
            &listener,
            &project(),
            "mac",
            &target,
            SharedTransport::with_custom_proxy(&proxy_url).unwrap(),
        );

        let response = tokio::spawn(async move {
            let (mut socket, _) = proxy.accept().await.unwrap();
            let head = read_head(&mut socket).await;
            assert!(!head.starts_with("CONNECT "), "loopback upstream was sent to the proxy");
        });

        assert_origin(address, &token, "direct").await;
        direct.await.unwrap();
        response.abort();
        listener.stop().await.unwrap();
    })
    .await
    .expect("loopback fixture timed out");
}

fn http_settings(proxy: SocketAddr) -> CFDictionary<CFString, CFType> {
    CFDictionary::from_CFType_pairs(&[
        (CFString::new("HTTPEnable"), CFNumber::from(1).as_CFType()),
        (CFString::new("HTTPProxy"), CFString::new(&proxy.ip().to_string()).as_CFType()),
        (CFString::new("HTTPPort"), CFNumber::from(i32::from(proxy.port())).as_CFType()),
        (
            CFString::new("ExceptionsList"),
            CFArray::from_CFTypes(&[
                CFString::new("*.example.test"),
                CFString::new("192.168.0.0/16"),
                CFString::new("<local>"),
            ])
            .as_CFType(),
        ),
    ])
}

#[test]
fn cc_route_026_loopback_aliases() {
    for host in
        ["localhost", "LOCALHOST.", "api.localhost", "127.1.2.3", "[::1]", "[::ffff:127.0.0.1]"]
    {
        assert!(is_loopback(host), "{host}");
    }
    for host in ["10.0.0.1", "localhost.example.test", "[::ffff:192.0.2.1]"] {
        assert!(!is_loopback(host), "{host}");
    }
}

#[test]
fn cc_route_026_native_settings_and_exceptions() {
    let settings = http_settings("127.0.0.1:7897".parse().unwrap());
    let proxy = SystemProxy::from_settings(Some(&settings), |key| {
        (key == "NO_PROXY").then(|| ".env.test".into())
    });
    let target = |host| format!("http://{host}/fixture").parse().unwrap();
    assert_eq!(proxy.url(&target("example.test")).unwrap().as_str(), "http://127.0.0.1:7897/");
    for host in ["a.example.test", "192.168.1.2", "intranet", "a.env.test"] {
        assert!(proxy.url(&target(host)).is_none(), "{host}");
    }
    assert!(proxy.url(&"https://example.test/fixture".parse().unwrap()).is_none());

    let store = SCDynamicStoreBuilder::new("ShiJu routing test").build().unwrap();
    let actual = store.get_proxies();
    let _ = SystemProxy::from_settings(actual.as_ref(), |_| None);
    SharedTransport::new().unwrap();
}

#[test]
fn cc_route_026_socks_fallback_and_environment_priority() {
    let settings = CFDictionary::from_CFType_pairs(&[
        (CFString::new("HTTPSEnable"), CFNumber::from(1).as_CFType()),
        (CFString::new("HTTPSProxy"), CFString::new("::1").as_CFType()),
        (CFString::new("HTTPSPort"), CFNumber::from(8443).as_CFType()),
        (CFString::new("SOCKSEnable"), CFNumber::from(1).as_CFType()),
        (CFString::new("SOCKSProxy"), CFString::new("127.0.0.1").as_CFType()),
        (CFString::new("SOCKSPort"), CFNumber::from(1080).as_CFType()),
    ]);
    let http = "http://example.test".parse().unwrap();
    let https = "https://example.test".parse().unwrap();
    let proxy = SystemProxy::from_settings(Some(&settings), |_| None);
    assert_eq!(proxy.url(&http).unwrap().as_str(), "socks5h://127.0.0.1:1080");
    assert_eq!(proxy.url(&https).unwrap().as_str(), "http://[::1]:8443/");
    let proxy = SystemProxy::from_settings(Some(&settings), |key| {
        (key == "HTTPS_PROXY").then(|| "http://127.0.0.1:9876".into())
    });
    assert_eq!(proxy.url(&https).unwrap().as_str(), "http://127.0.0.1:9876/");
    let direct = SystemProxy::from_settings(None, |_| None);
    assert!(direct.url(&http).is_none());
    assert!(direct.url(&https).is_none());
}

#[tokio::test]
async fn cc_route_026_system_and_explicit_proxy_keep_ordered_wire() {
    timeout(Duration::from_secs(5), async {
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let target = "http://proxy-target.invalid/fixture";
        for explicit in [false, true] {
            let proxy = fixture().await;
            let proxy_address = proxy.local_addr().unwrap();
            let transport = if explicit {
                SharedTransport::with_custom_proxy(&format!("http://{proxy_address}")).unwrap()
            } else {
                let settings = http_settings(proxy_address);
                SharedTransport::with_system_proxy(SystemProxy::from_settings(
                    Some(&settings),
                    |_| None,
                ))
                .unwrap()
            };
            let token = issue(&listener, &project(), "mac", target, transport);
            let tunnel = tokio::spawn(async move {
                let (mut socket, _) = proxy.accept().await.unwrap();
                let connect = read_head(&mut socket).await;
                assert!(connect.starts_with("CONNECT proxy-target.invalid:80 HTTP/1.1\r\n"));
                socket.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await.unwrap();
                let head = read_head(&mut socket).await;
                assert!(head.contains("\r\nAcCePt: application/json\r\n"));
                complete_upstream(&mut socket, &head, "proxy").await;
            });
            assert_origin(address, &token, "proxy").await;
            tunnel.await.unwrap();
        }
        listener.stop().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cc_route_026_current_listener_never_receives_egress() {
    timeout(Duration::from_secs(5), async {
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let upstream = fixture().await;
        // macOS connects an unspecified IPv4 destination to the local socket.
        // Use it here to exercise a non-loopback hostname through the selector.
        let target = format!("http://0.0.0.0:{}/fixture", upstream.local_addr().unwrap().port());
        let direct = direct_fixture(upstream, 1);
        let settings = http_settings(address);
        let token = issue(
            &listener,
            &project(),
            "mac",
            &target,
            SharedTransport::with_system_proxy(SystemProxy::from_settings(Some(&settings), |_| {
                None
            }))
            .unwrap(),
        );
        assert_origin(address, &token, "direct").await;
        direct.await.unwrap();

        let token = issue(
            &listener,
            &project(),
            "mac",
            &target,
            SharedTransport::with_custom_proxy(&format!("http://{address}")).unwrap(),
        );
        assert_eq!(request(address, &token).await.0, 502);

        let token = issue(
            &listener,
            &project(),
            "mac",
            &format!("http://{address}/fixture"),
            SharedTransport::new().unwrap(),
        );
        assert_eq!(request(address, &token).await.0, 502);
        listener.stop().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cc_route_026_rejects_self_before_dialing_with_unordered_requests() {
    let socket = fixture().await;
    let address = socket.local_addr().unwrap();
    let transport = SharedTransport::with_custom_proxy(&format!("http://{address}")).unwrap();
    let external = "http://proxy-target.invalid/fixture".parse::<reqwest::Url>().unwrap();
    let local = format!("http://{address}/fixture").parse::<reqwest::Url>().unwrap();
    for url in [&external, &local] {
        let request = reqwest::Request::new(reqwest::Method::POST, url.clone());
        let response = transport
            .forward_ordered(
                url,
                request,
                super::header_wire::HeaderWire::default(),
                tokio_util::sync::CancellationToken::new(),
                address,
            )
            .await;
        assert_eq!(response.unwrap_err(), axum::http::StatusCode::BAD_GATEWAY);
    }
    assert!(timeout(Duration::from_millis(25), socket.accept()).await.is_err());
}
