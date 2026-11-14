use std::io::Write;

use rusqlite::Connection;
use serde_json::{Value, json};
use tempfile::TempDir;
use vw_agent_apps::{
    sessions::{providers::claude::scan_sessions, source::SourceRegistry},
    usage::{
        claude_scan::scan_session,
        cursors::initialize_usage_cursors,
        store::{StoredUsageRecord, initialize_usage_store},
    },
};
use vw_api_types::{ProjectId, ProjectScope, external_session::*, usage::UsageMeasurementKind};

struct Fixture {
    _root: TempDir,
    file: std::path::PathBuf,
    connection: Connection,
    registry: SourceRegistry,
    identity: ExternalSessionIdentity,
}
impl Fixture {
    fn new(rows: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("projects/project/session-a.jsonl");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, rows).unwrap();
        let scope = ProjectScope::for_default_tenant(ProjectId::from("project")).unwrap();
        let template = ExternalSessionIdentity {
            scope: scope.clone(),
            target_host: "host".into(),
            instance_id: "native".into(),
            provider: SessionProvider::Claude,
            session_id: ExternalSessionId("session-a".into()),
            source: SessionSource::File {
                relative_path: "projects/project/session-a.jsonl".into(),
            },
        };
        let sessions = scan_sessions(root.path(), &template).unwrap();
        let meta =
            sessions.into_iter().find(|meta| meta.identity.session_id.0 == "session-a").unwrap();
        let mut registry = SourceRegistry::new(
            scope,
            "host".into(),
            "native".into(),
            SessionProvider::Claude,
            vec![root.path().into()],
        )
        .unwrap();
        registry.register(&meta, file.clone()).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_usage_store(&mut connection).unwrap();
        initialize_usage_cursors(&connection).unwrap();
        Self { _root: root, file, connection, registry, identity: meta.identity }
    }
    fn scan(&mut self) -> vw_agent_apps::usage::cursors::UsageScanOutcome {
        scan_session(&mut self.connection, &self.registry, &self.identity).unwrap()
    }
    fn append(&self, rows: &str) {
        let mut file = std::fs::OpenOptions::new().append(true).open(&self.file).unwrap();
        file.write_all(rows.as_bytes()).unwrap();
    }
    fn records(&self) -> Vec<StoredUsageRecord> {
        let mut statement =
            self.connection.prepare("SELECT payload_json FROM usage_records").unwrap();
        statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .map(|row| serde_json::from_str(&row.unwrap()).unwrap())
            .collect()
    }
}
const PARENT: &str = "00000000-0000-4000-8000-000000000001";
fn user() -> String {
    json!({"type":"user","sessionId":"session-a","uuid":PARENT,"timestamp":"2026-11-05T00:00:00Z","message":{"role":"user","content":"hello"}}).to_string() + "\n"
}
fn assistant(id: &str, time: &str) -> Value {
    json!({"type":"assistant","sessionId":"session-a","parentUuid":PARENT,"apiBlockIndex":0,"timestamp":time,
        "message":{"id":id,"role":"assistant","model":"claude-test","stop_reason":"end_turn","content":[{"type":"text","text":"answer"}],"usage":{"input_tokens":10,"output_tokens":5}}})
}
fn line(row: Value) -> String {
    row.to_string() + "\n"
}

#[test]
fn usage17_claude_scan_repeated_scan_and_tail_rewrite_do_not_recount() {
    let row = assistant("request-a", "2026-11-05T00:00:01Z");
    let mut fixture = Fixture::new(&(user() + &line(row.clone())));
    assert_eq!(fixture.scan().records, 1);
    assert_eq!(fixture.scan().records, 0);
    let mut revised = row;
    revised["message"]["usage"]["output_tokens"] = json!(99);
    std::fs::write(&fixture.file, user() + &line(revised)).unwrap();
    assert!(fixture.scan().history_changed);
    assert_eq!(fixture.records().len(), 1);
    fixture.append(&line(assistant("request-b", "2026-11-05T00:00:02Z")));
    fixture.scan();
    assert_eq!(fixture.records().len(), 2);
}

#[test]
fn usage17_claude_scan_bad_rows_and_partial_tail_resume_from_complete_boundary() {
    let mut fixture = Fixture::new(&user());
    fixture.scan();
    let row = line(assistant("request-a", "2026-11-05T00:00:01Z"));
    let split = row.len() / 2;
    fixture.append(&("invalid json\n".to_owned() + &row[..split]));
    let partial = fixture.scan();
    assert_eq!(partial.records, 0);
    fixture.append(&row[split..]);
    let complete = fixture.scan();
    assert!(complete.cursor.byte_offset > partial.cursor.byte_offset);
    let records = fixture.records();
    assert_eq!(records.len(), 1);
    let performance = &records[0].record.performance;
    assert_eq!(performance.latency_ms.as_ref().unwrap().value, 1000.0);
    assert_eq!(performance.latency_ms.as_ref().unwrap().kind, UsageMeasurementKind::Estimated);
    assert!(performance.first_token_ms.is_none());
    assert!(performance.output_tokens_per_second.is_none());
}

#[test]
fn usage17_claude_scan_rejects_untrusted_durations() {
    for time in ["2026-11-04T23:59:59Z", "2026-11-05T00:00:00.099Z", "2026-11-05T01:00:00.001Z"] {
        let mut fixture = Fixture::new(&(user() + &line(assistant("request-a", time))));
        fixture.scan();
        assert!(fixture.records()[0].record.performance.latency_ms.is_none(), "{time}");
    }
    let mut fixture =
        Fixture::new(&(user() + &line(assistant("request-a", "2026-11-05T00:00:00.100Z"))));
    fixture.scan();
    assert_eq!(fixture.records()[0].record.performance.latency_ms.as_ref().unwrap().value, 100.0);
}

#[test]
fn usage17_claude_scan_selects_final_snapshot_and_marks_thinking_estimates() {
    let mut first = assistant("request-a", "2026-11-05T00:00:00.500Z");
    first["message"]["stop_reason"] = Value::Null;
    first["message"]["usage"]["output_tokens"] = json!(1);
    first["message"]["content"] = json!([{"type":"thinking","thinking":"internal"}]);
    first["thinkingDurationMs"] = json!(300);
    let mut last = assistant("request-a", "2026-11-05T00:00:01Z");
    last["apiBlockIndex"] = json!(1);
    let mut fixture = Fixture::new(&(user() + &line(first) + &line(last)));
    fixture.scan();
    let records = fixture.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record.tokens.as_ref().unwrap().output, Some(5));
    let performance = &records[0].record.performance;
    assert_eq!(performance.first_token_ms.as_ref().unwrap().value, 200.0);
    let speed = performance.output_tokens_per_second.as_ref().unwrap();
    assert_eq!(speed.value, 6.25);
    assert_eq!(speed.kind, UsageMeasurementKind::Estimated);
}

#[test]
fn usage17_claude_scan_rejects_unregistered_identity_before_reading() {
    let mut fixture = Fixture::new(&user());
    let mut forged = fixture.identity.clone();
    forged.target_host = "another-host".into();
    assert!(scan_session(&mut fixture.connection, &fixture.registry, &forged).is_err());
    assert!(fixture.records().is_empty());
}
