use std::sync::Arc;
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use parking_lot::Mutex;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

use crate::types::{AcpAgentConfig, AcpMessageDirection};

use super::{AcpClient, MessageTap, ProcessHandles, TappedReader, TappedWriter};

struct FailingReader;

impl AsyncRead for FailingReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Err(io::Error::other("read failed")))
    }
}

struct PendingReader;

impl AsyncRead for PendingReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Pending
    }
}

struct ZeroWriter;

impl AsyncWrite for ZeroWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(0))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

struct FailingWriter;

impl AsyncWrite for FailingWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Err(io::Error::other("write failed")))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

struct PendingWriter;

impl AsyncWrite for PendingWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Pending
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn callback_values() -> Arc<Mutex<Vec<(AcpMessageDirection, Value)>>> {
    Arc::new(Mutex::new(Vec::new()))
}

fn push_callback(
    seen: &Arc<Mutex<Vec<(AcpMessageDirection, Value)>>>,
) -> crate::types::AcpMessageCallback {
    let seen = Arc::clone(seen);
    Arc::new(move |direction, message| {
        seen.lock().push((direction, serde_json::to_value(message).expect("serialize message")));
    })
}

#[tokio::test]
async fn tap_buffers_partial_lines_and_ignores_non_jsonrpc_payloads() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let tap = MessageTap::new(
        AcpMessageDirection::Inbound,
        Some({
            let seen = Arc::clone(&seen);
            Arc::new(move |direction, message| {
                seen.lock().push((direction, message));
            })
        }),
        None,
    );
    let (mut writer, reader) = tokio::io::duplex(128);
    writer
        .write_all(br#"{"jsonrpc":"2.0","method":"session/up"#)
        .await
        .expect("write partial input");
    assert!(seen.lock().is_empty());
    writer.write_all(br#"date"}"#).await.expect("write rest");
    writer
        .write_all(b"\nnot-json\n{\"jsonrpc\":\"1.0\",\"method\":\"bad\"}\n")
        .await
        .expect("write invalid input");
    drop(writer);

    let mut tapped_reader = TappedReader::new(reader, tap);
    let mut output = Vec::new();
    tapped_reader.read_to_end(&mut output).await.expect("read tapped");

    let seen = seen.lock();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, AcpMessageDirection::Inbound);
}

