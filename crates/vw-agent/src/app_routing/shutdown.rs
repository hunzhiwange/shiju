use std::{future::Future, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinSet,
};

pub(super) const STOP_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) async fn serve<F, Fut>(
    socket: TcpListener,
    mut stop: oneshot::Receiver<()>,
    serve_connection: F,
) where
    F: Fn(TcpStream) -> Option<Fut>,
    Fut: Future<Output = ()> + Send + 'static,
{
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            _ = &mut stop => break,
            _ = connections.join_next(), if !connections.is_empty() => {}
            accepted = socket.accept() => {
                let Ok((connection, _)) = accepted else { break };
                if let Some(task) = serve_connection(connection) {
                    connections.spawn(task);
                }
            }
        }
    }

    // Refuse new connections before canceling and joining the accepted tasks.
    drop(socket);
    connections.shutdown().await;
}
