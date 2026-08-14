use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use vw_gateway_client::{
    GatewayClient, GatewayEndpoint, PendingPermissionReplyDto, PendingPermissionRequestDto,
};

use super::super::model::permission::{
    PermissionDecision, PermissionPreviewKind, PermissionQueue, PermissionRequest,
};
use super::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::permission::{
    FullAccessPermissionRuntime, PermissionRuntime, approve_current_directory_for_full_access,
    reply_current_permission,
};

#[derive(Default)]
struct RecordingPermissionRuntime {
    replies: Vec<(String, PendingPermissionReplyDto, Option<String>)>,
    pending: Vec<PermissionRequest>,
    error: Option<String>,
    reply_errors: BTreeSet<String>,
}

impl PermissionRuntime for RecordingPermissionRuntime {
    fn reply(
        &mut self,
        request_id: &str,
        reply: PendingPermissionReplyDto,
        reason: Option<String>,
    ) -> Result<(), String> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        self.replies.push((request_id.to_string(), reply, reason));
        Ok(())
    }
}

impl FullAccessPermissionRuntime for RecordingPermissionRuntime {
    fn pending_for_current_directory(&mut self) -> Result<Vec<PermissionRequest>, String> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        Ok(self.pending.clone())
    }

    fn approve(
        &mut self,
        request_id: &str,
        reply: PendingPermissionReplyDto,
    ) -> Result<(), String> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        if self.reply_errors.contains(request_id) {
            return Err(format!("reply failed for {request_id}"));
        }
        self.replies.push((request_id.to_string(), reply, None));
        Ok(())
    }
}

fn request(id: &str, permission: &str, always: &[&str]) -> PermissionRequest {
    PermissionRequest::from_dto(PendingPermissionRequestDto {
        id: id.to_string(),
        session_id: "session-a".to_string(),
        permission: permission.to_string(),
        patterns: vec!["/workspace/**".to_string()],
        metadata: Map::<String, Value>::new(),
        always: always.iter().map(|value| (*value).to_string()).collect(),
        tool: None,
    })
}

#[test]
fn full_access_pending_drain_uses_once_without_persistable_pattern_and_keeps_processing() {
    let mut runtime = RecordingPermissionRuntime {
        pending: vec![
            request("once-only", "shell", &[]),
            request("fails", "edit", &["/workspace/**"]),
            request("always", "network", &["/workspace/**"]),
        ],
        reply_errors: BTreeSet::from(["fails".to_string()]),
        ..RecordingPermissionRuntime::default()
    };

    let summary = approve_current_directory_for_full_access(&mut runtime);

    assert_eq!(summary.approved, 2);
    assert_eq!(summary.failed, 1);
    assert!(summary.warning.as_deref().is_some_and(|warning| warning.contains("fails")));
    assert_eq!(
        runtime.replies,
        vec![
            ("once-only".to_string(), PendingPermissionReplyDto::Once, None),
            ("always".to_string(), PendingPermissionReplyDto::Always, None),
        ],
        "non-persistable requests use Once and a failed reply does not stop later requests"
    );
}

#[test]
fn tui_permission_queue_replies_once_always_reject_and_recovers_errors() {
    let mut queue = PermissionQueue::default();
    queue.replace(vec![
        request("once", "shell", &[]),
        request("always", "edit", &["/workspace/**"]),
        request("reject", "network", &[]),
    ]);
    assert_eq!(queue.current().unwrap().preview, PermissionPreviewKind::Shell);
    let mut runtime = RecordingPermissionRuntime::default();

    runtime.error = Some("network down".to_string());
    assert!(reply_current_permission(&mut queue, PermissionDecision::Once, &mut runtime,).is_err());
    assert_eq!(queue.current().unwrap().id, "once", "failed reply stays queued");

    runtime.error = None;
    reply_current_permission(&mut queue, PermissionDecision::Once, &mut runtime).unwrap();
    reply_current_permission(&mut queue, PermissionDecision::Always, &mut runtime).unwrap();
    reply_current_permission(
        &mut queue,
        PermissionDecision::Reject { reason: Some("unsafe target".to_string()) },
        &mut runtime,
    )
    .unwrap();
    assert_eq!(queue.len(), 0);
    assert_eq!(
        runtime.replies,
        vec![
            ("once".to_string(), PendingPermissionReplyDto::Once, None),
            ("always".to_string(), PendingPermissionReplyDto::Always, None),
            (
                "reject".to_string(),
                PendingPermissionReplyDto::Reject,
                Some("unsafe target".to_string()),
            ),
        ]
    );

    queue.replace(vec![request("no-pattern", "shell", &[])]);
    assert!(
        reply_current_permission(&mut queue, PermissionDecision::Always, &mut runtime,)
            .unwrap_err()
            .contains("no persistable pattern")
    );
    assert_eq!(queue.current().unwrap().id, "no-pattern");
}

#[test]
fn full_access_approves_only_runtime_scoped_pending_requests_as_always() {
    let mut runtime = RecordingPermissionRuntime {
        pending: vec![
            request("current-a", "shell", &["/workspace/**"]),
            request("current-b", "edit", &["/workspace/**"]),
        ],
        ..RecordingPermissionRuntime::default()
    };

    let summary = approve_current_directory_for_full_access(&mut runtime);
    assert_eq!(summary.approved, 2);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.warning, None);
    assert_eq!(
        runtime.replies,
        vec![
            ("current-a".to_string(), PendingPermissionReplyDto::Always, None),
            ("current-b".to_string(), PendingPermissionReplyDto::Always, None),
        ]
    );

    runtime.error = Some("permission service offline".to_string());
    let summary = approve_current_directory_for_full_access(&mut runtime);
    assert_eq!(summary.approved, 0);
    assert_eq!(summary.failed, 0);
    assert!(summary.warning.as_deref().is_some_and(|warning| warning.contains("offline")));
}

#[test]
fn gateway_full_access_approval_scopes_list_and_replies_to_runtime_directory() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind permission fixture");
    let port = listener.local_addr().expect("permission fixture address").port();
    let server = thread::spawn(move || {
        let responses = [
            r#"[{"id":"current-a","session_id":"session-a","permission":"shell","patterns":[],"metadata":{},"always":[],"tool":null},{"id":"current-b","session_id":"session-a","permission":"edit","patterns":[],"metadata":{},"always":[],"tool":null}]"#,
            "true",
            "true",
        ];
        responses
            .into_iter()
            .map(|body| {
                let (mut stream, _) = listener.accept().expect("accept permission client");
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1024];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    let read = stream.read(&mut buffer).expect("read permission request");
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                }
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).expect("write permission response");
                String::from_utf8(request).expect("permission request should be utf8")
            })
            .collect::<Vec<_>>()
    });
    let mut runtime = GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).expect("gateway client"),
        GatewaySessionSeed::new(PathBuf::from("/tmp/current-project")),
    );

    let summary = approve_current_directory_for_full_access(&mut runtime);
    assert_eq!(summary.approved, 2);
    assert_eq!(summary.failed, 0);
    let requests = server.join().expect("join permission fixture");
    assert!(requests[0].starts_with("GET /v1/permission?directory=%2Ftmp%2Fcurrent-project "));
    assert!(
        requests[1]
            .starts_with("POST /v1/permission/current-a/reply?directory=%2Ftmp%2Fcurrent-project ")
    );
    assert!(
        requests[2]
            .starts_with("POST /v1/permission/current-b/reply?directory=%2Ftmp%2Fcurrent-project ")
    );
    assert!(
        requests.iter().all(|request| !request.contains("other-project")),
        "no request outside the runtime directory may be approved"
    );
}
