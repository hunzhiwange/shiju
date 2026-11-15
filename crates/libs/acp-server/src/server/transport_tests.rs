//! 验证标准输入输出传输层只写出合法 JSON-RPC 帧。
//!
//! 双向内存流连接服务端与测试客户端，以隔离标准输入输出帧边界。

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::agent_tests::FakeBackend;

// 该端到端用例确认诊断文本不会混入 JSON-RPC 输出流。
#[tokio::test(flavor = "current_thread")]
async fn stdio_transport_writes_only_jsonrpc_frames() {
    let (mut client, server) = tokio::io::duplex(16 * 1024);
    let (server_read, server_write) = tokio::io::split(server);
    let server_task = super::serve_with_io(FakeBackend::default(), server_read, server_write);
    let client_task = async move {
        client
            .write_all(
                br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}
"#,
            )
            .await
            .expect("write initialize");
        let mut line = String::new();
        BufReader::new(&mut client).read_line(&mut line).await.expect("read response");
        client.shutdown().await.expect("shutdown client");
        line
    };

    let (server_result, output) = tokio::join!(server_task, client_task);
    server_result.expect("server transport");
    for line in output.lines() {
        let frame: serde_json::Value = serde_json::from_str(line).expect("JSON-RPC frame only");
        assert_eq!(frame["jsonrpc"], "2.0");
    }
}
