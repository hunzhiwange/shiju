#![allow(unused_imports, dead_code)]

//! `task-pool` 路由、确认与内部 publish 排除测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};

use clap::Parser;
use serde_json::{Value, json};

use super::run_with_io;
use crate::cli::Cli;


struct Case {
    command: Vec<String>,
    input: Vec<u8>,
    path: &'static str,
    body: Value,
}
impl Case {
    fn new(command: &[&str], input: impl AsRef<[u8]>, path: &'static str, body: Value) -> Self {
        Self {
            command: command.iter().map(|value| (*value).to_string()).collect(),
            input: input.as_ref().to_vec(),
            path,
            body,
        }
    }
}

async fn run_command<S: AsRef<str>>(
    common: &[&str],
    command: &[S],
    input: &[u8],
) -> Result<Vec<u8>, String> {
    let (yes, command) = if command.first().is_some_and(|part| part.as_ref() == "--yes") {
        (Some("--yes"), &command[1..])
    } else {
        (None, command)
    };
    let (family, command) = command.split_first().ok_or("API command family is required")?;
    let args = std::iter::once("shiju")
        .chain(std::iter::once(family.as_ref()))
        .chain(common.iter().copied())
        .chain(yes)
        .chain(command.iter().map(AsRef::as_ref))
        .collect::<Vec<_>>();
    let cli = Cli::try_parse_from(args).map_err(|error| error.to_string())?;
    let api_args = cli.command.into_api_args().ok_or("expected API command")?;
    let mut output = Vec::new();
    run_with_io(api_args, &mut Cursor::new(input), &mut output)
        .await
        .map_err(|error| error.to_string())?;
    Ok(output)
}

#[derive(Debug)]
struct RecordedRequest {
    method: String,
    path: String,
    body: Value,
}
fn spawn_server(responses: Vec<Value>) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind task-pool server");
    let port = listener.local_addr().expect("address").port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().expect("accept");
            tx.send(read_request(&mut stream)).expect("record");
            let response = response.to_string();
            write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}", response.len()).expect("respond");
        }
    });
    (port, rx)
}
fn read_request(stream: &mut std::net::TcpStream) -> RecordedRequest {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 1024];
    let end = loop {
        if let Some(p) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break p;
        }
        let n = stream.read(&mut chunk).expect("read");
        assert!(n > 0);
        bytes.extend_from_slice(&chunk[..n]);
    };
    let headers = String::from_utf8(bytes[..end].to_vec()).expect("headers");
    let len = headers
        .lines()
        .filter_map(|l| l.split_once(':'))
        .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = bytes[(end + 4)..].to_vec();
    while body.len() < len {
        let n = stream.read(&mut chunk).expect("body");
        assert!(n > 0);
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(len);
    let mut line = headers.lines().next().expect("line").split_whitespace();
    let method = line.next().expect("method").to_string();
    let path = line.next().expect("path").to_string();
    let body =
        if body.is_empty() { Value::Null } else { serde_json::from_slice(&body).expect("json") };
    RecordedRequest { method, path, body }
}
