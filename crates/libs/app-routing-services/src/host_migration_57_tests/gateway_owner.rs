use std::{net::Ipv4Addr, time::Duration};
const TIMEOUT: Duration = Duration::from_secs(20);
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cc_route_007_gateway_owner_waits_for_connections_before_returning() {
    use crate::gateway_owner::run_with_routing;
    use std::sync::{Mutex, mpsc};
    use tokio::sync::oneshot;

    struct ConnectionCleanup {
        started: Option<oneshot::Sender<()>>,
        release: mpsc::Receiver<()>,
    }

    impl Drop for ConnectionCleanup {
        fn drop(&mut self) {
            let _ = self.started.take().unwrap().send(());
            let _ = self.release.recv_timeout(TIMEOUT);
        }
    }

    let (ready, bound) = oneshot::channel();
    let (accepted, connected) = oneshot::channel();
    let (stop, shutdown) = oneshot::channel();
    let (cleanup_start, cleanup_started) = oneshot::channel();
    let (release_cleanup, cleanup_release) = mpsc::channel();
    let mut owner = tokio::spawn(run_with_routing(
        crate::test_database(),
        async |routing| {
            let connection = Mutex::new(Some((accepted, cleanup_start, cleanup_release)));
            let address = routing.start_at((Ipv4Addr::LOCALHOST, 0).into(), move |stream| {
                let (accepted, started, release) = connection.lock().unwrap().take()?;
                Some(async move {
                    let _stream = stream;
                    let _cleanup = ConnectionCleanup { started: Some(started), release };
                    let _ = accepted.send(());
                    std::future::pending::<()>().await;
                })
            })?;
            ready.send(address).unwrap();
            std::future::pending().await
        },
        async {
            shutdown.await.unwrap();
            Ok(())
        },
    ));
    let client = tokio::time::timeout(TIMEOUT, async {
        let address = bound.await.unwrap();
        let client = tokio::net::TcpStream::connect(address).await.unwrap();
        connected.await.unwrap();
        client
    })
    .await
    .unwrap();
    stop.send(()).unwrap();
    tokio::time::timeout(TIMEOUT, cleanup_started).await.unwrap().unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut owner).await.is_err(),
        "gateway returned while a connection was still being cleaned up"
    );
    release_cleanup.send(()).unwrap();
    tokio::time::timeout(TIMEOUT, owner).await.unwrap().unwrap().unwrap();
    assert!(tokio::net::TcpStream::connect(client.peer_addr().unwrap()).await.is_err());
}
