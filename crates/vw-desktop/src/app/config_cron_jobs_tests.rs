use std::ffi::OsStr;
use std::sync::{Arc, LazyLock};

use serde_json::Value;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use vw_config_types::ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};

const SOURCE: &str = include_str!("config_cron_jobs.rs");
static CRON_JOBS_ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Debug, Clone)]
struct RecordedRequest {
    method: String,
    path: String,
    body: Value,
}

struct EnvGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set_os(key: &'static str, value: &OsStr) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

struct TestGateway {
    _home: TempDir,
    _home_env: EnvGuard,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

impl TestGateway {
    async fn start(max_requests: usize) -> Self {
        let home = tempfile::tempdir().expect("temp home should be created");
        let home_env = EnvGuard::set_os("HOME", home.path().as_os_str());
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("test gateway should bind");
        let port = listener.local_addr().expect("test gateway address").port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        tokio::spawn(async move {
            for _ in 0..max_requests {
                let Ok((mut stream, _addr)) = listener.accept().await else {
                    break;
                };
                let mut buffer = vec![0; 16 * 1024];
                let Ok(read) = stream.read(&mut buffer).await else {
                    continue;
                };
                let raw = String::from_utf8_lossy(&buffer[..read]);
                let mut lines = raw.lines();
                let Some(request_line) = lines.next() else {
                    continue;
                };
                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap_or_default().to_string();
                let path = parts.next().unwrap_or_default().to_string();
                let content_length = raw
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("content-length: ")
                            .or_else(|| line.strip_prefix("Content-Length: "))
                    })
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                let body =
                    raw.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or_default().as_bytes();
                let body = if content_length == 0 {
                    Value::Null
                } else {
                    serde_json::from_slice(&body[..body.len().min(content_length)])
                        .unwrap_or(Value::Null)
                };
                server_requests.lock().await.push(RecordedRequest {
                    method: method.clone(),
                    path: path.clone(),
                    body,
                });
                let response = response_for(&method, &path);
                let _ = stream.write_all(response.as_bytes()).await;
            }
        });

        let mut config = GatewayClientSystemSettingsConfig::default();
        config.set_servers(
            vec![GatewayClientServerConfig {
                id: "test".to_string(),
                name: "Test".to_string(),
                host: "127.0.0.1".to_string(),
                port,
                bearer_token: "test-token".to_string(),
                ..GatewayClientServerConfig::default()
            }],
            "test".to_string(),
        );
        super::super::system_settings::save_gateway_client_bootstrap_config(&config);

        Self { _home: home, _home_env: home_env, requests }
    }

    async fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().await.clone()
    }
}

