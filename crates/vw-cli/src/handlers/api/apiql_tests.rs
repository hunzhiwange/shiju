//! `api apiql` 四种规范读取路由与查询校验测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};

use clap::Parser;
use serde_json::{Value, json};

use super::run_with_io;
use crate::cli::{Cli, Commands};

#[tokio::test(flavor = "current_thread")]
async fn api_apiql_commands_use_canonical_routes_for_all_query_modes() {
    let (port, requests) = spawn_server(vec![
        json!({"data":[{"id":1}]}),
        json!({"data":[{"id":1}]}),
        json!({"data":3}),
        json!({"data":[],"page":{"per_page":20,"current_page":2,"total_page":null,"total_record":null,"total_macro":0,"from":21,"to":null}}),
    ]);
    let port = port.to_string();
    let common = ["--host", "127.0.0.1", "--port", port.as_str()];

    for invalid in ["page=0", "page_size=0", "limit=not-a-number", "=value"] {
        let error = run_command(
            &common,
            &["apiql", "list-page", "product:product", "--param", invalid],
        )
        .await
        .expect_err("invalid APIQL pagination must fail");
        assert!(error.contains("invalid APIQL query parameter"));
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    let cases = [
        (
            vec![
                "apiql",
                "list",
                "product:product",
                "--param",
                "column[]=id",
                "--param",
                "orderBy=id asc",
                "--param",
                "limit=2",
            ],
            "/apiQL/product%3Aproduct?column%5B%5D=id&orderBy=id+asc&limit=2",
        ),
        (
            vec![
                "apiql",
                "list-only",
                "product:product",
                "--param",
                "column[]=id",
            ],
            "/apiQL/product%3Aproduct/list-only?column%5B%5D=id",
        ),
        (
            vec!["apiql", "list-count", "product:product", "--param", "active=1"],
            "/apiQL/product%3Aproduct/list-count?active=1",
        ),
        (
            vec![
                "apiql",
                "list-page",
                "product:product",
                "--param",
                "page=2",
                "--param",
                "page_size=20",
            ],
            "/apiQL/product%3Aproduct/list-page?page=2&page_size=20",
        ),
    ];
    for (command, path) in cases {
        run_command(&common, &command)
            .await
            .unwrap_or_else(|error| panic!("APIQL command failed: {error}"));
        let request = requests.recv().expect("APIQL request");
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, path);
        assert!(!request.path.split('?').next().expect("path").ends_with('/'));
    }
}

async fn run_command<S: AsRef<str>>(common: &[&str], command: &[S]) -> Result<Vec<u8>, String> {
    let args = std::iter::once("shiju")
        .chain(std::iter::once("api"))
        .chain(common.iter().copied())
        .chain(command.iter().map(AsRef::as_ref))
        .collect::<Vec<_>>();
    let cli = Cli::try_parse_from(args).map_err(|error| error.to_string())?;
    let Commands::Api { api_args } = cli.command else {
        return Err("expected api command".into());
    };
    let mut output = Vec::new();
    run_with_io(api_args, &mut Cursor::new(Vec::<u8>::new()), &mut output)
        .await
        .map_err(|error| error.to_string())?;
    Ok(output)
}

#[derive(Debug)]
struct RecordedRequest {
    method: String,
    path: String,
}

fn spawn_server(responses: Vec<Value>) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind APIQL server");
    let port = listener.local_addr().expect("address").port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().expect("accept");
            tx.send(read_request(&mut stream)).expect("record");
            let response = response.to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}",
                response.len()
            )
            .expect("respond");
        }
    });
    (port, rx)
}

fn read_request(stream: &mut std::net::TcpStream) -> RecordedRequest {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        let read = stream.read(&mut chunk).expect("read request");
        assert!(read > 0);
        bytes.extend_from_slice(&chunk[..read]);
    }
    let headers = String::from_utf8(bytes).expect("request headers");
    let mut request_line = headers.lines().next().expect("request line").split_whitespace();
    RecordedRequest {
        method: request_line.next().expect("method").to_string(),
        path: request_line.next().expect("path").to_string(),
    }
}
