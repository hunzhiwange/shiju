// A local HTTP connection blocks the real shell command until the test releases it.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn shell_barrier() -> (tokio::net::TcpListener, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let command = format!(
        "curl --noproxy '*' --silent --max-time 10 http://{}/",
        listener.local_addr().unwrap()
    );
    (listener, command)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn release_shell(mut stream: tokio::net::TcpStream) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut request = [0; 4096];
    stream.read(&mut request).await.unwrap();
    stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
        .await
        .unwrap();
}