#[tokio::test]
async fn tapped_reader_and_writer_forward_successful_bytes() {
    let read_seen = Arc::new(Mutex::new(0));
    let reader_tap = MessageTap::new(
        AcpMessageDirection::Inbound,
        Some({
            let read_seen = Arc::clone(&read_seen);
            Arc::new(move |_, _| *read_seen.lock() += 1)
        }),
        None,
    );
    let input = tokio::io::duplex(128);
    let (mut writer, reader) = input;
    writer.write_all(br#"{"jsonrpc":"2.0","method":"session/update"}"#).await.expect("write input");
    writer.write_all(b"\n").await.expect("write newline");
    drop(writer);

    let mut tapped_reader = TappedReader::new(reader, reader_tap);
    let mut sink = Vec::new();
    tapped_reader.read_to_end(&mut sink).await.expect("read tapped");
    assert_eq!(*read_seen.lock(), 1);

    let write_seen = Arc::new(Mutex::new(0));
    let writer_tap = MessageTap::new(
        AcpMessageDirection::Outbound,
        Some({
            let write_seen = Arc::clone(&write_seen);
            Arc::new(move |_, _| *write_seen.lock() += 1)
        }),
        None,
    );
    let (client, mut server) = tokio::io::duplex(128);
    let mut tapped_writer = TappedWriter::new(client, writer_tap);
    tapped_writer
        .write_all(br#"{"jsonrpc":"2.0","id":1,"result":{}}"#)
        .await
        .expect("write tapped");
    tapped_writer.write_all(b"\n").await.expect("write newline");
    tapped_writer.shutdown().await.expect("shutdown");
    let mut output = Vec::new();
    server.read_to_end(&mut output).await.expect("read output");

    assert_eq!(*write_seen.lock(), 1);
}

#[tokio::test]
async fn tapped_child_stdio_dispatches_real_process_streams() {
    let raw_seen = callback_values();
    let output_seen = callback_values();
    let client = AcpClient::new(
        "test-agent",
        AcpAgentConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "cat".to_string()],
            env: Default::default(),
        },
    )
    .with_acp_message_callback(Some(push_callback(&raw_seen)))
    .with_acp_output_message_callback(Some(push_callback(&output_seen)));
    let ProcessHandles { mut child, stderr_task } = client.spawn_child().expect("spawn should run");
    let stdin = child.stdin.take().expect("child stdin");
    let stdout = child.stdout.take().expect("child stdout");
    let mut tapped_writer =
        TappedWriter::new(stdin, client.make_message_tap(AcpMessageDirection::Outbound));
    let mut tapped_reader =
        TappedReader::new(stdout, client.make_message_tap(AcpMessageDirection::Inbound));

    tapped_writer
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":\"child\",\"result\":{}}\n")
        .await
        .expect("write child input");
    tapped_writer.shutdown().await.expect("shutdown child input");
    drop(tapped_writer);
    let mut output = Vec::new();
    tapped_reader.read_to_end(&mut output).await.expect("read child output");
    let finalized = client.finalize_child(child, stderr_task).await;

    assert_eq!(output, b"{\"jsonrpc\":\"2.0\",\"id\":\"child\",\"result\":{}}\n");
    assert_eq!(finalized.summary.exit_code, Some(0));
    let raw_seen = raw_seen.lock();
    let output_seen = output_seen.lock();
    assert_eq!(raw_seen.len(), 2);
    assert_eq!(output_seen.len(), 2);
    assert_eq!(raw_seen[0].0, AcpMessageDirection::Outbound);
    assert_eq!(raw_seen[1].0, AcpMessageDirection::Inbound);
}

#[tokio::test]
async fn client_message_tap_dispatches_raw_and_output_callbacks() {
    let raw_seen = callback_values();
    let output_seen = callback_values();
    let client = AcpClient::new(
        "test-agent",
        AcpAgentConfig { command: "unused".to_string(), args: Vec::new(), env: Default::default() },
    )
    .with_acp_message_callback(Some(push_callback(&raw_seen)))
    .with_acp_output_message_callback(Some(push_callback(&output_seen)));
    let (mut writer, reader) = tokio::io::duplex(128);
    writer
        .write_all(b"\n{\"jsonrpc\":\"2.0\",\"method\":\"session/update\"}\r\n")
        .await
        .expect("write message");
    writer
        .write_all(b"\r\n{\"jsonrpc\":\"2.0\",\"id\":\"ok\",\"result\":{}}\n")
        .await
        .expect("write response");
    drop(writer);

    let mut tapped_reader =
        TappedReader::new(reader, client.make_message_tap(AcpMessageDirection::Inbound));
    let mut output = Vec::new();
    tapped_reader.read_to_end(&mut output).await.expect("read tapped");

    let raw_seen = raw_seen.lock();
    let output_seen = output_seen.lock();
    assert_eq!(raw_seen.len(), 2);
    assert_eq!(output_seen.len(), 2);
    assert_eq!(raw_seen[0].0, AcpMessageDirection::Inbound);
    assert_eq!(output_seen[0].0, AcpMessageDirection::Inbound);
    assert_eq!(raw_seen[0].1["method"], "session/update");
    assert_eq!(output_seen[1].1["id"], "ok");
}

