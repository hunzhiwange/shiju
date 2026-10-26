//! Real native entry regressions; the twenty-minute test is explicitly opt-in.
use crate::tools::traits::ToolCallResult;
use crate::tools::{ToolRuntimeContext, ToolUseContext, execute_tool_call};
use serde_json::{Value, json};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

fn context(root: &Path, session: &str) -> ToolRuntimeContext {
    let root = Some(root.to_string_lossy().into_owned());
    let tools = ToolUseContext::new(session, root.clone()).with_full_access_enabled(true);
    ToolRuntimeContext::new(session, root).with_tool_use_context(tools)
}

fn call(root: &Path, session: &str, name: &str, args: Value) -> ToolCallResult {
    // Every call rebuilds context, registry and the synchronous bridge runtime.
    execute_tool_call(name, &args.to_string(), &context(root, session)).unwrap()
}

struct ProcessCleanup<'a> {
    root: &'a Path,
    session: &'a str,
    id: String,
}

impl Drop for ProcessCleanup<'_> {
    fn drop(&mut self) {
        let _ = execute_tool_call(
            "bash_cancel",
            &json!({"process_id": self.id}).to_string(),
            &context(self.root, self.session),
        );
    }
}

fn start<'a>(
    root: &'a Path,
    session: &'a str,
    script: &str,
) -> (ProcessCleanup<'a>, ToolCallResult) {
    std::fs::write(root.join("fixture.sh"), script).unwrap();
    let result = call(
        root,
        session,
        "bash",
        json!({
            "command": "sh fixture.sh", "description": "real native process regression",
            "yield_time_ms": 250
        }),
    );
    let cleanup = ProcessCleanup {
        root,
        session,
        id: result.data["process_id"].as_str().unwrap().to_owned(),
    };
    (cleanup, result)
}

fn poll(process: &ProcessCleanup<'_>, cursor: u64, yield_ms: u64) -> ToolCallResult {
    call(
        process.root,
        process.session,
        "bash_wait",
        json!({
            "process_id": process.id, "cursor": cursor, "yield_time_ms": yield_ms
        }),
    )
}

fn append(result: &ToolCallResult, output: &mut String, cursor: &mut u64) {
    assert_eq!(result.data["truncated"], false);
    assert_eq!(result.data["dropped_bytes"], 0);
    let next = result.data["next_cursor"].as_u64().unwrap();
    assert!(next >= *cursor);
    output.push_str(result.data["output"].as_str().unwrap());
    *cursor = next;
}

#[test]
#[ignore = "uses a real native process for at least twenty minutes"]
fn shell_long_running_real_entry_survives_twenty_minutes() {
    let root = tempfile::tempdir().unwrap();
    let wall_start = SystemTime::now();
    let start_time = Instant::now();
    let (process, mut result) = start(
        root.path(),
        "long-duration",
        r#"
i=0
while [ "$i" -lt 40 ]; do
    printf 'marker:%s\n' "$i"
    sleep 30
    i=$((i + 1))
done
printf 'marker:40\n'
"#,
    );
    let first_response = start_time.elapsed();
    eprintln!("start={wall_start:?} process={} first_response={first_response:?}", process.id);
    assert!(first_response < Duration::from_secs(30));
    assert_eq!(result.data["state"], "running");
    assert!(result.data["exit_code"].is_null());
    let mut output = String::new();
    let mut cursor = 0;
    loop {
        let previous = cursor;
        append(&result, &mut output, &mut cursor);
        eprintln!(
            "elapsed={:?} cursor={previous}->{cursor} state={}",
            start_time.elapsed(),
            result.data["state"]
        );
        if result.data["state"] != "running" {
            break;
        }
        assert!(
            start_time.elapsed() < Duration::from_secs(1320),
            "controlled process exceeded bounded deadline"
        );
        result = poll(&process, cursor, 30_000);
    }
    let elapsed = start_time.elapsed();
    eprintln!(
        "end={:?} elapsed={elapsed:?} state={} exit_code={}",
        SystemTime::now(),
        result.data["state"],
        result.data["exit_code"]
    );
    assert!(elapsed >= Duration::from_secs(1200));
    assert_eq!(result.data["state"], "exited");
    assert_eq!(result.data["exit_code"], 0);
    let expected: String = (0..=40).map(|i| format!("marker:{i}\n")).collect();
    assert_eq!(output, expected);
    assert!(cursor > 0);
    let tail = poll(&process, cursor, 250);
    assert_eq!(tail.data["output"], "");
    assert_eq!(tail.data["state"], "exited");
    assert_eq!(tail.data["next_cursor"], cursor);
}

