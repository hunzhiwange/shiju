//! 通过真实 tmux 窗格验证终端调整尺寸后，历史和输入框依据源码重新流式布局。
//! 手动忽略测试覆盖高度拆分、重复缩放、宽度恢复与已滚动多行草稿四类回归。
//! fixture 使用本地 SSE 服务和隔离配置，避免依赖真实模型响应。

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use tempfile::tempdir;
use wiremock::MockServer;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires tmux and a locally built codex binary; run with --ignored for manual resize smoke"]
// 新会话被垂直拆分后，输入框行不应向下漂移，关闭拆分时还需回到原锚点。
async fn tmux_split_preserves_fresh_session_composer_row_after_resize_reflow() -> Result<()> {
    if cfg!(windows) {
        return Ok(());
    }
    skip_if_no_network!(Ok(()));
    if Command::new("tmux").arg("-V").output().is_err() {
        eprintln!("skipping resize smoke because tmux is unavailable");
        return Ok(());
    }

    let repo_root = codex_utils_cargo_bin::repo_root()?;
    let codex = codex_binary(&repo_root)?;
    let codex_home = tempdir()?;
    let server = MockServer::start().await;
    let _response_mock = responses::mount_sse_once(&server, resize_reflow_sse()).await;
    let openai_base_url_config = format!("openai_base_url=\"{}/v1\"", server.uri());
    write_config(codex_home.path(), &repo_root)?;
    write_auth(codex_home.path())?;

    let session_name = format!("codex-resize-reflow-smoke-{}", std::process::id());
    let _session = TmuxSession {
        name: session_name.clone(),
    };

    let prompt = "Say hi.";
    let start_output = checked_output(
        Command::new("tmux")
            .arg("new-session")
            .arg("-d")
            .arg("-P")
            .arg("-F")
            .arg("#{pane_id}")
            .arg("-x")
            .arg("120")
            .arg("-y")
            .arg("40")
            .arg("-s")
            .arg(&session_name)
            .arg("--")
            .arg("env")
            .arg(format!("CODEX_HOME={}", codex_home.path().display()))
            .arg("OPENAI_API_KEY=dummy")
            .arg(codex)
            .arg("-c")
            .arg("analytics.enabled=false")
            .arg("-c")
            .arg(&openai_base_url_config)
            .arg("--no-alt-screen")
            .arg("-C")
            .arg(&repo_root)
            .arg(prompt),
    )?;
    let codex_pane = stdout_text(&start_output).trim().to_string();
    anyhow::ensure!(!codex_pane.is_empty(), "tmux did not report a pane id");

    wait_for_capture_contains(
        &codex_pane,
        "resize reflow sentinel",
        Duration::from_secs(/*secs*/ 15),
    )?;
    wait_for_capture_contains(
        &codex_pane,
        "gpt-5.4 default",
        Duration::from_secs(/*secs*/ 15),
    )?;
    let draft = "Notice where we are here in terms of y location.";
    check(
        Command::new("tmux")
            .arg("send-keys")
            .arg("-t")
            .arg(&codex_pane)
            .arg("-l")
            .arg(draft),
    )?;
    let baseline_capture =
        wait_for_capture_contains(&codex_pane, draft, Duration::from_secs(/*secs*/ 15))?;
    let baseline_row = last_composer_row(&baseline_capture).context("composer row before split")?;
    let baseline_history_row = first_row_containing(&baseline_capture, "resize reflow sentinel")
        .context("history row before split")?;

    let split_output = checked_output(
        Command::new("tmux")
            .arg("split-window")
            .arg("-d")
            .arg("-P")
            .arg("-F")
            .arg("#{pane_id}")
            .arg("-v")
            .arg("-l")
            .arg("12")
            .arg("-t")
            .arg(&codex_pane)
            .arg("sleep")
            .arg("30"),
    )?;
    let split_pane = stdout_text(&split_output).trim().to_string();

    sleep(Duration::from_millis(/*millis*/ 250));
    let first_capture = capture_pane(&codex_pane)?;
    let first_row = last_composer_row(&first_capture).context("composer row after split")?;

    sleep(Duration::from_millis(/*millis*/ 1_000));
    let second_capture = capture_pane(&codex_pane)?;
    let second_row =
        last_composer_row(&second_capture).context("composer row after reflow wait")?;

    anyhow::ensure!(
        first_row == second_row,
        "composer row drifted after split: before={first_row}, after={second_row}\n\
         before:\n{first_capture}\n\
         after:\n{second_capture}"
    );
    anyhow::ensure!(
        second_row <= baseline_row + 1,
        "composer row snapped downward after split: baseline={baseline_row}, after={second_row}\n\
         baseline:\n{baseline_capture}\n\
         after:\n{second_capture}"
    );

    check(
        Command::new("tmux")
            .arg("kill-pane")
            .arg("-t")
            .arg(&split_pane),
    )?;

    sleep(Duration::from_millis(/*millis*/ 500));
    let final_capture = capture_pane(&codex_pane)?;
    let final_row =
        last_composer_row(&final_capture).context("composer row after closing split")?;
    anyhow::ensure!(
        final_row == baseline_row,
        "composer row drifted after closing split: baseline={baseline_row}, after={final_row}\n\
         capture:\n{final_capture}"
    );
    let final_history_row = first_row_containing(&final_capture, "resize reflow sentinel")
        .context("history row after closing split")?;
    anyhow::ensure!(
        final_history_row == baseline_history_row,
        "history row drifted after closing split: baseline={baseline_history_row}, \
         after={final_history_row}\n\
         baseline:\n{baseline_capture}\n\
         after:\n{final_capture}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires tmux and a locally built codex binary; run with --ignored for manual resize smoke"]
// 多轮缩小和恢复必须保持输入框及历史哨兵行稳定，防止误差逐次累积。
async fn tmux_repeated_resizes_do_not_push_composer_down() -> Result<()> {
    if cfg!(windows) {
        return Ok(());
    }
    skip_if_no_network!(Ok(()));
    if Command::new("tmux").arg("-V").output().is_err() {
        eprintln!("skipping resize smoke because tmux is unavailable");
        return Ok(());
    }

    run_repeated_resize_smoke().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires tmux and a locally built codex binary; run with --ignored for manual resize smoke"]
// 宽度缩小再恢复时，可见历史与输入框都应回到原来的行锚点。
async fn tmux_width_resize_restore_keeps_visible_content_anchored() -> Result<()> {
    if cfg!(windows) {
        return Ok(());
    }
    skip_if_no_network!(Ok(()));
    if Command::new("tmux").arg("-V").output().is_err() {
        eprintln!("skipping resize smoke because tmux is unavailable");
        return Ok(());
    }

    let repo_root = codex_utils_cargo_bin::repo_root()?;
    let codex = codex_binary(&repo_root)?;
    let codex_home = tempdir()?;
    let server = MockServer::start().await;
    let _response_mock = responses::mount_sse_once(&server, resize_reflow_sse()).await;
    let openai_base_url_config = format!("openai_base_url=\"{}/v1\"", server.uri());
    write_config(codex_home.path(), &repo_root)?;
    write_auth(codex_home.path())?;

    let session_name = format!("codex-resize-width-{}", std::process::id());
    let _session = TmuxSession {
        name: session_name.clone(),
    };

    let prompt = "Send me a large paragraph of text for testing.";
    let start_output = checked_output(
        Command::new("tmux")
            .arg("new-session")
            .arg("-d")
            .arg("-P")
            .arg("-F")
            .arg("#{pane_id}")
            .arg("-x")
            .arg("120")
            .arg("-y")
            .arg("40")
            .arg("-s")
            .arg(&session_name)
            .arg("--")
            .arg("env")
            .arg(format!("CODEX_HOME={}", codex_home.path().display()))
            .arg("OPENAI_API_KEY=dummy")
            .arg(codex)
            .arg("-c")
            .arg("analytics.enabled=false")
            .arg("-c")
            .arg(&openai_base_url_config)
            .arg("--no-alt-screen")
            .arg("-C")
            .arg(&repo_root)
            .arg(prompt),
    )?;
    let codex_pane = stdout_text(&start_output).trim().to_string();
    anyhow::ensure!(!codex_pane.is_empty(), "tmux did not report a pane id");

    wait_for_capture_contains(
        &codex_pane,
        "resize reflow sentinel",
        Duration::from_secs(/*secs*/ 15),
    )?;
    wait_for_capture_contains(
        &codex_pane,
        "gpt-5.4 default",
        Duration::from_secs(/*secs*/ 15),
    )?;
    let draft = "Notice where we are here in terms of y location.";
    check(
        Command::new("tmux")
            .arg("send-keys")
            .arg("-t")
            .arg(&codex_pane)
            .arg("-l")
            .arg(draft),
    )?;
    let baseline_capture =
        wait_for_capture_contains(&codex_pane, draft, Duration::from_secs(/*secs*/ 15))?;
    let baseline_row = last_composer_row(&baseline_capture).context("composer row before split")?;
    let baseline_history_row = first_row_containing(&baseline_capture, "resize reflow sentinel")
        .context("history row before split")?;

    let split_output = checked_output(
        Command::new("tmux")
            .arg("split-window")
            .arg("-d")
            .arg("-P")
            .arg("-F")
            .arg("#{pane_id}")
            .arg("-h")
            .arg("-l")
            .arg("40")
            .arg("-t")
            .arg(&codex_pane)
            .arg("sleep")
            .arg("30"),
    )?;
    let split_pane = stdout_text(&split_output).trim().to_string();

    sleep(Duration::from_millis(/*millis*/ 750));
    check(
        Command::new("tmux")
            .arg("kill-pane")
            .arg("-t")
            .arg(&split_pane),
    )?;

    sleep(Duration::from_millis(/*millis*/ 1_000));
    let restored_capture = capture_pane(&codex_pane)?;
    let restored_row =
        last_composer_row(&restored_capture).context("composer row after width restore")?;
    let restored_history_row = first_row_containing(&restored_capture, "resize reflow sentinel")
        .context("history row after width restore")?;
    anyhow::ensure!(
        restored_row == baseline_row,
        "composer row drifted after width restore: baseline={baseline_row}, \
         restored={restored_row}\n\
         baseline:\n{baseline_capture}\n\
         restored:\n{restored_capture}"
    );
    anyhow::ensure!(
        restored_history_row == baseline_history_row,
        "history row drifted after width restore: baseline={baseline_history_row}, \
         restored={restored_history_row}\n\
         baseline:\n{baseline_capture}\n\
         restored:\n{restored_capture}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires tmux and a locally built codex binary; run with --ignored for manual resize smoke"]
// 已滚动的多行草稿在窄化和恢复后仍应显示连续完整行，且不得混入陈旧终端字符。
async fn tmux_scrolled_composer_resize_preserves_visible_draft_text() -> Result<()> {
    if cfg!(windows) {
        return Ok(());
    }
    skip_if_no_network!(Ok(()));
    if Command::new("tmux").arg("-V").output().is_err() {
        eprintln!("skipping resize smoke because tmux is unavailable");
        return Ok(());
    }

    let repo_root = codex_utils_cargo_bin::repo_root()?;
    let codex = codex_binary(&repo_root)?;
    let codex_home = tempdir()?;
    let server = MockServer::start().await;
    let openai_base_url_config = format!("openai_base_url=\"{}/v1\"", server.uri());
    write_config(codex_home.path(), &repo_root)?;
    write_auth(codex_home.path())?;

    let session_name = format!("codex-resize-scrolled-composer-{}", std::process::id());
    let _session = TmuxSession {
        name: session_name.clone(),
    };

    let start_output = checked_output(
        Command::new("tmux")
            .arg("new-session")
            .arg("-d")
            .arg("-P")
            .arg("-F")
            .arg("#{pane_id}")
            .arg("-x")
            .arg("44")
            .arg("-y")
            .arg("14")
            .arg("-s")
            .arg(&session_name)
            .arg("--")
            .arg("env")
            .arg(format!("CODEX_HOME={}", codex_home.path().display()))
            .arg("OPENAI_API_KEY=dummy")
            .arg(codex)
            .arg("--model")
            .arg("gpt-5.6-terra")
            .arg("-c")
            .arg("analytics.enabled=false")
            .arg("-c")
            .arg(&openai_base_url_config)
            .arg("--no-alt-screen")
            .arg("-C")
            .arg(&repo_root),
    )?;
    let codex_pane = stdout_text(&start_output).trim().to_string();
    anyhow::ensure!(!codex_pane.is_empty(), "tmux did not report a pane id");
    wait_for_capture_contains(
        &codex_pane,
        "gpt-5.6-terra",
        Duration::from_secs(/*secs*/ 15),
    )?;

    for index in 1..=9 {
        let line = format!("probe-{index:02} clean words");
        check(
            Command::new("tmux")
                .arg("send-keys")
                .arg("-t")
                .arg(&codex_pane)
                .arg("-l")
                .arg(line),
        )?;
        check(
            Command::new("tmux")
                .arg("send-keys")
                .arg("-t")
                .arg(&codex_pane)
                .arg("C-j"),
        )?;
    }

    let final_line = "q".repeat(/*n*/ 41);
    check(
        Command::new("tmux")
            .arg("send-keys")
            .arg("-t")
            .arg(&codex_pane)
            .arg("-l")
            .arg(&final_line),
    )?;
    let baseline =
        wait_for_capture_contains(&codex_pane, &final_line, Duration::from_secs(/*secs*/ 15))?;

    for (phase, width, height, minimum_visible_rows) in
        [("narrowed", "28", "9", 2), ("restored", "44", "14", 6)]
    {
        check(
            Command::new("tmux")
                .arg("resize-window")
                .arg("-t")
                .arg(&session_name)
                .arg("-x")
                .arg(width)
                .arg("-y")
                .arg(height),
        )?;
        sleep(Duration::from_millis(/*millis*/ 350));
        let capture = capture_pane(&codex_pane)?;
        let visible_rows = capture
            .lines()
            .filter_map(|line| line.find("probe-").map(|start| (line, start)))
            .collect::<Vec<_>>();

        anyhow::ensure!(
            visible_rows.len() >= minimum_visible_rows,
            "{phase} composer exposes too few draft rows: expected at least \
             {minimum_visible_rows}, found {}\nbaseline:\n{baseline}\ncapture:\n{capture}",
            visible_rows.len()
        );

        for (line, start) in visible_rows {
            let draft = &line[start..];
            let row_number = draft
                .get(6..8)
                .with_context(|| format!("malformed draft row after {phase}: {draft}"))?;
            let expected = format!("probe-{row_number} clean words");
            let prefix = line[..start].trim();
            anyhow::ensure!(
                draft.starts_with(&expected) && (prefix.is_empty() || prefix == "›"),
                "{phase} resize left stale terminal content in the composer: \
                 expected {expected:?}, found {line:?}\nbaseline:\n{baseline}\ncapture:\n{capture}"
            );
        }
    }

    Ok(())
}

async fn run_repeated_resize_smoke() -> Result<()> {
    // 共享长会话 fixture 执行多轮缩放，并在每轮恢复后比较相同行锚点。
    let repo_root = codex_utils_cargo_bin::repo_root()?;
    let codex = codex_binary(&repo_root)?;
    let codex_home = tempdir()?;
    let server = MockServer::start().await;
    let _response_mock = responses::mount_sse_once(&server, resize_reflow_sse()).await;
    let openai_base_url_config = format!("openai_base_url=\"{}/v1\"", server.uri());
    write_config(codex_home.path(), &repo_root)?;
    write_auth(codex_home.path())?;

    let session_name = format!("codex-resize-repeat-{}", std::process::id());
    let _session = TmuxSession {
        name: session_name.clone(),
    };

    let prompt = "Send me a large paragraph of text for testing.";
    let start_output = checked_output(
        Command::new("tmux")
            .arg("new-session")
            .arg("-d")
            .arg("-P")
            .arg("-F")
            .arg("#{pane_id}")
            .arg("-x")
            .arg("120")
            .arg("-y")
            .arg("40")
            .arg("-s")
            .arg(&session_name)
            .arg("--")
            .arg("env")
            .arg(format!("CODEX_HOME={}", codex_home.path().display()))
            .arg("OPENAI_API_KEY=dummy")
            .arg(codex)
            .arg("-c")
            .arg("analytics.enabled=false")
            .arg("-c")
            .arg(&openai_base_url_config)
            .arg("--no-alt-screen")
            .arg("-C")
            .arg(&repo_root)
            .arg(prompt),
    )?;
    let codex_pane = stdout_text(&start_output).trim().to_string();
    anyhow::ensure!(!codex_pane.is_empty(), "tmux did not report a pane id");

    wait_for_capture_contains(
        &codex_pane,
        "resize reflow sentinel",
        Duration::from_secs(/*secs*/ 15),
    )?;
    wait_for_capture_contains(
        &codex_pane,
        "gpt-5.4 default",
        Duration::from_secs(/*secs*/ 15),
    )?;
    let draft = "Notice where we are here in terms of y location.";
    check(
        Command::new("tmux")
            .arg("send-keys")
            .arg("-t")
            .arg(&codex_pane)
            .arg("-l")
            .arg(draft),
    )?;
    let baseline_capture =
        wait_for_capture_contains(&codex_pane, draft, Duration::from_secs(/*secs*/ 15))?;
    let baseline_row = last_composer_row(&baseline_capture).context("composer row before split")?;
    let baseline_history_row = first_row_containing(&baseline_capture, "resize reflow sentinel")
        .context("history row before split")?;

    for cycle in 1..=3 {
        let split_output = checked_output(
            Command::new("tmux")
                .arg("split-window")
                .arg("-d")
                .arg("-P")
                .arg("-F")
                .arg("#{pane_id}")
                .arg("-v")
                .arg("-l")
                .arg("12")
                .arg("-t")
                .arg(&codex_pane)
                .arg("sleep")
                .arg("30"),
        )?;
        let split_pane = stdout_text(&split_output).trim().to_string();

        sleep(Duration::from_millis(/*millis*/ 250));
        check(
            Command::new("tmux")
                .arg("kill-pane")
                .arg("-t")
                .arg(&split_pane),
        )?;

        sleep(Duration::from_millis(/*millis*/ 500));
        let restored_capture = capture_pane(&codex_pane)?;
        let restored_row = last_composer_row(&restored_capture)
            .with_context(|| format!("composer row after resize cycle {cycle}"))?;
        let restored_history_row =
            first_row_containing(&restored_capture, "resize reflow sentinel")
                .with_context(|| format!("history row after resize cycle {cycle}"))?;
        anyhow::ensure!(
            restored_row == baseline_row,
            "composer row drifted after resize cycle {cycle}: baseline={baseline_row}, \
             restored={restored_row}\n\
             baseline:\n{baseline_capture}\n\
             restored:\n{restored_capture}"
        );
        anyhow::ensure!(
            restored_history_row == baseline_history_row,
            "history row drifted after resize cycle {cycle}: baseline={baseline_history_row}, \
             restored={restored_history_row}\n\
             baseline:\n{baseline_capture}\n\
             restored:\n{restored_capture}"
        );
    }

    Ok(())
}

struct TmuxSession {
    name: String,
}

impl Drop for TmuxSession {
    // 测试结束时始终回收命名 tmux 会话，失败路径也不会留下后台进程。
    fn drop(&mut self) {
        let _ = Command::new("tmux")
            .arg("kill-session")
            .arg("-t")
            .arg(&self.name)
            .output();
    }
}

fn codex_binary(repo_root: &Path) -> Result<PathBuf> {
    // 优先使用测试运行文件，缺失时仅接受仓库内明确的调试构建产物。
    if let Ok(path) = codex_utils_cargo_bin::cargo_bin("codex") {
        return Ok(path);
    }

    let fallback = repo_root.join("codex-rs/target/debug/codex");
    anyhow::ensure!(
        fallback.is_file(),
        "codex binary is unavailable; run `cargo build -p codex-cli` first"
    );
    Ok(fallback)
}

fn write_config(codex_home: &Path, repo_root: &Path) -> Result<()> {
    // 写入最小可信项目配置，隔离宿主用户的模型和分析设置。
    let repo_root_display = repo_root.display();
    let config = format!(
        r#"model = "gpt-5.4"
model_provider = "openai"
suppress_unstable_features_warning = true

[projects."{repo_root_display}"]
trust_level = "trusted"
"#
    );
    std::fs::write(codex_home.join("config.toml"), config)?;
    Ok(())
}

fn write_auth(codex_home: &Path) -> Result<()> {
    // 伪认证只供本地模拟服务启动客户端，不携带真实凭据。
    std::fs::write(
        codex_home.join("auth.json"),
        r#"{"OPENAI_API_KEY":"dummy","tokens":null,"last_refresh":null}"#,
    )?;
    Ok(())
}

fn resize_reflow_sse() -> String {
    // 固定长段落确保宽窄窗格都有足够历史行可观察重排。
    let text = "resize reflow sentinel says hi. This paragraph is intentionally long enough to exercise terminal wrapping, scrollback redraw, and pane resize behavior without requiring a live model response. It includes enough ordinary prose to wrap across several rows in a narrow tmux pane, then keep going so repeated split and restore cycles have visible history above the composer. If a resize path accidentally inserts blank rows or anchors the viewport lower on each pass, the composer row will drift after the pane returns to its original height.";
    responses::sse(vec![
        responses::ev_response_created("resp-resize-smoke"),
        responses::ev_assistant_message("msg-resize-smoke", text),
        responses::ev_completed("resp-resize-smoke"),
    ])
}

fn wait_for_capture_contains(pane: &str, needle: &str, timeout: Duration) -> Result<String> {
    // 有界轮询窗格捕获，超时时保留最后画面用于诊断布局状态。
    let deadline = Instant::now() + timeout;
    let mut last_capture = String::new();
    while Instant::now() < deadline {
        last_capture = capture_pane(pane)?;
        if last_capture.contains(needle) {
            return Ok(last_capture);
        }
        sleep(Duration::from_millis(/*millis*/ 100));
    }

    anyhow::bail!("timed out waiting for {needle:?}; last capture:\n{last_capture}");
}

fn capture_pane(pane: &str) -> Result<String> {
    // 读取 tmux 当前可见文本，不解析样式控制序列。
    let output = output(
        Command::new("tmux")
            .arg("capture-pane")
            .arg("-p")
            .arg("-t")
            .arg(pane),
    )?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn last_composer_row(capture: &str) -> Option<usize> {
    capture
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            if line.trim_start().starts_with('\u{203a}') {
                Some(index)
            } else {
                None
            }
        })
        .last()
}

fn first_row_containing(capture: &str, needle: &str) -> Option<usize> {
    capture
        .lines()
        .enumerate()
        .find_map(|(index, line)| line.contains(needle).then_some(index))
}

fn check(command: &mut Command) -> Result<()> {
    checked_output(command)?;
    Ok(())
}

fn checked_output(command: &mut Command) -> Result<Output> {
    // 外部命令失败时同时报告状态和双路输出，避免把环境问题误判为布局回归。
    let output = output(command)?;
    anyhow::ensure!(
        output.status.success(),
        "command failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output)
}

fn output(command: &mut Command) -> Result<Output> {
    command
        .output()
        .with_context(|| format!("failed to run {command:?}"))
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}
