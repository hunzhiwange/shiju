//! Transport fixtures shared by desktop integration tests.
use std::{io::{Read, Write}, net::{TcpListener, TcpStream}, time::Duration};
use gateway_endpoint::GatewayEndpoint;
use vw_gateway_client::GatewayClient;
pub(in crate::app) fn gateway() -> (TcpListener, GatewayClient) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    (listener, GatewayClient::new(endpoint).unwrap())
}

pub(in crate::app) fn request(
    listener: &TcpListener,
    expected: &str,
) -> (TcpStream, String, String) {
    let (mut stream, _) = listener.accept().unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        header.push(byte[0]);
    }
    let header = String::from_utf8(header).unwrap();
    assert!(header.starts_with(expected), "{header}");
    let lower = header.to_ascii_lowercase();
    assert!(lower.contains("x-agent-apps-token: fixture-token\r\n"));
    let len = lower
        .lines()
        .find_map(|line| {
            line.strip_prefix("content-length:").map(|value| value.trim().parse().unwrap())
        })
        .unwrap_or(0);
    let mut body = vec![0; len];
    stream.read_exact(&mut body).unwrap();
    (stream, header, String::from_utf8(body).unwrap())
}

pub(in crate::app) fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
}