fn wait_until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(Instant::now() < deadline, "real process handshake/reaping deadline exceeded");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn read_pid(root: &Path, name: &str) -> i32 {
    let path = root.join(name);
    wait_until(|| std::fs::read_to_string(&path).is_ok_and(|s| s.trim().parse::<i32>().is_ok()));
    std::fs::read_to_string(path).unwrap().trim().parse().unwrap()
}

fn assert_gone(pid: i32) {
    wait_until(|| unsafe { libc::kill(pid, 0) } == -1);
    assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
}

#[test]
fn shell_long_running_real_entry_can_cancel() {
    let root = tempfile::tempdir().unwrap();
    let (process, first) = start(
        root.path(),
        "long-cancel",
        r#"
echo $$ > parent
trap 'wait; printf cancel-tail; exit 9' TERM
sleep 300 &
echo $! > child
printf ready
wait
"#,
    );
    let parent = read_pid(root.path(), "parent");
    let child = read_pid(root.path(), "child");
    assert_eq!(unsafe { libc::kill(parent, 0) }, 0);
    assert_eq!(unsafe { libc::kill(child, 0) }, 0);
    let ready = poll(&process, 0, 250);
    assert_eq!(first.data["state"], "running");
    assert!(ready.data["output"].as_str().unwrap().contains("ready"));
    let result =
        call(root.path(), process.session, "bash_cancel", json!({"process_id": process.id}));
    assert_eq!(result.data["state"], "cancelled");
    assert_eq!(result.data["exit_code"], 9);
    assert!(result.data["output"].as_str().unwrap().contains("cancel-tail"));
    assert_gone(parent);
    assert_gone(child);
    let tail = poll(&process, 0, 250);
    assert_eq!(tail.data["state"], "cancelled");
    assert!(tail.data["output"].as_str().unwrap().contains("ready"));
    eprintln!(
        "process={} parent={parent} child={child} reaped; state=cancelled exit=9 tail retained",
        process.id
    );
}

#[test]
fn shell_long_running_rejects_cross_session_poll() {
    let root = tempfile::tempdir().unwrap();
    let other_root = tempfile::tempdir().unwrap();
    let (process, _) = start(root.path(), "long-owner", "printf ready; exec sleep 300");
    let ready = poll(&process, 0, 250);
    assert_eq!(ready.data["state"], "running");
    assert!(ready.data["output"].as_str().unwrap().contains("ready"));
    for name in ["bash_wait", "bash_cancel"] {
        let args = json!({"process_id":process.id,"yield_time_ms":250}).to_string();
        let unknown = execute_tool_call(
            name,
            &json!({"process_id":"unknown","yield_time_ms":250}).to_string(),
            &context(root.path(), process.session),
        )
        .unwrap_err()
        .to_string();
        for ctx in
            [context(root.path(), "foreign-session"), context(other_root.path(), process.session)]
        {
            let denied = execute_tool_call(name, &args, &ctx).unwrap_err().to_string();
            assert_eq!(denied, unknown);
        }
        let denied = crate::session::ui_store::with_tenant_scope_sync(
            vw_api_types::id::TenantScope::new("fixture-platform", "foreign-company").unwrap(),
            || {
                execute_tool_call(name, &args, &context(root.path(), process.session))
                    .unwrap_err()
                    .to_string()
            },
        );
        assert_eq!(denied, unknown);
    }
    let owner = poll(&process, 0, 250);
    assert_eq!(owner.data["state"], "running");
    assert!(owner.data["output"].as_str().unwrap().contains("ready"));
    let done = call(root.path(), process.session, "bash_cancel", json!({"process_id": process.id}));
    assert_eq!(done.data["state"], "cancelled");
    eprintln!(
        "process={} denied foreign session/workspace/tenant read and cancel; owner survived and reaped",
        process.id
    );
}
