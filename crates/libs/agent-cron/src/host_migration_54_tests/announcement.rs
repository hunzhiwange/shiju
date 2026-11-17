use super::*;
use config_channel_types::MattermostConfig;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener, time::Duration};
async fn deliver_announcement(config: &Config, channel: &str, target: &str, output: &str) -> anyhow::Result<()> {
    crate::announcement::deliver_announcement(&TestHost, config, channel, target, output).await
}

/// 测试投递失败在非尽力模式下标记任务错误
///
/// 验证 DeliveryConfig.best_effort=false 的行为：
/// - 任务本身成功但投递失败时，整体标记为失败
/// - 任务保持启用状态以便重试
/// - 运行记录中状态为 "error"
#[tokio::test]
async fn wb_schedule_feedback_persist_job_result_delivery_failure_non_best_effort_marks_error() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    // 创建带有投递配置的任务，best_effort 为 false
    let job = cron::add_agent_job(
        &config,
        Some("announce-job".into()),
        Schedule::Cron { expr: "*/5 * * * *".into(), tz: None },
        "deliver this",
        SessionTarget::Isolated,
        None,
        Some(DeliveryConfig {
            mode: "announce".into(),
            channel: Some("telegram".into()),
            to: Some("123456".into()),
            best_effort: false,
        }),
        false,
    )
    .unwrap();
    let started = Utc::now();
    let finished = started + ChronoDuration::milliseconds(10);

    // 由于没有配置 Telegram，投递会失败，且 best_effort=false
    let success = persist_job_result(&config, &job, true, "ok", started, finished).await;
    assert!(!success);

    let updated = cron::get_job(&config, &job.id).unwrap();
    assert!(updated.enabled);
    assert_eq!(updated.last_status.as_deref(), Some("error"));

    let runs = cron::list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, "error");
    assert!(runs[0].output.as_deref().unwrap().contains("telegram channel not configured"));
}

/// 测试投递失败在尽力模式下保持成功
///
/// 验证 DeliveryConfig.best_effort=true 的行为：
/// - 任务本身成功时，投递失败不影响整体结果
/// - 任务状态保持为成功
/// - 适用于非关键通知场景
#[tokio::test]
async fn wb_schedule_feedback_persist_job_result_delivery_failure_best_effort_keeps_success() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    // 创建带有投递配置的任务，best_effort 为 true
    let job = cron::add_agent_job(
        &config,
        Some("announce-job-best-effort".into()),
        Schedule::Cron { expr: "*/5 * * * *".into(), tz: None },
        "deliver this",
        SessionTarget::Isolated,
        None,
        Some(DeliveryConfig {
            mode: "announce".into(),
            channel: Some("telegram".into()),
            to: Some("123456".into()),
            best_effort: true,
        }),
        false,
    )
    .unwrap();
    let started = Utc::now();
    let finished = started + ChronoDuration::milliseconds(10);

    let success = persist_job_result(&config, &job, true, "ok", started, finished).await;
    assert!(success);

    let updated = cron::get_job(&config, &job.id).unwrap();
    assert!(updated.enabled);
    assert_eq!(updated.last_status.as_deref(), Some("ok"));

    let runs = cron::list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, "ok");
    assert!(runs[0].output.as_deref().unwrap().contains("telegram channel not configured"));
}

/// 测试 deliver_if_configured 处理无配置和无效通道
///
/// 验证投递函数的健壮性：
/// - 无投递配置时返回 Ok
/// - 无效通道时返回错误
/// - 错误消息清晰指明原因
#[tokio::test]
async fn deliver_if_configured_handles_none_and_invalid_channel() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let mut job = test_job("echo ok");

    // 无投递配置时应该成功
    assert!(deliver_if_configured(&config, &job, "x").await.is_ok());

    // 配置无效通道时应该失败
    job.delivery = DeliveryConfig {
        mode: "announce".into(),
        channel: Some("invalid".into()),
        to: Some("target".into()),
        best_effort: true,
    };
    let err = deliver_if_configured(&config, &job, "x").await.unwrap_err();
    assert!(err.to_string().contains("unsupported delivery channel"));
}

