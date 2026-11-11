use super::{
    RoutingListener, SharedTransport,
    config_apply::RoutingConfigStore,
    outbound_config::EgressError,
    outbound_custom_tests::{
        assert_origin, complete_upstream, direct_fixture, fixture, issue, project, read_head,
    },
};
use crate::{global_database::GlobalDatabase, security::SecretStore};
use std::time::Duration;
use tokio::{io::AsyncWriteExt, time::timeout};
use vw_api_types::app_routing::{RoutingEgressMode, RoutingEgressUpdate};

#[tokio::test]
async fn cc_route_028_new_requests_use_the_committed_scoped_transport() {
    timeout(Duration::from_secs(10), async {
        let home = tempfile::tempdir().unwrap();
        let project = project();
        let host = "egress-fixture";
        let db = GlobalDatabase::for_home(home.path()).unwrap();
        let store =
            RoutingConfigStore::new(db.clone(), project.tenant_scope().clone(), host.into())
                .unwrap();
        let secrets = SecretStore::new(home.path(), true);
        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let target = "http://proxy-target.invalid/fixture";

        let proxy = fixture().await;
        let proxy_addr = proxy.local_addr().unwrap();
        let proxy_url = format!("http://fixture:test@{proxy_addr}");
        let upstream = fixture().await;
        let loopback_target = format!("http://{}/fixture", upstream.local_addr().unwrap());
        let direct = direct_fixture(upstream, 2);
        let tunnel = tokio::spawn(async move {
            for auth in [
                "Basic Zml4dHVyZTp0ZXN0",
                "Basic Zml4dHVyZTpvdGhlcg==",
                "Basic Zml4dHVyZTp0ZXN0",
                "Basic Zml4dHVyZTp0ZXN0",
            ] {
                let (mut socket, _) = proxy.accept().await.unwrap();
                let head = read_head(&mut socket).await;
                assert!(head.starts_with("CONNECT proxy-target.invalid:80 HTTP/1.1\r\n"));
                assert!(head.lines().any(|line| {
                    line.split_once(": ").is_some_and(|(name, value)| {
                        name.eq_ignore_ascii_case("proxy-authorization") && value == auth
                    })
                }));
                assert!(!head.to_ascii_lowercase().contains("bearer provider-fixture"));
                socket.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await.unwrap();
                let head = read_head(&mut socket).await;
                complete_upstream(&mut socket, &head, "configured-proxy").await;
            }
        });
        // Existing authorized routes keep their provider and credential while egress changes.
        let default = || {
            SharedTransport::with_system_proxy(super::outbound_system::SystemProxy::from_settings(
                None,
                |_| None,
            ))
            .unwrap()
        };
        let token = issue(&listener, &project, host, target, default());
        let other_host = issue(
            &listener,
            &project,
            "other-mac",
            target,
            SharedTransport::with_custom_proxy(&format!("http://fixture:other@{proxy_addr}"))
                .unwrap(),
        );
        let update =
            RoutingEgressUpdate { mode: RoutingEgressMode::Custom, proxy_url: Some(proxy_url) };
        store.apply_egress(&listener, &update, &secrets).unwrap();
        assert_origin(address, &token, "configured-proxy").await;
        assert_origin(address, &other_host, "configured-proxy").await;

        let self_proxy = RoutingEgressUpdate {
            mode: RoutingEgressMode::Custom,
            proxy_url: Some(format!("http://{address}")),
        };
        assert!(matches!(
            store.apply_egress(&listener, &self_proxy, &secrets),
            Err(EgressError::NotApplied)
        ));
        assert_origin(address, &token, "configured-proxy").await;

        let connection = db.connect_sqlite().unwrap();
        connection
            .execute_batch(
                "CREATE TRIGGER reject_egress BEFORE UPDATE ON gateway_settings
             BEGIN SELECT RAISE(ABORT, 'fixture write failure'); END;",
            )
            .unwrap();
        let direct_mode = RoutingEgressUpdate { mode: RoutingEgressMode::Direct, proxy_url: None };
        assert!(matches!(
            store.apply_egress(&listener, &direct_mode, &secrets),
            Err(EgressError::NotApplied)
        ));
        assert_origin(address, &token, "configured-proxy").await;
        connection.execute_batch("DROP TRIGGER reject_egress;").unwrap();

        let loopback = issue(&listener, &project, host, &loopback_target, default());
        assert_origin(address, &loopback, "direct").await;
        store.apply_egress(&listener, &direct_mode, &secrets).unwrap();
        assert_origin(address, &loopback, "direct").await;
        tunnel.await.unwrap();
        direct.await.unwrap();
        listener.stop().await.unwrap();
    })
    .await
    .unwrap();
}
