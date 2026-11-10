//! Observe the client socket after the authorized request body is consumed.

use parking_lot::Mutex;
use std::{
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, ReadBuf, copy, sink},
    net::tcp::OwnedReadHalf,
    sync::Notify,
};

/// Hyper may stop watching for EOF while it has pipelined bytes buffered.
#[derive(Clone, Default)]
pub(super) struct ClientDisconnect(Arc<Notify>);

impl ClientDisconnect {
    pub(super) fn request_body_read(&self) {
        self.0.notify_one();
    }

    pub(super) async fn wait(&self, mut reader: ClientReader<'_>) {
        self.0.notified().await;
        // The listener accepts one request per connection. Drain unread
        // pipeline bytes without dispatching them and return on EOF or reset.
        let _ = copy(&mut reader, &mut sink()).await;
    }
}

/// The HTTP driver and the disconnect watcher poll the same socket in the
/// same task. The watcher starts only once the request body has been read.
pub(super) struct ClientReader<'a>(pub(super) &'a Mutex<OwnedReadHalf>);

impl AsyncRead for ClientReader<'_> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut *self.0.lock()).poll_read(cx, buf)
    }
}
