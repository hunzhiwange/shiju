//! `api task-pool` 路由、确认与内部 publish 排除测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};

use clap::Parser;
use serde_json::{Value, json};

use super::run_with_io;
use crate::cli::{Cli, Commands};

#[tokio::test(flavor = "current_thread")]
async fn api_task_pool_commands_cover_pool_operations_and_exclude_internal_publish() {
    let responses = vec![
        json!({"project_id":"project-1","repo_root":"/repo","pool_root":"/repo/.pool","base_branch":"main","idle_count":0,"busy_count":0,"tainted_count":0,"recycling_count":0,"dead_count":0,"merge_target_locks":[],"slots":[]}),
        json!({"idle_count":2,"target_idle_count":2}),
        json!({"leased":false}), json!({"released":true}), json!({"recycled":true}),
        json!({"recycled":true}), json!({"locked":true}), json!({"released":true}),
        json!({"cursor":7,"events":[]}),
    ];
    let (port, requests) = spawn_server(responses);
    let port = port.to_string();
    let common = ["--host", "127.0.0.1", "--port", port.as_str()];
    let inputs = [
        br#"{"project_id":"project-1","desired_capacity":2}"#.as_slice(),
        br#"{"project_id":"project-1","task_id":"task-1"}"#.as_slice(),
        br#"{"project_id":"project-1","task_id":"task-1"}"#.as_slice(),
        br#"{"project_id":"project-1","task_id":"task-1","taint_reason":"dirty"}"#.as_slice(),
        br#"{"project_id":"project-1","slot_id":"slot-1"}"#.as_slice(),
        br#"{"project_id":"project-1","task_id":"task-1","target_branch":"main"}"#.as_slice(),
        br#"{"project_id":"project-1","task_id":"task-1","target_branch":"main"}"#.as_slice(),
    ];
    let dangerous = ["maintain", "acquire", "release", "recycle", "slot-recycle", "merge-lock", "merge-unlock"];
    for (operation, input) in dangerous.iter().zip(inputs.iter()) {
        let error = run_command(&common, &["task-pool", operation, "--input", "-"], input)
            .await.expect_err("pool mutation must require --yes");
        assert!(error.contains("--yes"));
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }
    let publish = run_command(&common, &["task-pool", "publish"], b"")
        .await.expect_err("internal publish must be excluded");
    assert!(publish.contains("internal producer"));
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));

    let cases = [
        Case::new(&["task-pool", "snapshot", "--input", "-"], br#"{"project_id":"project-1"}"#, "/v1/task-pool/worktrees/snapshot", json!({"project_id":"project-1"})),
        Case::new(&["--yes", "task-pool", "maintain", "--input", "-"], inputs[0], "/v1/task-pool/worktrees/maintain", json!({"project_id":"project-1","desired_capacity":2})),
        Case::new(&["--yes", "task-pool", "acquire", "--input", "-"], inputs[1], "/v1/task-pool/worktrees/acquire", json!({"project_id":"project-1","task_id":"task-1"})),
        Case::new(&["--yes", "task-pool", "release", "--input", "-"], inputs[2], "/v1/task-pool/worktrees/release", json!({"project_id":"project-1","task_id":"task-1"})),
        Case::new(&["--yes", "task-pool", "recycle", "--input", "-"], inputs[3], "/v1/task-pool/worktrees/recycle", json!({"project_id":"project-1","task_id":"task-1","taint_reason":"dirty"})),
        Case::new(&["--yes", "task-pool", "slot-recycle", "--input", "-"], inputs[4], "/v1/task-pool/worktrees/slots/recycle", json!({"project_id":"project-1","slot_id":"slot-1"})),
        Case::new(&["--yes", "task-pool", "merge-lock", "--input", "-"], inputs[5], "/v1/task-pool/worktrees/merge-lock", json!({"project_id":"project-1","task_id":"task-1","target_branch":"main"})),
        Case::new(&["--yes", "task-pool", "merge-unlock", "--input", "-"], inputs[6], "/v1/task-pool/worktrees/merge-unlock", json!({"project_id":"project-1","task_id":"task-1","target_branch":"main"})),
        Case::new(&["task-pool", "events", "--after-cursor", "6"], b"", "/v1/task-pool/matrix/events?after_cursor=6", Value::Null),
    ];
    for case in cases {
        run_command(&common, &case.command, &case.input).await.unwrap_or_else(|error| panic!("task-pool command failed: {error}"));
        let request = requests.recv().expect("task-pool request");
        assert_eq!(request.method, if case.path.contains("matrix/events") { "GET" } else { "POST" });
        assert_eq!(request.path, case.path);
        assert_eq!(request.body, case.body);
    }
}

struct Case { command: Vec<String>, input: Vec<u8>, path: &'static str, body: Value }
impl Case {
    fn new(command: &[&str], input: impl AsRef<[u8]>, path: &'static str, body: Value) -> Self {
        Self { command: command.iter().map(|value| (*value).to_string()).collect(), input: input.as_ref().to_vec(), path, body }
    }
}

async fn run_command<S: AsRef<str>>(common: &[&str], command: &[S], input: &[u8]) -> Result<Vec<u8>, String> {
    let args = std::iter::once("shiju").chain(std::iter::once("api")).chain(common.iter().copied()).chain(command.iter().map(AsRef::as_ref)).collect::<Vec<_>>();
    let cli = Cli::try_parse_from(args).map_err(|error| error.to_string())?;
    let Commands::Api { api_args } = cli.command else { return Err("expected api command".into()) };
    let mut output = Vec::new();
    run_with_io(api_args, &mut Cursor::new(input), &mut output).await.map_err(|error| error.to_string())?;
    Ok(output)
}

#[derive(Debug)]
struct RecordedRequest { method: String, path: String, body: Value }
fn spawn_server(responses: Vec<Value>) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind task-pool server");
    let port = listener.local_addr().expect("address").port(); let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || for response in responses {
        let (mut stream, _) = listener.accept().expect("accept"); tx.send(read_request(&mut stream)).expect("record");
        let response = response.to_string(); write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}", response.len()).expect("respond");
    }); (port, rx)
}
fn read_request(stream: &mut std::net::TcpStream) -> RecordedRequest {
    let mut bytes=Vec::new(); let mut chunk=[0_u8;1024]; let end=loop { if let Some(p)=bytes.windows(4).position(|w|w==b"\r\n\r\n"){break p} let n=stream.read(&mut chunk).expect("read"); assert!(n>0); bytes.extend_from_slice(&chunk[..n]);};
    let headers=String::from_utf8(bytes[..end].to_vec()).expect("headers"); let len=headers.lines().filter_map(|l|l.split_once(':')).find(|(n,_)|n.eq_ignore_ascii_case("content-length")).and_then(|(_,v)|v.trim().parse::<usize>().ok()).unwrap_or(0); let mut body=bytes[(end+4)..].to_vec(); while body.len()<len {let n=stream.read(&mut chunk).expect("body");assert!(n>0);body.extend_from_slice(&chunk[..n]);} body.truncate(len);
    let mut line=headers.lines().next().expect("line").split_whitespace(); let method=line.next().expect("method").to_string();let path=line.next().expect("path").to_string();let body=if body.is_empty(){Value::Null}else{serde_json::from_slice(&body).expect("json")};RecordedRequest{method,path,body}
}
