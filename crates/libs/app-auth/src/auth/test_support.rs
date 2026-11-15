//! Shared local HTTP fixture for authentication and usage unit tests.
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener, task::JoinHandle};
pub async fn oauth_server(
    status: &str,
    headers: &str,
    body: serde_json::Value,
) -> (reqwest::Url, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = reqwest::Url::parse(&format!(
        "http://{}/login/device/code",
        listener.local_addr().unwrap()
    ))
    .unwrap();
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{headers}Connection: close\r\n\r\n{body}",
        body.to_string().len(),
    );
    let received = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_request(&mut stream).await;
        stream.write_all(reply.as_bytes()).await.unwrap();
        request
    });
    (url, received)
}

pub async fn read_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut headers = Vec::new();
    while !headers.ends_with(b"\r\n\r\n") {
        headers.push(stream.read_u8().await.unwrap());
    }
    let headers = String::from_utf8(headers).unwrap();
    let size = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map_or(0, |(_, value)| value.trim().parse().unwrap());
    let mut body = vec![0; size];
    stream.read_exact(&mut body).await.unwrap();
    format!("{headers}{}", String::from_utf8(body).unwrap())
}