#[tokio::test]
async fn wb_schedule_feedback_rejects_each_unconfigured_known_channel() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let cases = [
        ("TELEGRAM", "telegram channel not configured"),
        ("discord", "discord channel not configured"),
        ("slack", "slack channel not configured"),
        ("mattermost", "mattermost channel not configured"),
        ("qq", "qq channel not configured"),
        ("email", "email channel not configured"),
    ];

    for (channel, expected) in cases {
        let err = deliver_announcement(&config, channel, "target", "output").await.unwrap_err();
        assert!(err.to_string().contains(expected), "unexpected error for {channel}: {err}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn deliver_announcement_sends_configured_mattermost_thread_message() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    let (base_url, request) = spawn_http_server("201 Created", r#"{"id":"post-1"}"#).await;
    config.channels_config.mattermost = Some(MattermostConfig {
        url: base_url,
        bot_token: "mattermost-token".into(),
        channel_id: Some("configured-channel".into()),
        allowed_users: vec!["*".into()],
        thread_replies: Some(false),
        mention_only: Some(true),
        group_reply: None,
    });

    deliver_announcement(&config, "MatterMost", "channel-a:root-b", "hello mattermost")
        .await
        .unwrap();
    let request = request.await.unwrap();

    assert!(request.starts_with("POST /api/v4/posts HTTP/1.1"));
    let (headers, _) = request.split_once("\r\n\r\n").unwrap();
    // HTTP header names are case-insensitive; credential values remain exact.
    let authorization = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .map(|(_, value)| value.trim());
    assert_eq!(authorization, Some("Bearer mattermost-token"));
    assert!(request.contains(r#""channel_id":"channel-a""#));
    assert!(request.contains(r#""root_id":"root-b""#));
    assert!(request.contains(r#""message":"hello mattermost""#));
}

#[cfg(not(target_arch = "wasm32"))]
async fn spawn_http_server(status: &str, body: &str) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let status = status.to_string();
    let body = body.to_string();

    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];

        loop {
            let read = socket.read(&mut buffer).await.unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            if http_request_is_complete(&request) {
                break;
            }
        }

        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8_lossy(&request).to_string()
    });

    (format!("http://{address}"), handle)
}

#[tokio::test]
async fn deliver_if_configured_treats_announce_mode_case_insensitively() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let mut job = test_job("echo ok");
    job.delivery = DeliveryConfig {
        mode: "AnNoUnCe".into(),
        channel: Some("invalid".into()),
        to: Some("target".into()),
        best_effort: true,
    };

    let err = deliver_if_configured(&config, &job, "output").await.unwrap_err();
    assert!(err.to_string().contains("unsupported delivery channel"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn cron_history_scheduled_run_is_visible_until_completion() {
    use crate::active_runs::snapshot;
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.autonomy.allowed_commands.push("curl".into());
    config.autonomy.block_high_risk_commands = false;
    let (listener, command) = shell_barrier().await;
    let delivery_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    config.channels_config.mattermost = Some(MattermostConfig {
        url: format!("http://{}", delivery_listener.local_addr().unwrap()),
        bot_token: "test-only".into(),
        channel_id: None,
        allowed_users: vec![],
        thread_replies: None,
        mention_only: None,
        group_reply: None,
    });
    let job = crate::add_job(&config, "*/5 * * * *", &command).unwrap();
    let job = cron::update_job(
        &config,
        &job.id,
        cron::CronJobPatch {
            delivery: Some(DeliveryConfig {
                mode: "announce".into(),
                channel: Some("mattermost".into()),
                to: Some("test-channel".into()),
                best_effort: false,
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);
    let execution = execute_and_persist_job(&config, &security, &job, "active-run-test");
    let mut execution = Box::pin(execution);
    let stream = tokio::select! {
        accepted = listener.accept() => accepted.unwrap().0,
        _ = &mut execution => panic!("execution ended before reaching the shell barrier"),
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => panic!("shell did not start"),
    };
    let active = snapshot(&config);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].job_id, job.id);
    assert!(crate::list_runs(&config, &job.id, 10).unwrap().is_empty());
    release_shell(stream).await;
    let mut delivery_stream = tokio::select! {
        accepted = delivery_listener.accept() => accepted.unwrap().0,
        _ = &mut execution => panic!("execution ended before delivery"),
        _ = tokio::time::sleep(Duration::from_secs(10)) => panic!("delivery did not start"),
    };
    assert_eq!(snapshot(&config), active);
    assert!(cron::list_runs(&config, &job.id, 10).unwrap().is_empty());
    let mut request = Vec::new();
    let mut buffer = [0; 1024];
    tokio::select! {
        _ = async {
            while !http_request_is_complete(&request) {
                let read = delivery_stream.read(&mut buffer).await.unwrap();
                assert_ne!(read, 0);
                request.extend_from_slice(&buffer[..read]);
            }
        } => {},
        _ = &mut execution => panic!("execution ended before delivery response"),
        _ = tokio::time::sleep(Duration::from_secs(10)) => panic!("delivery request did not arrive"),
    }
    delivery_stream.write_all(b"HTTP/1.1 201 Created\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"id\":\"post-1\"}").await.unwrap();
    assert!(execution.await.1);
    assert!(snapshot(&config).is_empty());
    let runs = crate::list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].started_at, active[0].started_at);
}

#[tokio::test]
async fn wb_schedule_feedback_unavailable_credentials_rejected() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp).await;
    config.channels_config.unavailable_channels.insert("slack:default".into());
    let err = deliver_announcement(&config, "slack", "target", "report").await.unwrap_err();
    assert!(err.to_string().contains("credentials unavailable"));
}

#[cfg(not(target_arch = "wasm32"))]
fn http_request_is_complete(request: &[u8]) -> bool {
    let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let body_start = header_end + 4;
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);

    request.len() >= body_start + content_length
}

// A local HTTP connection blocks the real shell command until the test releases it.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn shell_barrier() -> (tokio::net::TcpListener, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let command = format!(
        "curl --noproxy '*' --silent --max-time 10 http://{}/",
        listener.local_addr().unwrap()
    );
    (listener, command)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn release_shell(mut stream: tokio::net::TcpStream) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut request = [0; 4096];
    stream.read(&mut request).await.unwrap();
    stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
        .await
        .unwrap();
}
