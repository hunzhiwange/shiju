//! Only the external HTTP service is replaced; catalog algorithms remain real.
use gateway_endpoint::GatewayEndpoint;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};
pub(super) fn with_gateway(
    test: impl FnOnce(GatewayEndpoint, Receiver<String>, Sender<Option<String>>),
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (request_tx, requests) = mpsc::channel();
    let (responses, response_rx) = mpsc::channel::<Option<String>>();
    let server = std::thread::spawn(move || {
        while let Ok((mut stream, _)) = listener.accept() {
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let request = read_http_request(&mut stream);
            if request.is_empty() || request_tx.send(request).is_err() {
                break;
            }
            let body = match response_rx.recv_timeout(Duration::from_secs(5)) {
                Ok(Some(body)) => body,
                Ok(None) => continue,
                Err(_) => break,
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        test(GatewayEndpoint::new("127.0.0.1", address.port()), requests, responses)
    }));
    drop(TcpStream::connect(address));
    server.join().unwrap();
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}
fn read_http_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 512];
    loop {
        let read = stream.read(&mut buffer).expect("request should be readable");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if let Some(header_end) =
            request.windows(4).position(|window| window == b"\r\n\r\n").map(|index| index + 4)
        {
            let headers = String::from_utf8_lossy(&request[..header_end]);
            if request.len() >= header_end + http_content_length(&headers) {
                break;
            }
        }
    }
    String::from_utf8(request).expect("request should be utf8")
}

#[cfg(not(target_arch = "wasm32"))]
fn http_content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if !name.eq_ignore_ascii_case("content-length") {
                return None;
            }
            value.trim().parse().ok()
        })
        .unwrap_or(0)
}
