use super::listener::{ListenerError, RoutingListener};
use std::{io::ErrorKind, net::Ipv4Addr, sync::Arc, time::Duration};
use tokio::{io::AsyncReadExt, net::TcpStream, sync::Barrier, time::timeout};
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
    let mut response = [0; 1];
    let read = timeout(Duration::from_secs(5), connection.read(&mut response)).await.unwrap();
    assert_eq!(read.unwrap(), 0, "uncredentialed connections must be closed");

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

    assert!(matches!(
        listener.start(address.port()),
        Err(ListenerError::Bind(error)) if error.kind() == ErrorKind::AddrInUse
    ));
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
