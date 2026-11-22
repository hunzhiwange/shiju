use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

use serde_json::{Value, json};

use super::*;

const CREDENTIAL: &str = "fixture-employee-credential";
const PRIVATE_ID: &str = "private/员工?#%";
const SHARED_ID: &str = "shared/员工?#%";
const CONVERSATION_ID: &str = "ca7395ee-0b56-438a-9c1e-ac46c4c9c650";
const TURN_ID: &str = "b02b7e38-b75f-4151-bf06-96ba6cfeca0d";
const TIME: &str = "2026-11-21T00:00:00Z";

struct Request {
    line: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find_map(|(key, value)| (key == name).then_some(value.as_str()))
    }
}

fn read_request(stream: &mut TcpStream) -> Request {
    stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let mut headers = Vec::new();
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).unwrap();
        if header == "\r\n" {
            break;
        }
        let (name, value) = header.split_once(':').expect("HTTP header");
        headers.push((name.to_ascii_lowercase(), value.trim().to_owned()));
    }
    let mut request = Request { line: line.trim_end().to_owned(), headers, body: Vec::new() };
    if let Some(length) = request.header("content-length") {
        let length = length.parse::<usize>().unwrap();
        request.body.resize(length, 0);
        reader.read_exact(&mut request.body).unwrap();
    }
    request
}

struct Reply {
    status: u16,
    headers: String,
    body: Vec<u8>,
}

fn reply(status: u16, body: Value) -> Reply {
    Reply { status, headers: String::new(), body: serde_json::to_vec(&body).unwrap() }
}

fn fixture(replies: Vec<Reply>) -> (EmployeeConversationClient, thread::JoinHandle<Vec<Request>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let credential = ShijuCredential::new(CREDENTIAL).unwrap();
    let client =
        EmployeeConversationClient::with_loopback(listener.local_addr().unwrap(), &credential)
            .unwrap();
    let server = thread::spawn(move || {
        replies.into_iter().map(|reply| {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            write!(
                stream,
                "HTTP/1.1 {} Fixture\r\nContent-Type: application/json\r\nConnection: close\r\n{}Content-Length: {}\r\n\r\n",
                reply.status, reply.headers, reply.body.len()
            ).unwrap();
            if !reply.body.is_empty() {
                // A client may close early upon seeing an oversized Content-Length.
                let _ = stream.write_all(&reply.body);
            }
            request
        }).collect()
    });
    (client, server)
}

fn conversation(assistant_id: &str, kind: &str) -> Value {
    json!({
        "conversation_id": CONVERSATION_ID, "assistant_id": assistant_id,
        "active_version_id": "v1", "title": "员工对话", "kind": kind,
        "status": "ACTIVE", "created_at": TIME, "updated_at": TIME
    })
}

fn message(sender: &str, body: &str) -> Value {
    json!({"message_id": "user-message", "sender": sender, "body": body, "created_at": TIME})
}

fn turn(status: &str) -> Value {
    json!({
        "turn_id": TURN_ID, "user_message_id": "user-message", "employee_version_id": "v1",
        "status": status, "created_at": TIME, "updated_at": TIME
    })
}

fn submission(conversation: &Value, body: &str, idempotent: bool) -> Value {
    json!({
        "conversation": conversation, "message": message("user", body),
        "turn": turn("ACCEPTED"), "idempotent": idempotent
    })
}

