use super::listener::{ListenerError, RoutingListener};
use std::{io::ErrorKind, net::Ipv4Addr, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Barrier,
    time::timeout,
};
use vw_agent_apps::routing::lifecycle_state::LifecycleState;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cc_route_004_listener_has_one_owner_and_rejects_connections() {
    let listener = Arc::new(RoutingListener::default());
    let start_together = Arc::new(Barrier::new(3));
    let contenders: Vec<_> = (0..2)
        .map(|_| {
            let listener = Arc::clone(&listener);
            let start_together = Arc::clone(&start_together);
            tokio::spawn(async move {
                start_together.wait().await;
                listener.start(0)
            })
        })
        .collect();

    start_together.wait().await;
    let mut bound = None;
    for contender in contenders {
        match contender.await.unwrap() {
            Ok(address) => {
                assert_eq!(*bound.get_or_insert(address), address);
            }
            Err(ListenerError::TransitionInProgress) => {}
            Err(error) => panic!("unexpected start failure: {error}"),
        }
    }
    let address = bound.expect("one start must acquire the listener");
    assert_eq!(listener.state(), LifecycleState::Running);
    assert_eq!(listener.start(0).unwrap(), address);

    let mut connection = TcpStream::connect(address).await.unwrap();
    connection.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").await.unwrap();
    let mut response = Vec::new();
    timeout(Duration::from_secs(5), connection.read_to_end(&mut response)).await.unwrap().unwrap();
    assert!(response.starts_with(b"HTTP/1.1 401 Unauthorized\r\n"));

    listener.stop().await.unwrap();
    assert_eq!(listener.state(), LifecycleState::Stopped);
    assert_eq!(TcpStream::connect(address).await.unwrap_err().kind(), ErrorKind::ConnectionRefused);
    listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_004_listener_can_restart_after_a_real_bind_failure() {
    let occupied = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = occupied.local_addr().unwrap();
    let listener = RoutingListener::default();
    assert_eq!(listener.state(), LifecycleState::Stopped);

    assert!(matches!(listener.start(address.port()), Err(ListenerError::AddressInUse)));
    assert_eq!(listener.state(), LifecycleState::Failed);

    drop(occupied);
    assert_eq!(listener.start(address.port()).unwrap(), address);
    assert_eq!(listener.state(), LifecycleState::Running);
    listener.stop().await.unwrap();
    assert_eq!(listener.state(), LifecycleState::Stopped);
    let rebound = std::net::TcpListener::bind(address).unwrap();
    assert_eq!(rebound.local_addr().unwrap(), address);
}

#[tokio::test]
async fn cc_route_004_listener_finishes_stop_when_the_caller_leaves() {
    use std::{
        future::Future,
        pin::pin,
        task::{Context, Waker},
    };

    let listener = RoutingListener::default();
    let address = listener.start(0).unwrap();
    {
        let mut stop = pin!(listener.stop());
        let mut context = Context::from_waker(Waker::noop());
        assert!(stop.as_mut().poll(&mut context).is_pending());
        assert_eq!(listener.state(), LifecycleState::Stopping);
        assert!(matches!(listener.start(0), Err(ListenerError::TransitionInProgress)));
    }

    timeout(Duration::from_secs(5), async {
        while listener.state() == LifecycleState::Stopping {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(listener.state(), LifecycleState::Stopped);
    assert_eq!(listener.start(address.port()).unwrap(), address);
    listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_005_listener_expected_behavior() {
    let listener = RoutingListener::default();
    assert_eq!(listener.bound_address(), None);

    let address = listener.start(0).unwrap();
    assert_eq!(address.ip(), Ipv4Addr::LOCALHOST);
    assert_ne!(address.port(), 0);
    assert_eq!(listener.bound_address(), Some(address));
    assert_eq!(listener.start_on("127.0.0.1", 0).unwrap(), address);
    timeout(Duration::from_secs(5), TcpStream::connect(address)).await.unwrap().unwrap();

    listener.stop().await.unwrap();
    assert_eq!(listener.bound_address(), None);
}

#[tokio::test]
async fn cc_route_005_listener_normalizes_ipv6_and_mapped_ipv4() {
    let listener = RoutingListener::default();
    for (host, expected) in
        [("[0:0:0:0:0:0:0:1]", "::1"), ("::1", "::1"), ("[::ffff:127.0.0.1]", "127.0.0.1")]
    {
        let address = listener.start_on(host, 0).unwrap();
        assert_eq!(address.ip().to_string(), expected);
        assert_ne!(address.port(), 0);
        assert_eq!(listener.bound_address(), Some(address));
        timeout(Duration::from_secs(5), TcpStream::connect(address)).await.unwrap().unwrap();
        listener.stop().await.unwrap();
    }
}

#[tokio::test]
async fn cc_route_005_listener_rejects_non_loopback_and_invalid_addresses() {
    let listener = RoutingListener::default();
    for host in ["0.0.0.0", "::", "192.0.2.1", "::ffff:192.0.2.1", "2001:db8::1"] {
        assert!(matches!(listener.start_on(host, 0), Err(ListenerError::NonLoopbackDenied)));
    }
    for host in ["", "localhost", "http://127.0.0.1", "127.0.0.1:1234", "[::1]:1234"] {
        assert!(matches!(listener.start_on(host, 0), Err(ListenerError::InvalidAddress)));
    }
    assert_eq!(listener.state(), LifecycleState::Stopped);
    assert_eq!(listener.bound_address(), None);

    let address = listener.start(0).unwrap();
    assert!(matches!(listener.start_on("0.0.0.0", 0), Err(ListenerError::NonLoopbackDenied)));
    assert_eq!(listener.bound_address(), Some(address));
    listener.stop().await.unwrap();
}

#[tokio::test]
async fn cc_route_005_listener_does_not_publish_an_occupied_address() {
    let occupied = std::net::TcpListener::bind("[::1]:0").unwrap();
    let address = occupied.local_addr().unwrap();
    let listener = RoutingListener::default();

    assert!(matches!(listener.start_on("::1", address.port()), Err(ListenerError::AddressInUse)));
    assert_eq!(listener.state(), LifecycleState::Failed);
    assert_eq!(listener.bound_address(), None);

    drop(occupied);
    assert_eq!(listener.start_on("::1", address.port()).unwrap(), address);
    assert_eq!(listener.bound_address(), Some(address));
    listener.stop().await.unwrap();
}