fn response_for(method: &str, path: &str) -> String {
    let (status, body) = match (method, path) {
        ("GET", "/v1/cron") => (
            "200 OK",
            r#"{"jobs":[{"id":"job-1","name":"Nightly","job_type":"shell","schedule_kind":"cron","expression":"0 0 * * * *","at":null,"every_ms":null,"command":"echo ok","prompt":null,"model":null,"agent":null,"acp_agent":null,"project_path":null,"wake":false,"fallbacks":[],"full_access":false,"task_pool":false,"delivery_mode":"none","delivery_channel":null,"delivery_to":null,"delivery_best_effort":true,"delete_after_run":false,"next_run":"later","last_run":"now","last_status":"ok","last_output":"done","enabled":true}]}"#,
        ),
        ("GET", "/v1/cron/job-1/runs") => (
            "200 OK",
            r#"{"runs":[{"id":7,"job_id":"job-1","started_at":"start","finished_at":"finish","status":"ok","output":"done","duration_ms":5}]}"#,
        ),
        ("GET", "/v1/cron/runs/__probe__") => ("200 OK", r#"{"runs":[]}"#),
        ("POST", "/v1/cron") | ("PATCH", "/v1/cron/job-1") | ("PATCH", "/v1/cron/job-2") => {
            ("200 OK", r#"{"status":"ok","job":null}"#)
        }
        ("DELETE", "/v1/cron/job-1") | ("DELETE", "/v1/cron/job-2") => ("204 No Content", ""),
        _ => ("404 Not Found", r#"{"error":"missing"}"#),
    };
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn source_declares_symbol(name: &str) -> bool {
    let needles = [
        format!("fn {name}"),
        format!("pub fn {name}"),
        format!("struct {name}"),
        format!("pub struct {name}"),
        format!("enum {name}"),
        format!("pub enum {name}"),
        format!("type {name}"),
        format!("pub type {name}"),
        format!("const {name}"),
        format!("pub const {name}"),
        format!("static {name}"),
        format!("pub static {name}"),
        format!("impl {name}"),
    ];

    needles.iter().any(|needle| SOURCE.contains(needle))
}

fn assert_error_contains(result: Result<(), String>, expected: &str) {
    let err = result.expect_err("call should fail before gateway request");
    assert!(err.contains(expected), "expected {err:?} to contain {expected:?}");
}

#[test]
fn config_cron_jobs_tests_keeps_planned_coverage_targets() {
    for name in [
        "normalize_schedule_kind",
        "legacy_schedule_expression",
        "requires_extended_cron_api",
        "ensure_extended_cron_api_if_needed",
        "load_cron_jobs_async",
        "load_cron_job_runs_async",
        "add_cron_job_async",
        "update_cron_job_async",
        "set_cron_job_enabled_async",
        "set_cron_jobs_enabled_async",
        "delete_cron_job_async",
        "delete_cron_jobs_async",
    ] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}

#[test]
fn normalize_schedule_kind_accepts_aliases_and_infers_legacy_cron() {
    assert_eq!(super::normalize_schedule_kind("", None, None, None), Ok("cron".to_string()));
    assert_eq!(
        super::normalize_schedule_kind("指定时间", None, Some("2026-01-01T00:00:00Z"), None),
        Ok("at".to_string())
    );
    assert_eq!(
        super::normalize_schedule_kind("固定间隔", None, None, Some(1_000)),
        Ok("every".to_string())
    );
    assert_eq!(
        super::normalize_schedule_kind("cron", Some("0 * * * * *"), None, Some(2_000)),
        Ok("cron".to_string())
    );
    assert_eq!(
        super::normalize_schedule_kind("cron", None, None, Some(2_000)),
        Ok("every".to_string())
    );
    assert_eq!(
        super::normalize_schedule_kind("cron", None, Some("2026-01-01T00:00:00Z"), None),
        Ok("at".to_string())
    );
}

#[test]
fn normalize_schedule_kind_rejects_unknown_kind() {
    let err = super::normalize_schedule_kind("weekly", None, None, None)
        .expect_err("unknown schedule kind should fail");
    assert!(err.contains("不支持的调度类型"));
}

#[test]
fn legacy_schedule_expression_preserves_cron_and_clamps_every_seconds() {
    assert_eq!(super::legacy_schedule_expression("cron", Some("0 1 * * * *"), None), "0 1 * * * *");
    assert_eq!(super::legacy_schedule_expression("at", None, None), "0 * * * * *");
    assert_eq!(super::legacy_schedule_expression("every", None, Some(1)), "*/1 * * * * *");
    assert_eq!(super::legacy_schedule_expression("every", None, Some(61_000)), "*/59 * * * * *");
    assert_eq!(super::legacy_schedule_expression("every", None, Some(1_500)), "*/2 * * * * *");
}

#[test]
fn requires_extended_cron_api_only_for_non_legacy_fields() {
    assert!(!super::requires_extended_cron_api(
        "shell", "cron", None, None, None, None, None, false
    ));
    assert!(super::requires_extended_cron_api(
        "agent", "cron", None, None, None, None, None, false
    ));
    assert!(super::requires_extended_cron_api(
        "shell", "every", None, None, None, None, None, false
    ));
    assert!(super::requires_extended_cron_api(
        "shell",
        "cron",
        None,
        None,
        None,
        Some(&"openai/gpt".to_string()),
        None,
        false
    ));
    assert!(super::requires_extended_cron_api(
        "shell",
        "cron",
        None,
        None,
        None,
        None,
        Some(&vec!["backup".to_string()]),
        false,
    ));
    assert!(super::requires_extended_cron_api("shell", "cron", None, None, None, None, None, true));
}

#[tokio::test(flavor = "multi_thread")]
async fn add_cron_job_async_rejects_invalid_inputs_before_gateway() {
    assert_error_contains(
        super::add_cron_job_async(
            String::new(),
            "shell".to_string(),
            "cron".to_string(),
            String::new(),
            String::new(),
            String::new(),
            "echo ok".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            false,
            String::new(),
            String::new(),
            false,
            false,
            false,
            String::new(),
            String::new(),
            true,
            false,
        )
        .await,
        "Cron 表达式不能为空",
    );
    assert_error_contains(
        super::add_cron_job_async(
            String::new(),
            "shell".to_string(),
            "every".to_string(),
            String::new(),
            String::new(),
            "abc".to_string(),
            "echo ok".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            false,
            String::new(),
            String::new(),
            false,
            false,
            false,
            String::new(),
            String::new(),
            true,
            false,
        )
        .await,
        "固定间隔毫秒数无效",
    );
    assert_error_contains(
        super::add_cron_job_async(
            String::new(),
            "agent".to_string(),
            "at".to_string(),
            String::new(),
            "2026-01-01T00:00:00Z".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            false,
            String::new(),
            String::new(),
            false,
            false,
            false,
            String::new(),
            String::new(),
            true,
            false,
        )
        .await,
        "Agent 提示词不能为空",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn update_cron_job_async_rejects_missing_delivery_fields_before_gateway() {
    assert_error_contains(
        super::update_cron_job_async(
            "job-1".to_string(),
            String::new(),
            "shell".to_string(),
            "cron".to_string(),
            "0 0 * * * *".to_string(),
            String::new(),
            String::new(),
            "echo ok".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            false,
            String::new(),
            String::new(),
            false,
            false,
            true,
            String::new(),
            "ops".to_string(),
            true,
            false,
        )
        .await,
        "投递通道不能为空",
    );
    assert_error_contains(
        super::update_cron_job_async(
            "job-1".to_string(),
            String::new(),
            "shell".to_string(),
            "cron".to_string(),
            "0 0 * * * *".to_string(),
            String::new(),
            String::new(),
            "echo ok".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            false,
            String::new(),
            String::new(),
            false,
            false,
            true,
            "email".to_string(),
            String::new(),
            true,
            false,
        )
        .await,
        "投递目标不能为空",
    );
}

#[tokio::test]
async fn update_cron_job_async_rejects_schedule_and_payload_requirements_before_gateway() {
    assert_error_contains(
        super::update_cron_job_async(
            "job-1".to_string(),
            String::new(),
            "shell".to_string(),
            "at".to_string(),
            String::new(),
            String::new(),
            String::new(),
            "echo ok".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            false,
            String::new(),
            String::new(),
            false,
            false,
            false,
            String::new(),
            String::new(),
            true,
            false,
        )
        .await,
        "指定时间不能为空",
    );
    assert_error_contains(
        super::update_cron_job_async(
            "job-1".to_string(),
            String::new(),
            "shell".to_string(),
            "every".to_string(),
            String::new(),
            String::new(),
            String::new(),
            "echo ok".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            false,
            String::new(),
            String::new(),
            false,
            false,
            false,
            String::new(),
            String::new(),
            true,
            false,
        )
        .await,
        "固定间隔毫秒数不能为空",
    );
    assert_error_contains(
        super::update_cron_job_async(
            "job-1".to_string(),
            String::new(),
            "shell".to_string(),
            "cron".to_string(),
            "0 0 * * * *".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            false,
            String::new(),
            String::new(),
            false,
            false,
            false,
            String::new(),
            String::new(),
            true,
            false,
        )
        .await,
        "执行命令不能为空",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn load_cron_jobs_and_runs_use_gateway_responses() {
    let _guard = CRON_JOBS_ENV_LOCK.lock().await;
    let gateway = TestGateway::start(2).await;

    let jobs = super::load_cron_jobs_async().await.expect("jobs should load");
    let runs =
        super::load_cron_job_runs_async("job-1".to_string()).await.expect("runs should load");

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, "job-1");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, "ok");
    let requests = gateway.requests().await;
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/v1/cron");
    assert_eq!(requests[1].path, "/v1/cron/job-1/runs");
}

#[tokio::test(flavor = "multi_thread")]
async fn add_cron_job_async_sends_trimmed_agent_request() {
    let _guard = CRON_JOBS_ENV_LOCK.lock().await;
    let gateway = TestGateway::start(2).await;

    super::add_cron_job_async(
        "  Agent job ".to_string(),
        "agent".to_string(),
        "every".to_string(),
        String::new(),
        String::new(),
        "1500".to_string(),
        " ignored ".to_string(),
        "  summarize ".to_string(),
        " session-a ".to_string(),
        " main ".to_string(),
        " claude ".to_string(),
        " /tmp/project ".to_string(),
        true,
        " openai/gpt ".to_string(),
        " fallback-a,\nfallback-b ".to_string(),
        true,
        true,
        true,
        " email ".to_string(),
        " ops@example.com ".to_string(),
        false,
        true,
    )
    .await
    .expect("agent cron should be submitted");

    let requests = gateway.requests().await;
    assert_eq!(requests[0].path, "/v1/cron/runs/__probe__");
    assert_eq!(requests[1].method, "POST");
    assert_eq!(requests[1].path, "/v1/cron");
    let body = &requests[1].body;
    assert_eq!(body["name"], "Agent job");
    assert_eq!(body["job_type"], "agent");
    assert_eq!(body["schedule_kind"], "every");
    assert_eq!(body["schedule"], "*/2 * * * * *");
    assert_eq!(body["every_ms"], 1500);
    assert_eq!(body["prompt"], "summarize");
    assert_eq!(body["agent"], "main");
    assert_eq!(body["acp_agent"], "claude");
    assert_eq!(body["project_path"], "/tmp/project");
    assert_eq!(body["fallbacks"], serde_json::json!(["fallback-a", "fallback-b"]));
    assert_eq!(body["task_pool"], true);
    assert_eq!(body["delivery_mode"], "announce");
}

#[tokio::test(flavor = "multi_thread")]
async fn update_cron_job_async_sends_cron_request_without_extended_probe() {
    let _guard = CRON_JOBS_ENV_LOCK.lock().await;
    let gateway = TestGateway::start(1).await;

    super::update_cron_job_async(
        "job-1".to_string(),
        " Shell job ".to_string(),
        "shell".to_string(),
        "cron".to_string(),
        " 0 0 * * * * ".to_string(),
        String::new(),
        String::new(),
        " echo ok ".to_string(),
        String::new(),
        " session-a ".to_string(),
        " main ".to_string(),
        " claude ".to_string(),
        String::new(),
        false,
        String::new(),
        String::new(),
        false,
        true,
        false,
        String::new(),
        String::new(),
        true,
        false,
    )
    .await
    .expect("shell cron should be updated");

    let requests = gateway.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "PATCH");
    assert_eq!(requests[0].path, "/v1/cron/job-1");
    assert_eq!(requests[0].body["name"], "Shell job");
    assert_eq!(requests[0].body["schedule"], "0 0 * * * *");
    assert_eq!(requests[0].body["command"], "echo ok");
    assert_eq!(requests[0].body["task_pool"], false);
    assert!(requests[0].body.get("agent").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn enabled_and_delete_helpers_send_one_request_per_job() {
    let _guard = CRON_JOBS_ENV_LOCK.lock().await;
    let gateway = TestGateway::start(4).await;

    super::set_cron_job_enabled_async("job-1".to_string(), false)
        .await
        .expect("single enable update should succeed");
    super::set_cron_jobs_enabled_async(vec!["job-1".to_string(), "job-2".to_string()], true)
        .await
        .expect("batch enable update should succeed");
    super::delete_cron_job_async("job-1".to_string()).await.expect("delete should succeed");

    let requests = gateway.requests().await;
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[0].method, "PATCH");
    assert_eq!(requests[0].body["enabled"], false);
    assert_eq!(requests[1].body["enabled"], true);
    assert_eq!(requests[2].path, "/v1/cron/job-2");
    assert_eq!(requests[3].method, "DELETE");
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_cron_jobs_async_deletes_each_job() {
    let _guard = CRON_JOBS_ENV_LOCK.lock().await;
    let gateway = TestGateway::start(2).await;

    super::delete_cron_jobs_async(vec!["job-1".to_string(), "job-2".to_string()])
        .await
        .expect("batch delete should succeed");

    let requests = gateway.requests().await;
    assert_eq!(requests[0].method, "DELETE");
    assert_eq!(requests[0].path, "/v1/cron/job-1");
    assert_eq!(requests[1].path, "/v1/cron/job-2");
}