#[tokio::test]
async fn employee_conversations_routes_and_scopes_requests() {
    let private = EmployeeTarget { employee_id: PRIVATE_ID.into(), kind: EmployeeKind::Private };
    let shared = EmployeeTarget { employee_id: SHARED_ID.into(), kind: EmployeeKind::Shared };
    let private_conversation = conversation(PRIVATE_ID, "PRIVATE_DIRECT");
    let shared_conversation = conversation(SHARED_ID, "SHARED_DIRECT");
    let (client, server) = fixture(vec![
        reply(200, json!({"items": [private_conversation]})),
        reply(200, json!({"items": [shared_conversation]})),
        reply(201, submission(&private_conversation, "第一条", false)),
        reply(201, submission(&shared_conversation, "共享首条", false)),
        reply(
            200,
            json!({
                "messages": [{
                    "message_id": "employee-message", "sender": "employee", "body": "你好",
                    "reply_to_message_id": "user-message", "created_at": TIME
                }],
                "turns": [turn("COMPLETED")]
            }),
        ),
        reply(201, submission(&shared_conversation, "继续", true)),
        reply(200, turn("CANCELLED")),
    ]);

    let private_list = client.list_conversations(&private).await.unwrap();
    assert_eq!(private_list[0].assistant_id, PRIVATE_ID);
    assert_eq!(private_list[0].kind, ConversationKind::PrivateDirect);
    let shared_list = client.list_conversations(&shared).await.unwrap();
    assert_eq!(shared_list[0].assistant_id, SHARED_ID);
    assert_eq!(shared_list[0].kind, ConversationKind::SharedDirect);
    assert_eq!(
        client
            .create_conversation(&private, "第一条", Some("员工对话"), "first-private")
            .await
            .unwrap()
            .turn
            .status,
        TurnStatus::Accepted
    );
    assert_eq!(
        client
            .create_conversation(&shared, "共享首条", None, "first-shared")
            .await
            .unwrap()
            .conversation
            .assistant_id,
        SHARED_ID
    );
    let history = client.history(CONVERSATION_ID).await.unwrap();
    assert_eq!(history.messages[0].sender, MessageSender::Employee);
    assert_eq!(history.messages[0].reply_to_message_id.as_deref(), Some("user-message"));
    assert_eq!(history.turns[0].status, TurnStatus::Completed);
    assert!(client.send_message(CONVERSATION_ID, "继续", "next").await.unwrap().idempotent);
    assert_eq!(
        client.cancel_turn(CONVERSATION_ID, TURN_ID).await.unwrap().status,
        TurnStatus::Cancelled
    );

    let requests = server.join().unwrap();
    let encoded = "%2F%E5%91%98%E5%B7%A5%3F%23%25";
    let private_route = format!("assistants/private{encoded}/conversations");
    let shared_route = format!("employees/shared{encoded}/conversations");
    let messages_route = format!("conversations/{CONVERSATION_ID}/messages");
    let cancel_route = format!("conversations/{CONVERSATION_ID}/turns/{TURN_ID}/cancel");
    let expected = [
        ("GET", &private_route, None, None),
        ("GET", &shared_route, None, None),
        (
            "POST",
            &private_route,
            Some("first-private"),
            Some(json!({"message": "第一条", "title": "员工对话"})),
        ),
        ("POST", &shared_route, Some("first-shared"), Some(json!({"message": "共享首条"}))),
        ("GET", &messages_route, None, None),
        ("POST", &messages_route, Some("next"), Some(json!({"message": "继续"}))),
        ("POST", &cancel_route, None, None),
    ];
    for (request, (method, route, key, body)) in requests.iter().zip(expected) {
        assert_eq!(request.line, format!("{method} /sj/agent/api/v1/{route} HTTP/1.1"));
        assert!(request.header("x-api-key") == Some(CREDENTIAL), "expected user credential");
        for name in ["authorization", "cookie", "x-company-id", "x-user-id", "x-tenant-id"] {
            assert!(request.header(name).is_none(), "unexpected {name} header");
        }
        assert_eq!(request.header("idempotency-key"), key);
        if let Some(body) = body {
            assert_eq!(serde_json::from_slice::<Value>(&request.body).unwrap(), body);
        } else {
            assert!(request.body.is_empty());
        }
    }
}

#[tokio::test]
async fn employee_conversations_redacts_errors_and_rejects_redirects() {
    let destination = TcpListener::bind("127.0.0.1:0").unwrap();
    destination.set_nonblocking(true).unwrap();
    let cases = [
        (401, "ignored", Error::Unauthorized),
        (403, "ignored", Error::Forbidden),
        (404, "EMPLOYEE_NOT_FOUND", Error::NotFound),
        (409, "CONVERSATION_BUSY", Error::Busy),
        (409, "IDEMPOTENCY_CONFLICT", Error::IdempotencyConflict),
        (409, "unexpected-conflict-secret", Error::Conflict),
        (429, "ignored", Error::Unavailable),
        (503, "ignored", Error::Unavailable),
    ];
    let mut replies = cases
        .iter()
        .map(|&(status, code, _)| {
            reply(
                status,
                json!({"code": code, "message": "upstream-body-secret", "trace_id": CREDENTIAL}),
            )
        })
        .collect::<Vec<_>>();
    replies.push(Reply {
        status: 307,
        headers: format!("Location: http://{}/capture\r\n", destination.local_addr().unwrap()),
        body: Vec::new(),
    });
    replies.push(Reply { status: 200, headers: String::new(), body: vec![b' '; (8 << 20) + 1] });
    replies.push(reply(201, json!({"messages": [], "turns": []})));
    let (client, server) = fixture(replies);

    for (_, code, expected) in cases {
        let error = client.history(CONVERSATION_ID).await.unwrap_err();
        assert_eq!(error, expected);
        let diagnostic = format!("{error} {error:?}");
        for secret in ["upstream-body-secret", CREDENTIAL, code] {
            assert!(!diagnostic.contains(secret), "diagnostic exposed upstream data");
        }
    }

    for _ in 0..3 {
        assert_eq!(client.history(CONVERSATION_ID).await.unwrap_err(), Error::InvalidResponse);
    }
    assert_eq!(destination.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
    assert_eq!(server.join().unwrap().len(), cases.len() + 3);
}