#[tokio::test]
async fn tap_dispatches_output_callback_without_raw_callback() {
    let output_seen = callback_values();
    let tap =
        MessageTap::new(AcpMessageDirection::Outbound, None, Some(push_callback(&output_seen)));
    let (mut writer, reader) = tokio::io::duplex(128);
    writer
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":\"ok\",\"result\":{}}\n")
        .await
        .expect("write message");
    drop(writer);

    let mut tapped_reader = TappedReader::new(reader, tap);
    let mut output = Vec::new();
    tapped_reader.read_to_end(&mut output).await.expect("read tapped");

    let output_seen = output_seen.lock();
    assert_eq!(output_seen.len(), 1);
    assert_eq!(output_seen[0].0, AcpMessageDirection::Outbound);
    assert_eq!(output_seen[0].1["id"], "ok");
}

#[tokio::test]
async fn tap_without_callbacks_forwards_bytes_without_buffering() {
    let tap = MessageTap::new(AcpMessageDirection::Outbound, None, None);
    let (client, mut server) = tokio::io::duplex(128);
    let mut tapped_writer = TappedWriter::new(client, tap);

    tapped_writer
        .write_all(br#"{"jsonrpc":"2.0","method":"session/update"}"#)
        .await
        .expect("write message");
    tapped_writer.write_all(b"\n").await.expect("write newline");
    tapped_writer.shutdown().await.expect("shutdown writer");

    let mut output = Vec::new();
    server.read_to_end(&mut output).await.expect("read forwarded bytes");

    let expected = br#"{"jsonrpc":"2.0","method":"session/update"}
"#;
    assert_eq!(output, expected);
    assert!(tapped_writer.tap.buffer.is_empty());
}

#[tokio::test]
async fn tapped_reader_does_not_consume_failed_reads() {
    let seen = callback_values();
    let tap = MessageTap::new(AcpMessageDirection::Inbound, Some(push_callback(&seen)), None);
    let mut tapped_reader = TappedReader::new(FailingReader, tap);
    let mut output = Vec::new();

    let error = tapped_reader.read_to_end(&mut output).await.expect_err("read should fail");

    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert!(seen.lock().is_empty());
}

#[test]
fn tapped_reader_preserves_pending_reads() {
    let seen = callback_values();
    let tap = MessageTap::new(AcpMessageDirection::Inbound, Some(push_callback(&seen)), None);
    let mut tapped_reader = TappedReader::new(PendingReader, tap);
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);
    let mut bytes = [0; 32];
    let mut buffer = ReadBuf::new(&mut bytes);

    let result = Pin::new(&mut tapped_reader).poll_read(&mut cx, &mut buffer);

    assert!(matches!(result, Poll::Pending));
    assert!(buffer.filled().is_empty());
    assert!(seen.lock().is_empty());
}

#[tokio::test]
async fn tapped_writer_ignores_zero_byte_and_failed_writes() {
    let seen = callback_values();
    let tap = MessageTap::new(AcpMessageDirection::Outbound, Some(push_callback(&seen)), None);
    let mut zero_writer = TappedWriter::new(ZeroWriter, tap);

    let written = zero_writer
        .write(br#"{"jsonrpc":"2.0","method":"session/update"}"#)
        .await
        .expect("zero write should return");

    assert_eq!(written, 0);
    assert!(seen.lock().is_empty());
    zero_writer.flush().await.expect("flush should forward");
    zero_writer.shutdown().await.expect("shutdown should forward");

    let tap = MessageTap::new(AcpMessageDirection::Outbound, Some(push_callback(&seen)), None);
    let mut failing_writer = TappedWriter::new(FailingWriter, tap);
    let error = failing_writer
        .write(br#"{"jsonrpc":"2.0","method":"session/update"}"#)
        .await
        .expect_err("write should fail");

    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert!(seen.lock().is_empty());
}

#[test]
fn tapped_writer_preserves_pending_writes() {
    let seen = callback_values();
    let tap = MessageTap::new(AcpMessageDirection::Outbound, Some(push_callback(&seen)), None);
    let mut tapped_writer = TappedWriter::new(PendingWriter, tap);
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);

    let result =
        Pin::new(&mut tapped_writer).poll_write(&mut cx, br#"{"jsonrpc":"2.0","method":"x"}"#);

    assert!(matches!(result, Poll::Pending));
    assert!(seen.lock().is_empty());
}
