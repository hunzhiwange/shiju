//! ACP 消息 I/O 旁路监听。
//!
//! 该模块包装代理进程的 stdin/stdout，把通过流读写的换行分隔 JSON-RPC 消息
//! 复制给调试和输出回调，同时保持原始 `AsyncRead`/`AsyncWrite` 行为不变。

use super::*;

impl AcpClient {
    pub(super) fn make_message_tap(&self, direction: AcpMessageDirection) -> MessageTap {
        MessageTap::new(direction, self.on_acp_message.clone(), self.on_acp_output_message.clone())
    }
}

impl MessageTap {
    /// 创建一个消息监听器。
    ///
    /// `direction` 标识消息方向，两个回调用于分别消费原始 ACP 消息和输出消息。
    /// 返回值只缓存尚未遇到换行的字节，不会主动解析或阻塞 I/O。
    pub(super) fn new(
        direction: AcpMessageDirection,
        on_message: Option<AcpMessageCallback>,
        on_output_message: Option<AcpMessageCallback>,
    ) -> Self {
        Self { direction, on_message, on_output_message, buffer: Vec::new() }
    }

    fn consume(&mut self, bytes: &[u8]) {
        if self.on_message.is_none() && self.on_output_message.is_none() {
            return;
        }
        self.buffer.extend_from_slice(bytes);
        // ACP JSON-RPC 在这里以换行分隔。按行解析可以处理任意读写分片，
        // 同时避免在半包数据上误报解析错误。
        while let Some(newline_idx) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let mut line = self.buffer.drain(..=newline_idx).collect::<Vec<_>>();
            while matches!(line.last(), Some(b'\n' | b'\r')) {
                let _ = line.pop();
            }
            if line.is_empty() {
                continue;
            }
            self.dispatch_line(&line);
        }
    }

    fn dispatch_line(&self, line: &[u8]) {
        let Ok(value) = serde_json::from_slice::<Value>(line) else {
            return;
        };
        // 只转发符合 ACP JSON-RPC 形状的消息，避免代理日志或其它 stdout/stderr
        // 内容污染调用方的协议观察回调。
        if !is_acp_json_rpc_message(&value) {
            return;
        }
        if let Some(callback) = &self.on_output_message
            && let Ok(message) = serde_json::from_value::<AcpJsonRpcMessage>(value.clone())
        {
            callback(self.direction, message);
        }
        if let Some(callback) = &self.on_message
            && let Ok(message) = serde_json::from_value::<AcpJsonRpcMessage>(value)
        {
            callback(self.direction, message);
        }
    }
}

impl<R> TappedReader<R> {
    /// 包装一个异步读取器，并在成功读取后监听新增字节。
    pub(super) fn new(inner: R, tap: MessageTap) -> Self {
        Self { inner, tap }
    }
}

impl<W> TappedWriter<W> {
    /// 包装一个异步写入器，并在成功写入后监听已写字节。
    pub(super) fn new(inner: W, tap: MessageTap) -> Self {
        Self { inner, tap }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for TappedReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let filled_before = buf.filled().len();
        match Pin::new(&mut self.inner).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                let filled_after = buf.filled().len();
                if filled_after > filled_before {
                    self.tap.consume(&buf.filled()[filled_before..filled_after]);
                }
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for TappedWriter<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match Pin::new(&mut self.inner).poll_write(cx, buf) {
            Poll::Ready(Ok(written)) => {
                if written > 0 {
                    self.tap.consume(&buf[..written]);
                }
                Poll::Ready(Ok(written))
            }
            other => other,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}
